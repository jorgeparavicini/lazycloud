use crate::Theme;
use crate::app::AppMessage;
use crate::cache::Cache;
use crate::commands::Command;
use crate::context::{CloudContext, GcpContext};
use crate::event_queue::EventQueue;
use crate::provider::Provider;
use crate::provider::gcp::secret_manager::client::{ClientError, SecretManagerClient};
use crate::provider::gcp::secret_manager::payload::PayloadMsg;
use crate::provider::gcp::secret_manager::secrets::{Secret, SecretsMsg};
use crate::provider::gcp::secret_manager::versions::VersionsMsg;
use crate::provider::gcp::secret_manager::{payload, secrets, versions};
use crate::registry::ServiceProvider;
use crate::service::{Service, ServiceMsg};
use crate::ui::{Component, ConfirmDialog, ConfirmEvent, EventResult, Keybinding, Modal};
use crate::view_stack::ViewStack;
use async_trait::async_trait;
use color_eyre::Result;
use crossterm::event::KeyEvent;
use google_cloud_api_serviceusage_v1::client::ServiceUsage;
use google_cloud_lro::Poller;
use ratatui::Frame;
use ratatui::layout::Rect;
use tokio::sync::mpsc::UnboundedSender;
use tracing::{error, info};

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
    pub(super) views: ViewStack<SecretManagerMsg>,
    pub(super) cache: Cache,
    events: EventQueue<SecretManagerMsg>,
    context: GcpContext,
    client: Option<SecretManagerClient>,
    editing_secret: Option<Secret>,
}

impl SecretManager {
    pub fn new(ctx: GcpContext) -> Self {
        let mut views = ViewStack::new();
        views.set_loading("Initializing...");
        Self {
            events: EventQueue::new(),
            views,
            cache: Cache::new(),
            context: ctx,
            client: None,
            editing_secret: None,
        }
    }

    // === Public helpers for feature slices ===

    pub(super) fn queue(&self, msg: SecretManagerMsg) {
        self.events.send(msg);
    }

    pub(super) fn clone_sender(&self) -> UnboundedSender<SecretManagerMsg> {
        self.events.clone_sender()
    }

    pub(super) fn get_client(&self) -> Result<SecretManagerClient> {
        self.client
            .clone()
            .ok_or_else(|| color_eyre::eyre::eyre!("Secret Manager client not initialized"))
    }

    pub(super) fn set_editing_secret(&mut self, secret: Secret) {
        self.editing_secret = Some(secret);
    }

    // === Message processing ===

    fn process_message(&mut self, msg: SecretManagerMsg) -> Result<ServiceMsg> {
        match msg {
            // === Lifecycle ===
            SecretManagerMsg::Initialize => {
                self.views.set_loading("Initializing Secret Manager...");
                Ok(InitClientCmd {
                    context: self.context.clone(),
                    tx: self.events.clone_sender(),
                }
                .into())
            }

            SecretManagerMsg::ClientInitialized(client) => {
                self.client = Some(client);
                self.events.send(SecretsMsg::Load.into());
                Ok(ServiceMsg::Idle)
            }

            // === API Enable Flow ===
            SecretManagerMsg::ApiDisabled => {
                self.views.clear_loading();
                self.views.show_modal(EnableApiDialog::new());
                Ok(ServiceMsg::Idle)
            }

            SecretManagerMsg::EnableApi => {
                self.views.close_modal();
                self.views.set_loading("Enabling Secret Manager API...");
                Ok(EnableApiCmd {
                    context: self.context.clone(),
                    tx: self.events.clone_sender(),
                }
                .into())
            }

            SecretManagerMsg::ApiEnabled => {
                self.events.send(SecretManagerMsg::Initialize);
                Ok(ServiceMsg::Idle)
            }

            // === Navigation ===
            SecretManagerMsg::NavigateBack => {
                if self.views.pop() {
                    Ok(ServiceMsg::Idle)
                } else {
                    Ok(ServiceMsg::Close)
                }
            }

            SecretManagerMsg::DialogCancelled => {
                self.views.close_modal();
                // If there's nothing to show (e.g., API dialog before secrets loaded), close
                if !self.views.has_screens() {
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
        self.events.send(SecretManagerMsg::Initialize);
    }

    fn handle_tick(&mut self) {
        self.views.handle_tick();
    }

    fn handle_key(&mut self, key: KeyEvent) -> EventResult<()> {
        self.views
            .handle_key(key, &self.events, SecretManagerMsg::NavigateBack)
    }

    fn update(&mut self) -> Result<ServiceMsg> {
        let mut commands: Vec<Box<dyn crate::commands::Command>> = Vec::new();

        loop {
            let messages = self.events.drain();
            if messages.is_empty() {
                break;
            }
            match EventQueue::process_events(messages, |msg| self.process_message(msg))? {
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
            self.events
                .send(PayloadMsg::SaveEdit { secret, new_data }.into());
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        self.views.render(frame, area, theme);
    }

    fn breadcrumbs(&self) -> Vec<String> {
        self.views.breadcrumbs("Secret Manager")
    }

    fn keybindings(&self) -> Vec<Keybinding> {
        self.views.keybindings()
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
            EventResult::Event(ConfirmEvent::Cancelled) => SecretManagerMsg::DialogCancelled.into(),
            _ => EventResult::Consumed,
        })
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        self.dialog.render(frame, area, theme);
    }
}

// === Commands ===

// TODO: Move to global state
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
