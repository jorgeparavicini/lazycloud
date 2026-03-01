use std::collections::HashMap;

use async_trait::async_trait;
use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent};
use google_cloud_api_serviceusage_v1::client::ServiceUsage;
use google_cloud_lro::Poller;
use ratatui::Frame;
use ratatui::layout::Rect;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tracing::{error, info};
use crate::Theme;
use crate::app::AppMessage;
use crate::commands::Command;
use crate::context::{CloudContext, GcpContext};
use crate::provider::Provider;
use crate::provider::gcp::secret_manager::client::{ClientError, SecretManagerClient};
use crate::provider::gcp::secret_manager::payload::{PayloadMsg, SecretPayload};
use crate::provider::gcp::secret_manager::secrets::{Secret, SecretsMsg};
use crate::provider::gcp::secret_manager::versions::{SecretVersion, VersionsMsg};
use crate::provider::gcp::secret_manager::{payload, secrets, versions};
use crate::registry::ServiceProvider;
use crate::service::{Service, ServiceMsg};
use crate::ui::{
    Component, ConfirmDialog, ConfirmEvent, EventResult, EventResultExt, Keybinding, Modal, Screen,
    Spinner,
};

// === Messages ===

#[derive(Debug, Clone)]
pub enum SecretManagerMsg {
    Initialize,
    ClientInitialized(SecretManagerClient),

    ApiDisabled,
    EnableApi,
    ApiEnabled,

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

    fn create_service(&self, ctx: &CloudContext) -> Box<dyn Service> {
        let CloudContext::Gcp(gcp_ctx) = ctx;
        Box::new(SecretManager::new(gcp_ctx.clone()))
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
    editing_secret: Option<Secret>,
}

impl SecretManager {
    pub fn new(ctx: GcpContext) -> Self {
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
            editing_secret: None,
        }
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

    pub(super) fn invalidate_payload_cache(&mut self, secret: &Secret) {
        let prefix = format!("{}/", secret.name);
        self.cached_payloads.retain(|k, _| !k.starts_with(&prefix));
    }

    // === Editor support ===

    pub(super) fn set_editing_secret(&mut self, secret: Secret) {
        self.editing_secret = Some(secret);
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

            // === API Enable Flow ===
            SecretManagerMsg::ApiDisabled => {
                self.hide_loading_spinner();
                self.display_overlay(EnableApiDialog::new());
                Ok(ServiceMsg::Idle)
            }

            SecretManagerMsg::EnableApi => {
                self.close_overlay();
                self.display_loading_spinner("Enabling Secret Manager API...");
                Ok(EnableApiCmd {
                    context: self.context.clone(),
                    tx: self.msg_tx.clone(),
                }
                .into())
            }

            SecretManagerMsg::ApiEnabled => {
                self.queue(SecretManagerMsg::Initialize);
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
                // If there's nothing to show (e.g. API dialog before secrets loaded), close
                if self.screen_stack.is_empty() {
                    return Ok(ServiceMsg::Close);
                }
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
        if key.code == KeyCode::Esc {
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
                msg @ ServiceMsg::EditExternal { .. } => return Ok(msg),
            }
        }

        if commands.is_empty() {
            Ok(ServiceMsg::Idle)
        } else {
            Ok(ServiceMsg::Run(commands))
        }
    }

    fn handle_editor_result(&mut self, new_content: Option<String>) {
        if let Some(secret) = self.editing_secret.take()
            && let Some(new_data) = new_content
        {
            self.queue(
                PayloadMsg::SaveEdit {
                    secret,
                    new_data,
                }
                .into(),
            );
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

// === Dialogs ===

struct EnableApiDialog {
    dialog: ConfirmDialog,
}

impl EnableApiDialog {
    fn new() -> Self {
        let dialog = ConfirmDialog::new(
            "The Secret Manager API is not enabled for this project. Would you like to enable it?",
        )
        .with_title("API Not Enabled")
        .with_confirm_text("Enable");
        Self { dialog }
    }
}

impl Modal for EnableApiDialog {
    type Output = SecretManagerMsg;

    fn handle_key(&mut self, key: KeyEvent) -> Result<EventResult<Self::Output>> {
        Ok(match self.dialog.handle_key(key)? {
            EventResult::Event(ConfirmEvent::Confirmed) => SecretManagerMsg::EnableApi.into(),
            EventResult::Event(ConfirmEvent::Cancelled) => {
                SecretManagerMsg::DialogCancelled.into()
            }
            _ => EventResult::Consumed,
        })
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        self.dialog.render(frame, area, theme);
    }
}

// === Commands ===

struct EnableApiCmd {
    context: GcpContext,
    tx: UnboundedSender<SecretManagerMsg>,
}

#[async_trait]
impl Command for EnableApiCmd {
    fn name(&self) -> String {
        "Enabling Secret Manager API".to_string()
    }

    async fn execute(self: Box<Self>, _action_tx: UnboundedSender<AppMessage>) -> Result<()> {
        let credentials = self.context.create_credentials()?;
        let client = ServiceUsage::builder()
            .with_credentials(credentials)
            .build()
            .await?;

        let service_name = format!(
            "projects/{}/services/secretmanager.googleapis.com",
            self.context.project_id
        );

        client
            .enable_service()
            .set_name(service_name)
            .poller()
            .until_done()
            .await?;

        self.tx.send(SecretManagerMsg::ApiEnabled)?;
        Ok(())
    }
}

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
        match SecretManagerClient::new(&self.context).await {
            Ok(client) => {
                info!("Successfully initialized Secret Manager client");
                self.tx.send(SecretManagerMsg::ClientInitialized(client))?;
                Ok(())
            }
            Err(ClientError::ApiDisabled) => {
                self.tx.send(SecretManagerMsg::ApiDisabled)?;
                Ok(())
            }
            Err(e) => {
                error!("Failed to initialize Secret Manager client: {e}");
                Err(e.into())
            }
        }
    }
}
