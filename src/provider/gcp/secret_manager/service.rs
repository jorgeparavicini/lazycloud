use crate::core::command::{Command, CopyToClipboardCmd};
use crate::core::event::Event;
use crate::core::service::{Service, UpdateResult};
use crate::model::GcpContext;
use crate::provider::gcp::secret_manager::client::SecretManagerClient;
use crate::provider::gcp::secret_manager::command::{
    AddVersionCmd, CreateSecretCmd, DeleteSecretCmd, DestroyVersionCmd, DisableVersionCmd,
    EnableVersionCmd, FetchIamPolicyCmd, FetchLatestPayloadCmd, FetchPayloadCmd,
    FetchSecretMetadataCmd, FetchSecretsCmd, FetchVersionsCmd, InitClientCmd, UpdateLabelsCmd,
};
use crate::provider::gcp::secret_manager::message::SecretManagerMsg;
use crate::provider::gcp::secret_manager::model::{Secret, SecretPayload, SecretVersion};
use crate::provider::gcp::secret_manager::view::{
    CreateSecretNameOverlay, CreateSecretPayloadOverlay, CreateVersionOverlay, DeleteSecretOverlay,
    DestroyVersionOverlay, IamPolicyView, LabelsView, PayloadView, ReplicationView, SecretListView,
    SecretManagerView, VersionListView,
};
use crate::view::{KeyResult, SpinnerView, View};
use crate::Theme;
use crossterm::event::KeyCode;
use ratatui::layout::Rect;
use ratatui::Frame;
use std::collections::HashMap;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

