use std::fmt::Display;
use std::sync::Arc;

use async_trait::async_trait;
use crossterm::event::KeyEvent;
use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use ratatui::widgets::Cell;
use tokio::sync::mpsc::UnboundedSender;

use crate::Theme;
use crate::ui::{
    ColumnDef,
    Component,
    ConfirmDialog,
    ConfirmEvent,
    EventResult,
    Keybinding,
    Modal,
    Result,
    Screen,
    Table,
    TableEvent,
    TableRow,
    TextInput,
    TextInputEvent,
};
use crate::config::{KeyResolver, SearchAction, VersionsAction};
use crate::commands::{Command, CommandEnv};
use crate::service::ServiceMsg;
use crate::provider::gcp::secret_manager::SecretManager;
use crate::provider::gcp::secret_manager::client::SecretManagerClient;
use crate::provider::gcp::secret_manager::payload::PayloadMsg;
use crate::provider::gcp::secret_manager::secrets::Secret;
use crate::provider::gcp::secret_manager::service::SecretManagerMsg;
use crate::search::Matcher;

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
    Load(Secret),
    Loaded {
        secret: Secret,
        versions: Vec<SecretVersion>,
    },

    StartCreation(Secret),
    Create {
        secret: Secret,
        payload: String,
    },
    Created {
        secret: Secret,
    },

    Disable {
        secret: Secret,
        version: SecretVersion,
    },
    Disabled {
        secret: Secret,
    },

    Enable {
        secret: Secret,
        version: SecretVersion,
    },
    Enabled {
        secret: Secret,
    },

    ConfirmDestroy {
        secret: Secret,
        version: SecretVersion,
    },
    /// Permanently destroys the version. Cannot be undone.
    Destroy {
        secret: Secret,
        version: SecretVersion,
    },
    Destroyed {
        secret: Secret,
    },

    ViewPayload {
        secret: Secret,
        version: SecretVersion,
    },
}

impl From<VersionsMsg> for SecretManagerMsg {
    fn from(msg: VersionsMsg) -> Self {
        Self::Version(msg)
    }
}

impl From<VersionsMsg> for EventResult<SecretManagerMsg> {
    fn from(msg: VersionsMsg) -> Self {
        Self::Event(SecretManagerMsg::Version(msg))
    }
}

// === Screens ===

pub struct VersionListScreen {
    secret: Secret,
    table: Table<SecretVersion>,
    resolver: Arc<KeyResolver>,
}

impl VersionListScreen {
    pub fn new(secret: Secret, versions: Vec<SecretVersion>, resolver: Arc<KeyResolver>) -> Self {
        let title = format!(" {} - Versions ", secret.name);
        Self {
            secret,
            table: Table::new(versions, resolver.clone()).with_title(title),
            resolver,
        }
    }
}

impl Screen for VersionListScreen {
    type Output = SecretManagerMsg;

    fn handle_key(&mut self, key: KeyEvent) -> Result<EventResult<Self::Output>> {
        // Delegate to table first (handles search mode, navigation, etc.)
        let result = self.table.handle_key(key)?;
        if let EventResult::Event(TableEvent::Activated(version)) = result {
            return Ok(VersionsMsg::ViewPayload {
                secret: self.secret.clone(),
                version,
            }
            .into());
        }
        if result.is_consumed() {
            return Ok(EventResult::Consumed);
        }

        // Handle local shortcuts only if table didn't consume the key
        if self.resolver.matches_versions(&key, VersionsAction::Reload) {
            return Ok(VersionsMsg::Load(self.secret.clone()).into());
        }
        if self.resolver.matches_versions(&key, VersionsAction::Add) {
            return Ok(VersionsMsg::StartCreation(self.secret.clone()).into());
        }
        if self
            .resolver
            .matches_versions(&key, VersionsAction::Disable)
            && let Some(v) = self.table.selected_item()
                && v.state.contains("Enabled") {
                    return Ok(VersionsMsg::Disable {
                        secret: self.secret.clone(),
                        version: v.clone(),
                    }
                    .into());
                }
        if self.resolver.matches_versions(&key, VersionsAction::Enable)
            && let Some(v) = self.table.selected_item()
                && v.state.contains("Disabled") {
                    return Ok(VersionsMsg::Enable {
                        secret: self.secret.clone(),
                        version: v.clone(),
                    }
                    .into());
                }
        if self
            .resolver
            .matches_versions(&key, VersionsAction::Destroy)
            && let Some(v) = self.table.selected_item()
                && !v.state.contains("Destroyed") {
                    return Ok(VersionsMsg::ConfirmDestroy {
                        secret: self.secret.clone(),
                        version: v.clone(),
                    }
                    .into());
                }

        Ok(EventResult::Ignored)
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        self.table.render(frame, area, theme);
    }

