use std::collections::HashMap;
use std::sync::Arc;

use crate::app::AppMessage;
use crate::commands::Command;
use crate::config::{GlobalAction, KeyResolver};
use crate::context::{CloudContext, GcpContext};
use crate::provider::gcp::secret_manager::client::SecretManagerClient;
use crate::provider::gcp::secret_manager::payload::{PayloadMsg, SecretPayload};
use crate::provider::gcp::secret_manager::secrets::{Secret, SecretsMsg};
use crate::provider::gcp::secret_manager::versions::{SecretVersion, VersionsMsg};
use crate::provider::gcp::secret_manager::{payload, secrets, versions};
use crate::provider::Provider;
use crate::registry::ServiceProvider;
use crate::service::{Service, ServiceMsg};
use crate::ui::{Component, EventResult, EventResultExt, Keybinding, Modal, Screen, Spinner};
use crate::Theme;
use async_trait::async_trait;
use color_eyre::Result;
use crossterm::event::KeyEvent;
use ratatui::layout::Rect;
use ratatui::Frame;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

// === Messages ===

#[derive(Debug, Clone)]
pub enum SecretManagerMsg {
    Initialize,
    ClientInitialized(SecretManagerClient),

    NavigateBack,
    DialogCancelled,

    Secret(SecretsMsg),
    Version(VersionsMsg),
    Payload(PayloadMsg),
}

// === Provider ===

pub struct SecretManagerProvider;

impl ServiceProvider for SecretManagerProvider {
    fn provider(&self) -> Provider {
        Provider::Gcp
    }

    fn service_key(&self) -> &'static str {
        "secret-manager"
    }

    fn display_name(&self) -> &'static str {
        "Secret Manager"
    }

    fn description(&self) -> &'static str {
        "Store and manage secrets, API keys, and certificates"
    }

    fn icon(&self) -> Option<&'static str> {
        None
    }

    fn create_service(&self, ctx: &CloudContext, resolver: Arc<KeyResolver>) -> Box<dyn Service> {
        let CloudContext::Gcp(gcp_ctx) = ctx;
        Box::new(SecretManager::new(gcp_ctx.clone(), resolver))
    }
}

// === Service ===

pub struct SecretManager {
    context: GcpContext,
    spinner: Spinner,
    client: Option<SecretManagerClient>,
    screen_stack: Vec<Box<dyn Screen<Output = SecretManagerMsg>>>,
    loading: Option<&'static str>,
    modal: Option<Box<dyn Modal<Output = SecretManagerMsg>>>,
    msg_tx: UnboundedSender<SecretManagerMsg>,
    msg_rx: UnboundedReceiver<SecretManagerMsg>,
    cached_secrets: Option<Vec<Secret>>,
    /// Key: secret name
    cached_versions: HashMap<String, Vec<SecretVersion>>,
    /// Key: "`secret_name/version_id`"
    cached_payloads: HashMap<String, SecretPayload>,
    resolver: Arc<KeyResolver>,
}

impl SecretManager {
    pub fn new(ctx: GcpContext, resolver: Arc<KeyResolver>) -> Self {
        let (msg_tx, msg_rx) = mpsc::unbounded_channel();
        Self {
            context: ctx,
            spinner: Spinner::new(),
            client: None,
            screen_stack: Vec::new(),
            loading: Some("Initializing..."),
            modal: None,
            msg_tx,
            msg_rx,
            cached_secrets: None,
            cached_versions: HashMap::new(),
            cached_payloads: HashMap::new(),
            resolver,
        }
    }

    pub(super) fn get_resolver(&self) -> Arc<KeyResolver> {
        self.resolver.clone()
    }

    // === Public helpers for feature slices ===

    pub(super) fn get_client(&self) -> Result<SecretManagerClient> {
        self.client
            .clone()
            .ok_or_else(|| color_eyre::eyre::eyre!("Secret Manager client not initialized"))
    }

