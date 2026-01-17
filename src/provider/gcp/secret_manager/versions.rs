use crate::Theme;
use crate::core::{Command, UpdateResult};
use crate::provider::gcp::secret_manager::SecretManager;
use crate::provider::gcp::secret_manager::client::SecretManagerClient;
use crate::provider::gcp::secret_manager::payload::PayloadMsg;
use crate::provider::gcp::secret_manager::secrets::Secret;
use crate::provider::gcp::secret_manager::service::SecretManagerMsg;
use crate::search::Matcher;
use crate::view::{
    ColumnDef, ConfirmDialog, ConfirmEvent, KeyResult, TableEvent, TableRow, TableView,
    TextInputEvent, TextInputView, View,
};
use async_trait::async_trait;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use ratatui::widgets::Cell;
use std::fmt::Display;
use tokio::sync::mpsc::UnboundedSender;

// === Models ===

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretVersion {
    pub version_id: String,
    pub state: String,
    pub created_at: String,
}

impl Display for SecretVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.version_id)
    }
}

impl TableRow for SecretVersion {
    fn columns() -> &'static [ColumnDef] {
        static COLUMNS: &[ColumnDef] = &[
            ColumnDef::new("Version", Constraint::Length(10)),
            ColumnDef::new("State", Constraint::Length(12)),
            ColumnDef::new("Created", Constraint::Min(20)),
        ];
        COLUMNS
    }

    fn render_cells(&self, _theme: &Theme) -> Vec<Cell<'static>> {
        vec![
            Cell::from(self.version_id.clone()),
            Cell::from(self.state.clone()),
            Cell::from(self.created_at.clone()),
        ]
    }

    fn matches(&self, query: &str) -> bool {
        let matcher = Matcher::new();
        matcher.matches(&self.version_id, query) || matcher.matches(&self.state, query)
    }
}

// === Messages ===

#[derive(Debug, Clone)]
pub enum VersionsMsg {
    /// Load versions for a secret
    Load(Secret),
    /// Versions loaded successfully
    Loaded { secret: Secret, versions: Vec<SecretVersion> },

    /// Show dialog to add a new version
    StartCreation(Secret),
    /// Add a new version to a secret
    Create { secret: Secret, payload: String },
    /// Version created successfully
    Created { secret: Secret },

    /// Disable a secret version
    Disable { secret: Secret, version: SecretVersion },
    /// Version disabled successfully
    Disabled { secret: Secret },

    /// Enable a secret version
    Enable { secret: Secret, version: SecretVersion },
    /// Version enabled successfully
    Enabled { secret: Secret },

    /// Show destroy confirmation for a version
    ConfirmDestroy { secret: Secret, version: SecretVersion },
    /// Confirmed destruction of a version
    Destroy { secret: Secret, version: SecretVersion },
    /// Version destroyed successfully
    Destroyed { secret: Secret },

    /// View payload for a specific version
    ViewPayload { secret: Secret, version: SecretVersion },
}

impl From<VersionsMsg> for SecretManagerMsg {
    fn from(msg: VersionsMsg) -> Self {
        SecretManagerMsg::Version(msg)
    }
}

impl From<VersionsMsg> for KeyResult<SecretManagerMsg> {
    fn from(msg: VersionsMsg) -> Self {
        KeyResult::Event(SecretManagerMsg::Version(msg))
    }
}

// === Screens ===

pub struct VersionListScreen {
    secret: Secret,
    table: TableView<SecretVersion>,
}

impl VersionListScreen {
    pub fn new(secret: Secret, versions: Vec<SecretVersion>) -> Self {
        let title = format!(" {} - Versions ", secret.name);
        Self {
            secret,
            table: TableView::new(versions).with_title(title),
        }
    }
}

impl View for VersionListScreen {
    type Event = SecretManagerMsg;