    fn keybindings(&self) -> Vec<Keybinding> {
        vec![
            Keybinding::hint(
                self.resolver.display_versions(VersionsAction::ViewPayload),
                "Payload",
            ),
            Keybinding::hint(
                self.resolver.display_versions(VersionsAction::Add),
                "Add version",
            ),
            Keybinding::hint(self.resolver.display_search(SearchAction::Toggle), "Search"),
            Keybinding::new(
                self.resolver.display_versions(VersionsAction::Disable),
                "Disable",
            ),
            Keybinding::new(
                self.resolver.display_versions(VersionsAction::Enable),
                "Enable",
            ),
            Keybinding::new(
                self.resolver.display_versions(VersionsAction::Destroy),
                "Destroy",
            ),
            Keybinding::new(
                self.resolver.display_versions(VersionsAction::Reload),
                "Reload",
            ),
        ]
    }
}

// === Dialogs ===

pub struct CreateVersionDialog {
    secret: Secret,
    input: TextInput,
    _resolver: Arc<KeyResolver>,
}

impl CreateVersionDialog {
    pub fn new(secret: Secret, resolver: Arc<KeyResolver>) -> Self {
        Self {
            secret,
            input: TextInput::new("New Version Payload"),
            _resolver: resolver,
        }
    }
}

impl Modal for CreateVersionDialog {
    type Output = SecretManagerMsg;

    fn handle_key(&mut self, key: KeyEvent) -> Result<EventResult<Self::Output>> {
        Ok(match self.input.handle_key(key)? {
            EventResult::Event(TextInputEvent::Submitted(payload)) if !payload.is_empty() => {
                VersionsMsg::Create {
                    secret: self.secret.clone(),
                    payload,
                }
                .into()
            }
            EventResult::Event(TextInputEvent::Cancelled) => SecretManagerMsg::DialogCancelled.into(),
            // Empty submission
            _ => EventResult::Consumed,
        })
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        self.input.render(frame, area, theme);
    }
}

pub struct DestroyVersionDialog {
    secret: Secret,
    version: SecretVersion,
    dialog: ConfirmDialog,
    _resolver: Arc<KeyResolver>,
}

impl DestroyVersionDialog {
    pub fn new(secret: Secret, version: SecretVersion, resolver: Arc<KeyResolver>) -> Self {
        let dialog = ConfirmDialog::new(
            format!(
                "Destroy version '{}'? This is permanent and cannot be undone.",
                version.version_id
            ),
            resolver.clone(),
        )
        .with_title("Destroy Version")
        .with_confirm_text("Destroy")
        .danger();

        Self {
            secret,
            version,
            dialog,
            _resolver: resolver,
        }
    }
}

impl Modal for DestroyVersionDialog {
    type Output = SecretManagerMsg;

    fn handle_key(&mut self, key: KeyEvent) -> Result<EventResult<Self::Output>> {
        Ok(match self.dialog.handle_key(key)? {
            EventResult::Event(ConfirmEvent::Confirmed) => VersionsMsg::Destroy {
                secret: self.secret.clone(),
                version: self.version.clone(),
            }
            .into(),
            EventResult::Event(ConfirmEvent::Cancelled) => SecretManagerMsg::DialogCancelled.into(),
            _ => EventResult::Consumed,
        })
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        self.dialog.render(frame, area, theme);
    }
}

// === Update Logic ===