    pub(super) fn get_msg_sender(&self) -> UnboundedSender<SecretManagerMsg> {
        self.msg_tx.clone()
    }

    pub(super) fn queue(&self, msg: SecretManagerMsg) {
        let _ = self.msg_tx.send(msg);
    }

    // === Screen stack management ===

    pub(super) fn push_view<T: Screen<Output = SecretManagerMsg> + 'static>(&mut self, screen: T) {
        self.hide_loading_spinner();
        self.screen_stack.push(Box::new(screen));
    }

    pub(super) fn pop_view(&mut self) -> bool {
        if self.screen_stack.len() > 1 {
            self.screen_stack.pop();
            true
        } else {
            false
        }
    }

    pub(super) fn pop_to_root(&mut self) {
        while self.screen_stack.len() > 1 {
            self.screen_stack.pop();
        }
        self.screen_stack.clear();
    }

    // === Modal management ===

    pub(super) fn display_overlay<T: Modal<Output = SecretManagerMsg> + 'static>(
        &mut self,
        modal: T,
    ) {
        self.modal = Some(Box::new(modal));
    }

    pub(super) fn close_overlay(&mut self) {
        self.modal = None;
    }

    // === Loading spinner ===

    pub(super) const fn display_loading_spinner(&mut self, label: &'static str) {
        self.loading = Some(label);
    }

    pub(super) const fn hide_loading_spinner(&mut self) {
        self.loading = None;
    }

    // === Caching: Secrets ===

    pub(super) fn get_cached_secrets(&self) -> Option<Vec<Secret>> {
        self.cached_secrets.clone()
    }

    pub(super) fn cache_secrets(&mut self, secrets: &[Secret]) {
        self.cached_secrets = Some(secrets.to_vec());
    }

    pub(super) fn invalidate_secrets_cache(&mut self) {
        self.cached_secrets = None;
    }

    // === Caching: Versions ===

    pub(super) fn get_cached_versions(&self, secret: &Secret) -> Option<Vec<SecretVersion>> {
        self.cached_versions.get(&secret.name).cloned()
    }

    pub(super) fn cache_versions(&mut self, secret: &Secret, versions: Vec<SecretVersion>) {
        self.cached_versions.insert(secret.name.clone(), versions);
    }

    pub(super) fn invalidate_versions_cache(&mut self, secret: &Secret) {
        self.cached_versions.remove(&secret.name);
    }

    // === Caching: Payloads ===

    pub(super) fn get_cached_payload(
        &self,
        secret: &Secret,
        version: Option<&SecretVersion>,
    ) -> Option<SecretPayload> {
        let cache_key = Self::payload_cache_key(secret, version);
        self.cached_payloads.get(&cache_key).cloned()
    }

    pub(super) fn cache_payload(
        &mut self,
        secret: &Secret,
        version: Option<&SecretVersion>,
        payload: SecretPayload,
    ) {
        let cache_key = Self::payload_cache_key(secret, version);
        self.cached_payloads.insert(cache_key, payload);
    }

    fn payload_cache_key(secret: &Secret, version: Option<&SecretVersion>) -> String {
        let version_id = version.map_or("latest", |v| v.version_id.as_str());
        format!("{}/{}", secret.name, version_id)
    }

    // === Message processing ===

    fn current_screen(&self) -> Option<&dyn Screen<Output = SecretManagerMsg>> {
        self.screen_stack.last().map(|b| &**b)
    }

    fn current_screen_mut(&mut self) -> Option<&mut Box<dyn Screen<Output = SecretManagerMsg>>> {
        self.screen_stack.last_mut()
    }

    fn process_message(&mut self, msg: SecretManagerMsg) -> Result<ServiceMsg> {
        match msg {
            // === Lifecycle ===
            SecretManagerMsg::Initialize => {
                self.loading = Some("Initializing Secret Manager...");
                Ok(InitClientCmd {
                    context: self.context.clone(),
                    tx: self.msg_tx.clone(),
                }
                .into())
            }

            SecretManagerMsg::ClientInitialized(client) => {
                self.client = Some(client);
                self.queue(SecretsMsg::Load.into());
                Ok(ServiceMsg::Idle)
            }

            // === Navigation ===
            SecretManagerMsg::NavigateBack => {
                if self.pop_view() {
                    Ok(ServiceMsg::Idle)
                } else {
                    Ok(ServiceMsg::Close)
                }
            }

            SecretManagerMsg::DialogCancelled => {
                self.close_overlay();
                Ok(ServiceMsg::Idle)
            }

            // === Feature Dispatching ===
            SecretManagerMsg::Secret(msg) => secrets::update(self, msg),
            SecretManagerMsg::Version(msg) => versions::update(self, msg),
            SecretManagerMsg::Payload(msg) => payload::update(self, msg),
        }
    }
}