    fn handle_key(&mut self, key: KeyEvent) -> KeyResult<Self::Event> {
        // Delegate to table first (handles search mode, navigation, etc.)
        let result = self.table.handle_key(key);
        if let KeyResult::Event(TableEvent::Activated(version)) = result {
            return VersionsMsg::ViewPayload {
                secret: self.secret.clone(),
                version,
            }
            .into();
        }
        if result.is_consumed() {
            return KeyResult::Consumed;
        }

        // Handle local shortcuts only if table didn't consume the key
        match key.code {
            KeyCode::Char('r') => VersionsMsg::Load(self.secret.clone()).into(),
            // Add new version
            KeyCode::Char('a') | KeyCode::Char('n') => {
                VersionsMsg::StartCreation(self.secret.clone()).into()
            }
            // Disable version (only for Enabled versions)
            KeyCode::Char('d') => match self.table.selected_item() {
                Some(v) if v.state.contains("Enabled") => VersionsMsg::Disable {
                    secret: self.secret.clone(),
                    version: v.clone(),
                }
                .into(),
                _ => KeyResult::Ignored,
            },
            // Enable version (only for Disabled versions)
            KeyCode::Char('e') => match self.table.selected_item() {
                Some(v) if v.state.contains("Disabled") => VersionsMsg::Enable {
                    secret: self.secret.clone(),
                    version: v.clone(),
                }
                .into(),
                _ => KeyResult::Ignored,
            },
            // Destroy version (shift+D, only for non-Destroyed versions)
            KeyCode::Char('D') => match self.table.selected_item() {
                Some(v) if !v.state.contains("Destroyed") => VersionsMsg::ConfirmDestroy {
                    secret: self.secret.clone(),
                    version: v.clone(),
                }
                .into(),
                _ => KeyResult::Ignored,
            },
            _ => KeyResult::Ignored,
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        self.table.render(frame, area, theme);
    }
}

// === Dialogs ===

pub struct CreateVersionDialog {
    secret: Secret,
    input: TextInputView,
}

impl CreateVersionDialog {
    pub fn new(secret: Secret) -> Self {
        Self {
            secret,
            input: TextInputView::new("New Version Payload"),
        }
    }
}

impl View for CreateVersionDialog {
    type Event = SecretManagerMsg;

    fn handle_key(&mut self, key: KeyEvent) -> KeyResult<Self::Event> {
        match self.input.handle_key(key) {
            KeyResult::Event(TextInputEvent::Submitted(payload)) if !payload.is_empty() => {
                VersionsMsg::Create {
                    secret: self.secret.clone(),
                    payload,
                }
                .into()
            }
            KeyResult::Event(TextInputEvent::Cancelled) => SecretManagerMsg::DialogCancelled.into(),
            KeyResult::Event(_) => KeyResult::Consumed, // Empty submission
            _ => KeyResult::Consumed,
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        self.input.render(frame, area, theme);
    }
}

pub struct DestroyVersionDialog {
    secret: Secret,
    version: SecretVersion,
    dialog: ConfirmDialog,
}

impl DestroyVersionDialog {
    pub fn new(secret: Secret, version: SecretVersion) -> Self {
        let dialog = ConfirmDialog::new(format!(
            "Destroy version '{}'? This is permanent and cannot be undone.",
            version.version_id
        ))
        .with_title("Destroy Version")
        .with_confirm_text("Destroy")
        .danger();

        Self {
            secret,
            version,
            dialog,
        }
    }
}

impl View for DestroyVersionDialog {
    type Event = SecretManagerMsg;

    fn handle_key(&mut self, key: KeyEvent) -> KeyResult<Self::Event> {
        match self.dialog.handle_key(key) {
            KeyResult::Event(ConfirmEvent::Confirmed) => VersionsMsg::Destroy {
                secret: self.secret.clone(),
                version: self.version.clone(),
            }
            .into(),
            KeyResult::Event(ConfirmEvent::Cancelled) => SecretManagerMsg::DialogCancelled.into(),
            _ => KeyResult::Consumed,
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        self.dialog.render(frame, area, theme);
    }
}

// === Update Logic ===

pub(super) fn update(
    state: &mut SecretManager,
    msg: VersionsMsg,
) -> color_eyre::Result<UpdateResult> {
    match msg {
        VersionsMsg::Load(secret) => {
            // Use cached versions if available
            if let Some(versions) = state.get_cached_versions(&secret) {
                state.push_view(VersionListScreen::new(secret, versions));
                return Ok(UpdateResult::Idle);
            }

            state.display_loading_spinner("Loading versions...");

            Ok(FetchVersionsCmd {
                secret,
                client: state.get_client()?,
                tx: state.get_msg_sender(),
            }
            .into())
        }

        VersionsMsg::Loaded { secret, versions } => {
            state.hide_loading_spinner();
            state.cache_versions(&secret, versions.clone());
            state.push_view(VersionListScreen::new(secret, versions));
            Ok(UpdateResult::Idle)
        }

        VersionsMsg::StartCreation(secret) => {
            state.display_overlay(CreateVersionDialog::new(secret));
            Ok(UpdateResult::Idle)
        }

        VersionsMsg::Create { secret, payload } => {
            state.display_loading_spinner("Creating version...");
            state.close_overlay();
            state.invalidate_versions_cache(&secret);

            Ok(AddVersionCmd {
                secret,
                payload,
                client: state.get_client()?,
                tx: state.get_msg_sender(),
            }
            .into())
        }

        VersionsMsg::Created { secret } => {
            state.pop_view();
            state.queue(VersionsMsg::Load(secret).into());
            Ok(UpdateResult::Idle)
        }

        VersionsMsg::Disable { secret, version } => {
            state.display_loading_spinner("Disabling version...");
            state.invalidate_versions_cache(&secret);

            Ok(DisableVersionCmd {
                secret,
                version,
                client: state.get_client()?,
                tx: state.get_msg_sender(),
            }
            .into())
        }

        VersionsMsg::Disabled { secret } => {
            state.pop_view();
            state.queue(VersionsMsg::Load(secret).into());
            Ok(UpdateResult::Idle)
        }

        VersionsMsg::Enable { secret, version } => {
            state.display_loading_spinner("Enabling version...");
            state.invalidate_versions_cache(&secret);

            Ok(EnableVersionCmd {
                secret,
                version,
                client: state.get_client()?,
                tx: state.get_msg_sender(),
            }
            .into())
        }

        VersionsMsg::Enabled { secret } => {
            state.pop_view();
            state.queue(VersionsMsg::Load(secret).into());
            Ok(UpdateResult::Idle)
        }

        VersionsMsg::ConfirmDestroy { secret, version } => {
            state.display_overlay(DestroyVersionDialog::new(secret, version));
            Ok(UpdateResult::Idle)
        }

        VersionsMsg::Destroy { secret, version } => {
            state.display_loading_spinner("Destroying version...");
            state.close_overlay();
            state.invalidate_versions_cache(&secret);

            Ok(DestroyVersionCmd {
                secret,
                version,
                client: state.get_client()?,
                tx: state.get_msg_sender(),
            }
            .into())
        }

        VersionsMsg::Destroyed { secret } => {
            state.pop_view();
            state.queue(VersionsMsg::Load(secret).into());
            Ok(UpdateResult::Idle)
        }

        VersionsMsg::ViewPayload { secret, version } => {
            state.queue(
                PayloadMsg::Load {
                    secret,
                    version: Some(version),
                }
                .into(),
            );
            Ok(UpdateResult::Idle)
        }
    }
}

// === Commands ===

/// Fetch versions for a secret.
struct FetchVersionsCmd {
    client: SecretManagerClient,
    secret: Secret,
    tx: UnboundedSender<SecretManagerMsg>,
}

#[async_trait]
impl Command for FetchVersionsCmd {
    fn name(&self) -> &'static str {
        "Loading versions"
    }

