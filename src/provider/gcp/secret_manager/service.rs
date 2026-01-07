use crate::core::command::{Command, CopyToClipboardCmd};
use crate::core::event::Event;
use crate::core::service::{Service, UpdateResult};
use crate::model::GcpContext;
use crate::provider::gcp::secret_manager::client::SecretManagerClient;
use crate::provider::gcp::secret_manager::command::{
    FetchPayloadCmd, FetchSecretsCmd, FetchVersionsCmd, InitClientCmd,
};
use crate::provider::gcp::secret_manager::message::SecretManagerMsg;
use crate::provider::gcp::secret_manager::model::{Secret, SecretVersion};
use crate::provider::gcp::secret_manager::view::{PayloadView, SecretListView, VersionListView};
use crate::provider::gcp::secret_manager::SecretManagerView;
use crate::widget::Spinner;
use crossterm::event::KeyCode;
use ratatui::layout::Rect;
use ratatui::Frame;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

/// Current view state of the Secret Manager service.
enum State {
    Loading,
    SecretList(SecretListView),
    VersionList(VersionListView),
    Payload(PayloadView),
}

/// Service for managing GCP Secret Manager secrets.
pub struct SecretManager {
    project_id: String,
    spinner: Spinner,
    client: Option<SecretManagerClient>,
    state: State,
    /// Breadcrumb context preserved during loading transitions.
    loading_context: Vec<String>,
    msg_tx: UnboundedSender<SecretManagerMsg>,
    msg_rx: UnboundedReceiver<SecretManagerMsg>,
}

impl SecretManager {
    pub fn new(ctx: &GcpContext) -> Self {
        let (msg_tx, msg_rx) = mpsc::unbounded_channel();
        Self {
            project_id: ctx.project_id.clone(),
            spinner: Spinner::new(),
            client: None,
            state: State::Loading,
            loading_context: vec!["Secrets".to_string()],
            msg_tx,
            msg_rx,
        }
    }

    /// Queue a message to be processed by update().
    fn queue(&self, msg: SecretManagerMsg) {
        let _ = self.msg_tx.send(msg);
    }

    fn enter_loading_state(&mut self, label: &'static str) {
        self.spinner.set_label(label);
        // Capture breadcrumb context from current state (skip "Secret Manager" prefix)
        self.loading_context = self.breadcrumbs().into_iter().skip(1).collect();
        self.state = State::Loading;
    }

    /// Process a single message and return the result.
    fn process_message(&mut self, msg: SecretManagerMsg) -> UpdateResult {
        match msg {
            SecretManagerMsg::Initialize => {
                self.enter_loading_state("Initializing Secret Manager...");
                UpdateResult::Commands(vec![Box::new(InitClientCmd::new(
                    self.project_id.clone(),
                    self.msg_tx.clone(),
                ))])
            }

            SecretManagerMsg::NavigateBack => self.navigate_back(),

            SecretManagerMsg::ReloadData => self.reload_current_view(),

            SecretManagerMsg::SelectSecret(secret) => self.fetch_versions(secret),

            SecretManagerMsg::SelectVersion(secret, version) => {
                self.fetch_payload(secret, version)
            }

            SecretManagerMsg::CopyPayload => self.copy_payload_to_clipboard(),

            SecretManagerMsg::ClientInitialized(client) => {
                self.client = Some(client);
                self.fetch_secrets()
            }

            SecretManagerMsg::SecretsLoaded(secrets) => {
                self.state = State::SecretList(SecretListView::new(secrets));
                UpdateResult::Idle
            }

            SecretManagerMsg::VersionsLoaded { secret, versions } => {
                self.state = State::VersionList(VersionListView::new(secret, versions));
                UpdateResult::Idle
            }

            SecretManagerMsg::PayloadLoaded {
                secret,
                version,
                payload,
            } => {
                self.state = State::Payload(PayloadView::new(secret, version, payload));
                UpdateResult::Idle
            }

            SecretManagerMsg::OperationFailed(err) => UpdateResult::Error(err),
        }
    }

    fn navigate_back(&mut self) -> UpdateResult {
        match &self.state {
            State::Loading | State::SecretList(_) => UpdateResult::Close,
            State::VersionList(_) => self.fetch_secrets(),
            State::Payload(v) => self.fetch_versions(v.secret().clone()),
        }
    }