pub(super) fn update(
    state: &mut SecretManager,
    msg: VersionsMsg,
) -> Result<ServiceMsg> {
    match msg {
        VersionsMsg::Load(secret) => {
            // Use cached versions if available
            if let Some(versions) = state.get_cached_versions(&secret) {
                state.push_view(VersionListScreen::new(
                    secret,
                    versions,
                    state.get_resolver(),
                ));
                return Ok(ServiceMsg::Idle);
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
            state.push_view(VersionListScreen::new(
                secret,
                versions,
                state.get_resolver(),
            ));
            Ok(ServiceMsg::Idle)
        }

        VersionsMsg::StartCreation(secret) => {
            state.display_overlay(CreateVersionDialog::new(secret, state.get_resolver()));
            Ok(ServiceMsg::Idle)
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

        VersionsMsg::Created { secret }
        | VersionsMsg::Disabled { secret }
        | VersionsMsg::Enabled { secret }
        | VersionsMsg::Destroyed { secret } => {
            state.pop_view();
            state.queue(VersionsMsg::Load(secret).into());
            Ok(ServiceMsg::Idle)
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

        VersionsMsg::ConfirmDestroy { secret, version } => {
            state.display_overlay(DestroyVersionDialog::new(
                secret,
                version,
                state.get_resolver(),
            ));
            Ok(ServiceMsg::Idle)
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

        VersionsMsg::ViewPayload { secret, version } => {
            state.queue(
                PayloadMsg::Load {
                    secret,
                    version: Some(version),
                }
                .into(),
            );
            Ok(ServiceMsg::Idle)
        }
    }
}

// === Commands ===

struct FetchVersionsCmd {
    client: SecretManagerClient,
    secret: Secret,
    tx: UnboundedSender<SecretManagerMsg>,
}

#[async_trait]
impl Command for FetchVersionsCmd {
    fn name(&self) -> String {
        format!("Loading '{}' versions", self.secret.name)
    }

    async fn execute(self: Box<Self>, _env: CommandEnv) -> Result<()> {
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

struct AddVersionCmd {
    client: SecretManagerClient,
    secret: Secret,
    payload: String,
    tx: UnboundedSender<SecretManagerMsg>,
}

#[async_trait]
impl Command for AddVersionCmd {
    fn name(&self) -> String {
        format!("Adding version to '{}'", self.secret.name)
    }

    async fn execute(self: Box<Self>, _env: CommandEnv) -> Result<()> {
        self.client
            .add_secret_version(&self.secret.name, self.payload.as_bytes())
            .await?;
        self.tx.send(
            VersionsMsg::Created {
                secret: self.secret,
            }
            .into(),
        )?;
        Ok(())
    }
}

struct DisableVersionCmd {
    client: SecretManagerClient,
    secret: Secret,
    version: SecretVersion,
    tx: UnboundedSender<SecretManagerMsg>,
}

#[async_trait]
impl Command for DisableVersionCmd {
    fn name(&self) -> String {
        format!(
            "Disabling '{}' v{}",
            self.secret.name, self.version.version_id
        )
    }

    async fn execute(self: Box<Self>, _env: CommandEnv) -> Result<()> {
        self.client
            .disable_version(&self.secret.name, &self.version.version_id)
            .await?;
        self.tx.send(
            VersionsMsg::Disabled {
                secret: self.secret,
            }
            .into(),
        )?;
        Ok(())
    }
}

struct EnableVersionCmd {
    client: SecretManagerClient,
    secret: Secret,
    version: SecretVersion,
    tx: UnboundedSender<SecretManagerMsg>,
}

#[async_trait]
impl Command for EnableVersionCmd {
    fn name(&self) -> String {
        format!(
            "Enabling '{}' v{}",
            self.secret.name, self.version.version_id
        )
    }

    async fn execute(self: Box<Self>, _env: CommandEnv) -> Result<()> {
        self.client
            .enable_version(&self.secret.name, &self.version.version_id)
            .await?;
        self.tx.send(
            VersionsMsg::Enabled {
                secret: self.secret,
            }
            .into(),
        )?;
        Ok(())
    }
}

struct DestroyVersionCmd {
    client: SecretManagerClient,
    secret: Secret,
    version: SecretVersion,
    tx: UnboundedSender<SecretManagerMsg>,
}

#[async_trait]
impl Command for DestroyVersionCmd {
    fn name(&self) -> String {
        format!(
            "Destroying '{}' v{}",
            self.secret.name, self.version.version_id
        )
    }

    async fn execute(self: Box<Self>, _env: CommandEnv) -> Result<()> {
        self.client
            .destroy_version(&self.secret.name, &self.version.version_id)
            .await?;
        self.tx.send(
            VersionsMsg::Destroyed {
                secret: self.secret,
            }
            .into(),
        )?;
        Ok(())
    }
}