impl Service for SecretManager {
    fn init(&mut self) {
        self.queue(SecretManagerMsg::Initialize);
    }

    fn handle_tick(&mut self) {
        if self.loading.is_some() {
            self.spinner.handle_tick();
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> EventResult<()> {
        if self.loading.is_some() {
            return EventResult::Ignored;
        }

        // Handle modal first if present (captures all input)
        if let Some(modal) = &mut self.modal {
            let (consumed, msg) = modal.handle_key(key).process();
            if let Some(msg) = msg {
                self.queue(msg);
            }
            if consumed {
                return EventResult::Consumed;
            }
        }

        // Handle current screen
        if let Some(screen) = self.current_screen_mut() {
            let (consumed, msg) = screen.handle_key(key).process();
            if let Some(msg) = msg {
                self.queue(msg);
            }
            if consumed {
                return EventResult::Consumed;
            }
        }

        // Global navigation
        if self.resolver.matches_global(&key, GlobalAction::Back) {
            self.queue(SecretManagerMsg::NavigateBack);
            return EventResult::Consumed;
        }

        EventResult::Ignored
    }

    fn update(&mut self) -> Result<ServiceMsg> {
        let mut commands: Vec<Box<dyn Command>> = Vec::new();

        while let Ok(msg) = self.msg_rx.try_recv() {
            match self.process_message(msg)? {
                ServiceMsg::Idle => {}
                ServiceMsg::Run(cmds) => commands.extend(cmds),
                ServiceMsg::Close => return Ok(ServiceMsg::Close),
            }
        }

        if commands.is_empty() {
            Ok(ServiceMsg::Idle)
        } else {
            Ok(ServiceMsg::Run(commands))
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        if let Some(label) = self.loading {
            self.spinner.set_label(label);
            self.spinner.render(frame, area, theme);
        } else if let Some(screen) = self.current_screen_mut() {
            screen.render(frame, area, theme);
        }

        // Render modal on top if present
        if let Some(modal) = &mut self.modal {
            modal.render(frame, area, theme);
        }
    }

    fn breadcrumbs(&self) -> Vec<String> {
        let mut bc = vec!["Secret Manager".to_string()];
        for screen in &self.screen_stack {
            bc.extend(screen.breadcrumbs());
        }
        bc
    }

    fn keybindings(&self) -> Vec<Keybinding> {
        self.current_screen()
            .map(Screen::keybindings)
            .unwrap_or_default()
    }
}

// === Commands ===

struct InitClientCmd {
    context: GcpContext,
    tx: UnboundedSender<SecretManagerMsg>,
}

#[async_trait]
impl Command for InitClientCmd {
    fn name(&self) -> String {
        format!("Connecting to {}", self.context.display_name)
    }

    async fn execute(self: Box<Self>, _action_tx: UnboundedSender<AppMessage>) -> Result<()> {
        let client = SecretManagerClient::new(&self.context).await?;
        self.tx.send(SecretManagerMsg::ClientInitialized(client))?;
        Ok(())
    }
}
