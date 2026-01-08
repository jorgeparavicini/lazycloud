use crate::Theme;
use crate::core::command::{Command, CopyToClipboardCmd};
use crate::core::event::Event;
use crate::core::service::{Service, UpdateResult};
use crate::model::GcpContext;
use crate::provider::gcp::secret_manager::client::SecretManagerClient;
use crate::provider::gcp::secret_manager::command::{
    FetchPayloadCmd, FetchSecretsCmd, FetchVersionsCmd, InitClientCmd,
};
use crate::provider::gcp::secret_manager::message::SecretManagerMsg;
use crate::provider::gcp::secret_manager::model::{Secret, SecretPayload, SecretVersion};
use crate::provider::gcp::secret_manager::view::{
    PayloadView, SecretListView, ServiceView, VersionListView,
};
use crate::view::{KeyResult, SpinnerView, View};
use crossterm::event::KeyCode;
use ratatui::Frame;
use ratatui::layout::Rect;
use std::collections::HashMap;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

/// Service for managing GCP Secret Manager secrets.
pub struct SecretManager {
    project_id: String,
    spinner: SpinnerView,
    client: Option<SecretManagerClient>,
    /// Navigation stack - top is current view.
    view_stack: Vec<Box<dyn ServiceView>>,
    /// Loading overlay label (None = not loading).
    loading: Option<&'static str>,
    msg_tx: UnboundedSender<SecretManagerMsg>,
    msg_rx: UnboundedReceiver<SecretManagerMsg>,
    /// Cached versions by secret name.
    cached_versions: HashMap<String, Vec<SecretVersion>>,
    /// Cached payloads by "secret_name/version_id".
    cached_payloads: HashMap<String, SecretPayload>,
}

impl SecretManager {
    pub fn new(ctx: &GcpContext) -> Self {
        let (msg_tx, msg_rx) = mpsc::unbounded_channel();
        Self {
            project_id: ctx.project_id.clone(),
            spinner: SpinnerView::new(),
            client: None,
            view_stack: Vec::new(),
            loading: Some("Initializing..."),
            msg_tx,
            msg_rx,
            cached_versions: HashMap::new(),
            cached_payloads: HashMap::new(),
        }
    }

    fn payload_cache_key(secret_name: &str, version_id: &str) -> String {
        format!("{}/{}", secret_name, version_id)
    }

    /// Queue a message to be processed by update().
    fn queue(&self, msg: SecretManagerMsg) {
        let _ = self.msg_tx.send(msg);
    }

    fn current_view_mut(&mut self) -> Option<&mut Box<dyn ServiceView>> {
        self.view_stack.last_mut()
    }

    fn push_view(&mut self, view: Box<dyn ServiceView>) {
        self.loading = None;
        self.view_stack.push(view);
    }

    fn pop_view(&mut self) -> bool {
        if self.view_stack.len() > 1 {
            self.view_stack.pop();
            true
        } else {
            false
        }
    }

    /// Process a single message and return the result.
    fn process_message(&mut self, msg: SecretManagerMsg) -> UpdateResult {
        match msg {
            SecretManagerMsg::Initialize => {
                self.loading = Some("Initializing Secret Manager...");
                UpdateResult::Commands(vec![Box::new(InitClientCmd::new(
                    self.project_id.clone(),
                    self.msg_tx.clone(),
                ))])
            }

            SecretManagerMsg::NavigateBack => {
                if self.pop_view() {
                    UpdateResult::Idle
                } else {
                    UpdateResult::Close
                }
            }

            SecretManagerMsg::ReloadData => self.reload_current_view(),

            SecretManagerMsg::LoadSecrets => self.load_secrets(),

            SecretManagerMsg::SelectSecret(secret) => self.load_versions(secret),

            SecretManagerMsg::SelectVersion(secret, version) => self.load_payload(secret, version),

            SecretManagerMsg::CopyPayload(data) => {
                UpdateResult::Commands(vec![Box::new(CopyToClipboardCmd::new(data))])
            }

            SecretManagerMsg::ClientInitialized(client) => {
                self.client = Some(client);
                self.load_secrets()
            }

            SecretManagerMsg::SecretsLoaded(secrets) => {
                self.push_view(Box::new(SecretListView::new(secrets)));
                UpdateResult::Idle
            }

            SecretManagerMsg::VersionsLoaded { secret, versions } => {
                self.cached_versions
                    .insert(secret.name.clone(), versions.clone());
                self.push_view(Box::new(VersionListView::new(secret, versions)));
                UpdateResult::Idle
            }

            SecretManagerMsg::PayloadLoaded {
                secret,
                version,
                payload,
            } => {
                let key = Self::payload_cache_key(&secret.name, &version.version_id);
                self.cached_payloads.insert(key, payload.clone());
                self.push_view(Box::new(PayloadView::new(secret, version, payload)));
                UpdateResult::Idle
            }

            SecretManagerMsg::OperationFailed(err) => {
                self.loading = None;
                UpdateResult::Error(err)
            }
        }
    }