/// Service for managing GCP Secret Manager secrets.
pub struct SecretManager {
    project_id: String,
    account: String,
    spinner: SpinnerView,
    client: Option<SecretManagerClient>,
    /// Navigation stack - top is current view.
    view_stack: Vec<Box<dyn SecretManagerView>>,
    /// Loading overlay label (None = not loading).
    loading: Option<&'static str>,
    /// Active overlay dialog.
    overlay: Option<Box<dyn View<Event = SecretManagerMsg>>>,
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
            account: ctx.account.clone(),
            spinner: SpinnerView::new(),
            client: None,
            view_stack: Vec::new(),
            loading: Some("Initializing..."),
            overlay: None,
            msg_tx,
            msg_rx,
            cached_versions: HashMap::new(),
            cached_payloads: HashMap::new(),
        }
    }

    fn payload_cache_key(secret: &Secret, version: &Option<SecretVersion>) -> String {
        let version_id = version
            .as_ref()
            .map(|v| v.version_id.as_str())
            .unwrap_or("latest");
        format!("{}/{}", secret.name, version_id)
    }

    /// Queue a message to be processed by update().
    fn queue(&self, msg: SecretManagerMsg) {
        let _ = self.msg_tx.send(msg);
    }

    fn current_view_mut(&mut self) -> Option<&mut Box<dyn SecretManagerView>> {
        self.view_stack.last_mut()
    }

    fn push_view<T: SecretManagerView + 'static>(&mut self, view: T) {
        self.loading = None;
        self.view_stack.push(Box::new(view));
    }

    fn pop_view(&mut self) -> bool {
        if self.view_stack.len() > 1 {
            self.view_stack.pop();
            true
        } else {
            false
        }
    }

    fn show_overlay<T: View<Event = SecretManagerMsg> + 'static>(&mut self, overlay: T) {
        self.overlay = Some(Box::new(overlay));
    }

    /// Process a single message and return the result.
    fn process_message(&mut self, msg: SecretManagerMsg) -> UpdateResult {
        match msg {
            // === Lifecycle ===
            SecretManagerMsg::Initialize => self.initialize_client(),

            // === Navigation ===
            SecretManagerMsg::NavigateBack => {
                if self.pop_view() {
                    UpdateResult::Idle
                } else {
                    UpdateResult::Close
                }
            }
            SecretManagerMsg::ReloadData => self.reload_current_view(),
            SecretManagerMsg::DialogCancelled => self.close_dialog(),

            // === Secrets ===
            SecretManagerMsg::LoadSecrets => self.load_secrets(),
            SecretManagerMsg::ShowCreateSecretDialog => self.show_create_secret_dialog(),
            SecretManagerMsg::CreateSecretStep2 { name } => self.show_create_secret_payload(name),
            SecretManagerMsg::CreateSecret { name, payload } => self.create_secret(name, payload),
            SecretManagerMsg::ShowDeleteSecretDialog(secret) => {
                self.show_delete_secret_dialog(secret)
            }
            SecretManagerMsg::DeleteSecret(secret) => self.delete_secret(secret),

            // === Version ===
            SecretManagerMsg::LoadVersions(secret) => self.load_versions(secret),
            SecretManagerMsg::ShowCreateVersionDialog(secret) => {
                self.show_create_version_dialog(secret)
            }
            SecretManagerMsg::CreateVersion { secret, payload } => {
                self.create_version(secret, payload)
            }
            SecretManagerMsg::DisableVersion { secret, version } => {
                self.disable_version(&secret, version)
            }
            SecretManagerMsg::EnableVersion { secret, version } => {
                self.enable_version(&secret, version)
            }
            SecretManagerMsg::ShowDestroyVersionDialog { secret, version } => {
                self.show_destroy_version_dialog(secret, version)
            }
            SecretManagerMsg::DestroyVersion { secret, version } => {
                self.destroy_version(&secret, version)
            }

            // === Payload ===
            SecretManagerMsg::LoadPayload(secret, version) => self.load_payload(secret, version),
            SecretManagerMsg::CopyPayload(data) => CopyToClipboardCmd::new(data).into(),

            // === Metadata ===
            SecretManagerMsg::ShowLabels(secret) => self.show_labels(secret),
            SecretManagerMsg::UpdateLabels { secret, labels } => self.update_labels(secret, labels),
            SecretManagerMsg::ShowIamPolicy(secret) => self.show_iam_policy(secret),
            SecretManagerMsg::ShowReplicationInfo(secret) => self.show_replication_info(secret),

            // === Async Results ===
            SecretManagerMsg::ClientInitialized(client) => {
                self.client = Some(client);
                self.load_secrets()
            }
            SecretManagerMsg::SecretsLoaded(secrets) => {
                self.push_view(SecretListView::new(secrets));
                UpdateResult::Idle
            }
            SecretManagerMsg::VersionsLoaded { secret, versions } => {
                self.cached_versions
                    .insert(secret.name.clone(), versions.clone());
                self.push_view(VersionListView::new(secret, versions));
                UpdateResult::Idle
            }
            SecretManagerMsg::PayloadLoaded {
                secret,
                version,
                payload,
            } => {
                let key = Self::payload_cache_key(&secret, &version);
                self.cached_payloads.insert(key, payload.clone());
                self.push_view(PayloadView::new(secret, version, payload));
                UpdateResult::Idle
            }
            SecretManagerMsg::SecretCreated(_secret) => self.load_secrets(),
            SecretManagerMsg::SecretDeleted(_name) => {
                // Pop back to secrets list and refresh
                while self.view_stack.len() > 1 {
                    self.view_stack.pop();
                }
                self.view_stack.clear();
                self.load_secrets()
            }
            SecretManagerMsg::VersionAdded { secret, .. }
            | SecretManagerMsg::VersionDisabled { secret, .. }
            | SecretManagerMsg::VersionEnabled { secret, .. }
            | SecretManagerMsg::VersionDestroyed { secret, .. } => {
                // Refresh versions list
                self.cached_versions.remove(&secret.name);
                self.view_stack.pop();
                self.load_versions(secret)
            }
            SecretManagerMsg::LabelsUpdated(secret) => {
                // Refresh labels view
                self.view_stack.pop();
                self.push_view(LabelsView::new(secret));
                UpdateResult::Idle
            }
            SecretManagerMsg::IamPolicyLoaded { secret, policy } => {
                self.push_view(IamPolicyView::new(secret, policy));
                UpdateResult::Idle
            }
            SecretManagerMsg::ReplicationInfoLoaded {
                secret,
                replication,
            } => {
                self.push_view(ReplicationView::new(secret, replication));
                UpdateResult::Idle
            }
        }
    }

    // === Lifecycle ===

    fn initialize_client(&mut self) -> UpdateResult {
        self.loading = Some("Initializing Secret Manager...");
        InitClientCmd::new(
            self.project_id.clone(),
            self.account.clone(),
            self.msg_tx.clone(),
        )
        .into()
    }

    // === Navigation ===

    fn reload_current_view(&mut self) -> UpdateResult {
        if let Some(view) = self.view_stack.pop() {
            let msg = view.reload();
            // Clear cache based on reload message
            match &msg {
                SecretManagerMsg::LoadVersions(secret) => {
                    self.cached_versions.remove(&secret.name);
                }
                SecretManagerMsg::LoadPayload(secret, version) => {
                    let key = Self::payload_cache_key(&secret, &version);
                    self.cached_payloads.remove(&key);
                }
                _ => {}
            }
            self.process_message(msg)
        } else {
            UpdateResult::Idle
        }
    }

    fn close_dialog(&mut self) -> UpdateResult {
        self.overlay = None;
        UpdateResult::Idle
    }

    // === Secrets ===

    fn load_secrets(&mut self) -> UpdateResult {
        self.loading = Some("Loading secrets...");
        if let Some(client) = &self.client {
            FetchSecretsCmd::new(client.clone(), self.msg_tx.clone()).into()
        } else {
            UpdateResult::Error("Client not initialized".to_string())
        }
    }

    fn show_create_secret_dialog(&mut self) -> UpdateResult {
        self.show_overlay(CreateSecretNameOverlay::new());
        UpdateResult::Idle
    }

    fn show_create_secret_payload(&mut self, name: String) -> UpdateResult {
        self.show_overlay(CreateSecretPayloadOverlay::new(name));
        UpdateResult::Idle
    }

    fn create_secret(&mut self, name: String, payload: Option<String>) -> UpdateResult {
        self.loading = Some("Creating secret...");
        if let Some(client) = &self.client {
            CreateSecretCmd::new(client.clone(), name, payload, self.msg_tx.clone()).into()
        } else {
            UpdateResult::Error("Client not initialized".to_string())
        }
    }

    fn show_delete_secret_dialog(&mut self, secret: Secret) -> UpdateResult {
        self.show_overlay(DeleteSecretOverlay::new(secret));
        UpdateResult::Idle
    }

    fn delete_secret(&mut self, secret: Secret) -> UpdateResult {
        self.loading = Some("Deleting secret...");
        if let Some(client) = &self.client {
            DeleteSecretCmd::new(client.clone(), secret, self.msg_tx.clone()).into()
        } else {
            UpdateResult::Error("Client not initialized".to_string())
        }
    }

    // === Versions ===

    fn load_versions(&mut self, secret: Secret) -> UpdateResult {
        // Use cached versions if available
        if let Some(versions) = self.cached_versions.get(&secret.name) {
            self.push_view(VersionListView::new(secret, versions.clone()));
            return UpdateResult::Idle;
        }

        self.loading = Some("Loading versions...");
        if let Some(client) = &self.client {
            FetchVersionsCmd::new(client.clone(), secret, self.msg_tx.clone()).into()
        } else {
            UpdateResult::Error("Client not initialized".to_string())
        }
    }

    fn show_create_version_dialog(&mut self, secret: Secret) -> UpdateResult {
        self.show_overlay(CreateVersionOverlay::new(secret));
        UpdateResult::Idle
    }

    fn create_version(&mut self, secret: Secret, payload: String) -> UpdateResult {
        self.loading = Some("Creating version...");
        self.cached_versions.remove(&secret.name);
        if let Some(client) = &self.client {
            AddVersionCmd::new(client.clone(), secret, payload, self.msg_tx.clone()).into()
        } else {
            UpdateResult::Error("Client not initialized".to_string())
        }
    }

    fn disable_version(&mut self, secret: &Secret, version: SecretVersion) -> UpdateResult {
        self.loading = Some("Disabling version...");
        self.cached_versions.remove(&secret.name);
        if let Some(client) = &self.client {
            DisableVersionCmd::new(client.clone(), secret.clone(), version, self.msg_tx.clone())
                .into()
        } else {
            UpdateResult::Error("Client not initialized".to_string())
        }
    }

    fn enable_version(&mut self, secret: &Secret, version: SecretVersion) -> UpdateResult {
        self.loading = Some("Enabling version...");
        self.cached_versions.remove(&secret.name);
        if let Some(client) = &self.client {
            EnableVersionCmd::new(client.clone(), secret.clone(), version, self.msg_tx.clone())
                .into()
        } else {
            UpdateResult::Error("Client not initialized".to_string())
        }
    }

    fn show_destroy_version_dialog(
        &mut self,
        secret: Secret,
        version: SecretVersion,
    ) -> UpdateResult {
        self.show_overlay(DestroyVersionOverlay::new(secret, version));
        UpdateResult::Idle
    }

    fn destroy_version(&mut self, secret: &Secret, version: SecretVersion) -> UpdateResult {
        self.loading = Some("Destroying version...");
        self.cached_versions.remove(&secret.name);
        if let Some(client) = &self.client {
            DestroyVersionCmd::new(client.clone(), secret.clone(), version, self.msg_tx.clone())
                .into()
        } else {
            UpdateResult::Error("Client not initialized".to_string())
        }
    }

    // === Payload ===

    fn load_payload(&mut self, secret: Secret, version: Option<SecretVersion>) -> UpdateResult {
        // Use cached payload if available
        let key = Self::payload_cache_key(&secret, &version);
        if let Some(payload) = self.cached_payloads.get(&key) {
            self.push_view(PayloadView::new(secret, version, payload.clone()));
            return UpdateResult::Idle;
        }

        self.loading = Some("Loading payload...");
        if let Some(client) = &self.client {
            match version {
                Some(v) => {
                    FetchPayloadCmd::new(client.clone(), secret, v, self.msg_tx.clone()).into()
                }
                None => {
                    FetchLatestPayloadCmd::new(client.clone(), secret, self.msg_tx.clone()).into()
                }
            }
        } else {
            UpdateResult::Error("Client not initialized".to_string())
        }
    }

    // === Metadata ===

    fn show_labels(&mut self, secret: Secret) -> UpdateResult {
        self.push_view(LabelsView::new(secret));
        UpdateResult::Idle
    }

    fn update_labels(&mut self, secret: Secret, labels: HashMap<String, String>) -> UpdateResult {
        self.loading = Some("Updating labels...");
        if let Some(client) = &self.client {
            UpdateLabelsCmd::new(client.clone(), secret, labels, self.msg_tx.clone()).into()
        } else {
            UpdateResult::Error("Client not initialized".to_string())
        }
    }

    fn show_iam_policy(&mut self, secret: Secret) -> UpdateResult {
        self.loading = Some("Loading IAM policy...");
        if let Some(client) = &self.client {
            FetchIamPolicyCmd::new(client.clone(), secret, self.msg_tx.clone()).into()
        } else {
            UpdateResult::Error("Client not initialized".to_string())
        }
    }

    fn show_replication_info(&mut self, secret: Secret) -> UpdateResult {
        self.loading = Some("Loading replication info...");
        if let Some(client) = &self.client {
            FetchSecretMetadataCmd::new(client.clone(), secret, self.msg_tx.clone()).into()
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

        if self.loading.is_some() {
            return false;
        }

        if let Some(overlay) = &mut self.overlay {
            match overlay.handle_key(*key) {
                KeyResult::Event(msg) => {
                    self.queue(msg);
                    return true;
                }
                KeyResult::Consumed => return true,
                KeyResult::Ignored => {}
            }
        }

        if let Some(view) = self.current_view_mut() {
            match view.handle_key(*key) {
                KeyResult::Event(msg) => {
                    self.queue(msg);
                    return true;
                }
                KeyResult::Consumed => return true,
                KeyResult::Ignored => {}
            }
        }

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

        // Render overlay on top if present
        if let Some(overlay) = &mut self.overlay {
            overlay.render(frame, area, theme);
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