    fn reload_current_view(&mut self) -> UpdateResult {
        match &self.state {
            State::Loading => UpdateResult::Idle,
            State::SecretList(_) => self.fetch_secrets(),
            State::VersionList(v) => self.fetch_versions(v.secret().clone()),
            State::Payload(v) => self.fetch_payload(v.secret().clone(), v.version().clone()),
        }
    }

    fn fetch_secrets(&mut self) -> UpdateResult {
        self.enter_loading_state("Loading secrets...");
        if let Some(client) = &self.client {
            UpdateResult::Commands(vec![Box::new(FetchSecretsCmd::new(
                client.clone(),
                self.msg_tx.clone(),
            ))])
        } else {
            UpdateResult::Error("Client not initialized".to_string())
        }
    }

    fn fetch_versions(&mut self, secret: Secret) -> UpdateResult {
        self.enter_loading_state("Loading versions...");
        if let Some(client) = &self.client {
            UpdateResult::Commands(vec![Box::new(FetchVersionsCmd::new(
                client.clone(),
                secret,
                self.msg_tx.clone(),
            ))])
        } else {
            UpdateResult::Error("Client not initialized".to_string())
        }
    }

    fn fetch_payload(&mut self, secret: Secret, version: SecretVersion) -> UpdateResult {
        self.enter_loading_state("Loading payload...");
        if let Some(client) = &self.client {
            UpdateResult::Commands(vec![Box::new(FetchPayloadCmd::new(
                client.clone(),
                secret,
                version,
                self.msg_tx.clone(),
            ))])
        } else {
            UpdateResult::Error("Client not initialized".to_string())
        }
    }

    fn copy_payload_to_clipboard(&self) -> UpdateResult {
        if let State::Payload(view) = &self.state {
            UpdateResult::Commands(vec![Box::new(CopyToClipboardCmd::new(
                view.payload().data.clone(),
            ))])
        } else {
            UpdateResult::Idle
        }
    }
}

impl Service for SecretManager {
    fn init(&mut self) {
        self.queue(SecretManagerMsg::Initialize);
    }

    fn handle_tick(&mut self) {
        if matches!(self.state, State::Loading) {
            self.spinner.on_tick();
        }
    }

    fn handle_input(&mut self, event: &Event) -> bool {
        let Event::Key(key) = event else {
            return false;
        };

        // Let the current view handle the key
        let msg = match &mut self.state {
            State::Loading => None,
            State::SecretList(v) => v.handle_key(*key),
            State::VersionList(v) => v.handle_key(*key),
            State::Payload(v) => v.handle_key(*key),
        };

        if let Some(m) = msg {
            self.queue(m);
            return true;
        }

        // Handle Esc for back navigation
        if key.code == KeyCode::Esc {
            self.queue(SecretManagerMsg::NavigateBack);
            return true;
        }

        false
    }

    fn update(&mut self) -> UpdateResult {
        // Drain all pending messages
        let mut commands: Vec<Box<dyn Command>> = Vec::new();

        while let Ok(msg) = self.msg_rx.try_recv() {
            match self.process_message(msg) {
                UpdateResult::Idle => {}
                UpdateResult::Commands(cmds) => commands.extend(cmds),
                UpdateResult::Close => return UpdateResult::Close,
                UpdateResult::Error(e) => return UpdateResult::Error(e),
            }
        }

        if commands.is_empty() {
            UpdateResult::Idle
        } else {
            UpdateResult::Commands(commands)
        }
    }

    fn view(&mut self, frame: &mut Frame, area: Rect) {
        match &mut self.state {
            State::Loading => self.spinner.render(frame, area),
            State::SecretList(v) => v.render(frame, area),
            State::VersionList(v) => v.render(frame, area),
            State::Payload(v) => v.render(frame, area),
        }
    }

    fn breadcrumbs(&self) -> Vec<String> {
        let mut bc = vec!["Secret Manager".to_string()];

        match &self.state {
            State::Loading => bc.extend(self.loading_context.clone()),
            State::SecretList(_) => bc.push("Secrets".to_string()),
            State::VersionList(v) => {
                bc.push(v.secret().to_string());
                bc.push("Versions".to_string());
            }
            State::Payload(v) => {
                bc.push(v.secret().to_string());
                bc.push(format!("v{}", v.version()));
            }
        }
        bc
    }
}