    fn reload_current_view(&mut self) -> UpdateResult {
        if let Some(view) = self.view_stack.pop() {
            let msg = view.reload();
            // Clear cache based on reload message
            match &msg {
                SecretManagerMsg::SelectSecret(secret) => {
                    self.cached_versions.remove(&secret.name);
                }
                SecretManagerMsg::SelectVersion(secret, version) => {
                    let key = Self::payload_cache_key(&secret.name, &version.version_id);
                    self.cached_payloads.remove(&key);
                }
                _ => {}
            }
            self.process_message(msg)
        } else {
            UpdateResult::Idle
        }
    }

    fn load_secrets(&mut self) -> UpdateResult {
        self.loading = Some("Loading secrets...");
        if let Some(client) = &self.client {
            UpdateResult::Commands(vec![Box::new(FetchSecretsCmd::new(
                client.clone(),
                self.msg_tx.clone(),
            ))])
        } else {
            UpdateResult::Error("Client not initialized".to_string())
        }
    }

    fn load_versions(&mut self, secret: Secret) -> UpdateResult {
        // Use cached versions if available
        if let Some(versions) = self.cached_versions.get(&secret.name) {
            self.push_view(Box::new(VersionListView::new(secret, versions.clone())));
            return UpdateResult::Idle;
        }

        self.loading = Some("Loading versions...");
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

    fn load_payload(&mut self, secret: Secret, version: SecretVersion) -> UpdateResult {
        // Use cached payload if available
        let key = Self::payload_cache_key(&secret.name, &version.version_id);
        if let Some(payload) = self.cached_payloads.get(&key) {
            self.push_view(Box::new(PayloadView::new(secret, version, payload.clone())));
            return UpdateResult::Idle;
        }

        self.loading = Some("Loading payload...");
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

}

impl Service for SecretManager {
    fn init(&mut self) {
        self.queue(SecretManagerMsg::Initialize);
    }

    fn handle_tick(&mut self) {
        if self.loading.is_some() {
            self.spinner.on_tick();
        }
    }

    fn handle_input(&mut self, event: &Event) -> bool {
        let Event::Key(key) = event else {
            return false;
        };

        // Don't handle input while loading
        if self.loading.is_some() {
            return false;
        }

        // Let the current view handle the key
        let result = if let Some(view) = self.current_view_mut() {
            view.handle_key(*key)
        } else {
            KeyResult::Ignored
        };

        if let KeyResult::Event(m) = result {
            self.queue(m);
            return true;
        }

        if result.is_consumed() {
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

    fn view(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        if let Some(label) = self.loading {
            self.spinner.set_label(label);
            self.spinner.render(frame, area, theme);
        } else if let Some(view) = self.current_view_mut() {
            view.render(frame, area, theme);
        }
    }

    fn breadcrumbs(&self) -> Vec<String> {
        let mut bc = vec!["Secret Manager".to_string()];

        for view in &self.view_stack {
            bc.extend(view.breadcrumbs());
        }

        bc
    }
}