    async fn execute(self: Box<Self>) -> color_eyre::Result<()> {
        let versions = self.client.list_versions(&self.secret.name).await?;
        self.tx.send(
            VersionsMsg::Loaded {
                secret: self.secret,
                versions,
            }
            .into(),
        )?;
        Ok(())
    }
}

/// Add a new version to a secret.
struct AddVersionCmd {
    client: SecretManagerClient,
    secret: Secret,
    payload: String,
    tx: UnboundedSender<SecretManagerMsg>,
}

#[async_trait]
impl Command for AddVersionCmd {
    fn name(&self) -> &'static str {
        "Adding secret version"
    }

    async fn execute(self: Box<Self>) -> color_eyre::Result<()> {
        self.client
            .add_secret_version(&self.secret.name, self.payload.as_bytes())
            .await?;
        self.tx
            .send(VersionsMsg::Created { secret: self.secret }.into())?;
        Ok(())
    }
}

/// Disable a secret version.
struct DisableVersionCmd {
    client: SecretManagerClient,
    secret: Secret,
    version: SecretVersion,
    tx: UnboundedSender<SecretManagerMsg>,
}

#[async_trait]
impl Command for DisableVersionCmd {
    fn name(&self) -> &'static str {
        "Disabling version"
    }

    async fn execute(self: Box<Self>) -> color_eyre::Result<()> {
        self.client
            .disable_version(&self.secret.name, &self.version.version_id)
            .await?;
        self.tx
            .send(VersionsMsg::Disabled { secret: self.secret }.into())?;
        Ok(())
    }
}

/// Enable a secret version.
struct EnableVersionCmd {
    client: SecretManagerClient,
    secret: Secret,
    version: SecretVersion,
    tx: UnboundedSender<SecretManagerMsg>,
}

#[async_trait]
impl Command for EnableVersionCmd {
    fn name(&self) -> &'static str {
        "Enabling version"
    }

    async fn execute(self: Box<Self>) -> color_eyre::Result<()> {
        self.client
            .enable_version(&self.secret.name, &self.version.version_id)
            .await?;
        self.tx
            .send(VersionsMsg::Enabled { secret: self.secret }.into())?;
        Ok(())
    }
}

/// Destroy a secret version.
struct DestroyVersionCmd {
    client: SecretManagerClient,
    secret: Secret,
    version: SecretVersion,
    tx: UnboundedSender<SecretManagerMsg>,
}

#[async_trait]
impl Command for DestroyVersionCmd {
    fn name(&self) -> &'static str {
        "Destroying version"
    }

    async fn execute(self: Box<Self>) -> color_eyre::Result<()> {
        self.client
            .destroy_version(&self.secret.name, &self.version.version_id)
            .await?;
        self.tx
            .send(VersionsMsg::Destroyed { secret: self.secret }.into())?;
        Ok(())
    }
}
