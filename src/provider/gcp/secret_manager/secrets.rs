use crate::Theme;
use crate::core::{Command, UpdateResult};
use crate::provider::gcp::secret_manager::SecretManager;
use crate::provider::gcp::secret_manager::client::SecretManagerClient;
use crate::provider::gcp::secret_manager::service::{SecretManagerMsg, SecretManagerView};
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
use std::collections::HashMap;
use std::fmt::Display;
use tokio::sync::mpsc::UnboundedSender;

// === Models ===

/// A secret managed by GCP.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Secret {
    pub name: String,
    pub replication: ReplicationConfig,
    pub created_at: String,
    pub expire_time: Option<String>,
    pub labels: HashMap<String, String>,
}

impl Display for Secret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl TableRow for Secret {
    fn columns() -> &'static [ColumnDef] {
        static COLUMNS: &[ColumnDef] = &[
            ColumnDef::new("Name", Constraint::Min(20)),
            ColumnDef::new("Replication", Constraint::Length(14)),
            ColumnDef::new("Created", Constraint::Length(18)),
            ColumnDef::new("Expiration", Constraint::Length(18)),
            ColumnDef::new("Labels", Constraint::Length(23)),
        ];
        COLUMNS
    }

    fn render_cells(&self, theme: &Theme) -> Vec<Cell<'static>> {
        self.render_cells_with_query(theme, "")
    }

    fn render_cells_with_query(&self, _theme: &Theme, query: &str) -> Vec<Cell<'static>> {
        let labels_display = format_labels(&self.labels, query);
        let expiration = self.expire_time.clone().unwrap_or_else(|| "—".to_string());

        vec![
            Cell::from(self.name.clone()),
            Cell::from(self.replication.short_display()),
            Cell::from(self.created_at.clone()),
            Cell::from(expiration),
            Cell::from(labels_display),
        ]
    }

    fn matches(&self, query: &str) -> bool {
        let matcher = Matcher::new();

        // Check for label filter syntax: "key:value" or "key:"
        if let Some(colon_pos) = query.find(':') {
            let key_pattern = &query[..colon_pos];
            let value_pattern = &query[colon_pos + 1..];

            // Find labels matching the key pattern (fuzzy)
            for (key, value) in &self.labels {
                if matcher.matches(key, key_pattern) {
                    // If value pattern is empty, match any value
                    if value_pattern.is_empty() {
                        return true;
                    }
                    // Otherwise, check if value matches (fuzzy)
                    if matcher.matches(value, value_pattern) {
                        return true;
                    }
                }
            }
            return false;
        }

        // Regular fuzzy search: match name or any label key/value
        if matcher.matches(&self.name, query) {
            return true;
        }

        // Check label keys and values
        for (key, value) in &self.labels {
            if matcher.matches(key, query) || matcher.matches(value, query) {
                return true;
            }
        }

        false
    }
}

/// Replication configuration for a secret.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplicationConfig {
    /// Automatic replication managed by GCP.
    Automatic,
    /// User-managed replication with specific locations.
    UserManaged { locations: Vec<String> },
}

impl ReplicationConfig {
    /// Short display string for table column.
    pub fn short_display(&self) -> String {
        match self {
            ReplicationConfig::Automatic => "Automatic".to_string(),
            ReplicationConfig::UserManaged { locations } if locations.len() == 1 => {
                locations[0].clone()
            }
            ReplicationConfig::UserManaged { locations } => {
                format!("{} regions", locations.len())
            }
        }
    }
}

// === Messages ===

#[derive(Debug, Clone)]
pub enum SecretsMsg {
    /// Load secrets list
    Load,
    /// List of secrets successfully loaded
    Loaded(Vec<Secret>),

    /// Show the wizard to create a new secret
    StartCreation,
    /// Create a new secret with the given name and optional payload
    Create {
        name: String,
        payload: Option<String>,
    },

    /// Show delete secret confirmation dialog
    ConfirmDelete(Secret),
    /// Delete a secret
    Delete(Secret),

    /// Show labels for a secret
    ViewLabels(Secret),
    /// Update labels for a secret
    UpdateLabels {
        secret: Secret,
        labels: HashMap<String, String>,
    },
    /// Show IAM policy for a secret
    ViewIamPolicy(Secret),
    /// Show replication info for a secret
    ViewReplicationInfo(Secret),

    /// Navigate to secret versions view
    ViewVersions(Secret),
    /// Navigate to secret payload view
    ViewPayload(Secret),
}

impl From<SecretsMsg> for SecretManagerMsg {
    fn from(msg: SecretsMsg) -> Self {
        SecretManagerMsg::Secret(msg).into()
    }
}

impl From<SecretsMsg> for KeyResult<SecretManagerMsg> {
    fn from(msg: SecretsMsg) -> Self {
        KeyResult::Event(SecretManagerMsg::Secret(msg).into())
    }
}

// === Screens ===

pub struct SecretListScreen {
    table: TableView<Secret>,
}

impl SecretListScreen {
    pub fn new(secrets: Vec<Secret>) -> Self {
        Self {
            table: TableView::new(secrets).with_title(" Secrets "),
        }
    }
}

impl View for SecretListScreen {
    type Event = SecretManagerMsg;

    fn handle_key(&mut self, key: KeyEvent) -> KeyResult<Self::Event> {
        let result = self.table.handle_key(key);

        if let KeyResult::Event(TableEvent::Activated(secret)) = result {
            return SecretsMsg::ViewPayload(secret).into();
        }
        if result.is_consumed() {
            return KeyResult::Consumed;
        }

        match key.code {
            KeyCode::Char('r') => KeyResult::Event(SecretsMsg::Load.into()),
            KeyCode::Char('n') | KeyCode::Char('c') => SecretsMsg::StartCreation.into(),
            KeyCode::Char('d') | KeyCode::Delete => match self.table.selected_item() {
                None => KeyResult::Ignored,
                Some(secret) => SecretsMsg::ConfirmDelete(secret.clone()).into(),
            },
            KeyCode::Char('v') => match self.table.selected_item() {
                None => KeyResult::Ignored,
                Some(secret) => SecretsMsg::ViewVersions(secret.clone()).into(),
            },
            KeyCode::Char('l') => match self.table.selected_item() {
                None => KeyResult::Ignored,
                Some(secret) => SecretsMsg::ViewLabels(secret.clone()).into(),
            },
            KeyCode::Char('i') => match self.table.selected_item() {
                None => KeyResult::Ignored,
                Some(secret) => SecretsMsg::ViewIamPolicy(secret.clone()).into(),
            },
            KeyCode::Char('R') => match self.table.selected_item() {
                None => KeyResult::Ignored,
                Some(secret) => SecretsMsg::ViewReplicationInfo(secret.clone()).into(),
            },
            _ => KeyResult::Ignored,
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        self.table.render(frame, area, theme);
    }
}

// === Wizards & Dialogs ===

enum CreateSecretWizardStep {
    Name,
    Payload,
}

struct CreateSecretWizard {
    step: CreateSecretWizardStep,
    name_input: TextInputView,
    payload_input: TextInputView,
}
impl CreateSecretWizard {
    pub fn new() -> Self {
        Self {
            step: CreateSecretWizardStep::Name,
            name_input: TextInputView::new("Secret Name").with_placeholder("my-secret"),
            payload_input: TextInputView::new("Initial Payload (optional)"),
        }
    }
}

impl View for CreateSecretWizard {
    type Event = SecretManagerMsg;

    fn handle_key(&mut self, key: KeyEvent) -> KeyResult<Self::Event> {
        match self.step {
            CreateSecretWizardStep::Name => match self.name_input.handle_key(key) {
                KeyResult::Event(TextInputEvent::Submitted(name)) if !name.is_empty() => {
                    self.step = CreateSecretWizardStep::Payload;
                    KeyResult::Consumed
                }
                KeyResult::Event(TextInputEvent::Cancelled) => {
                    SecretManagerMsg::DialogCancelled.into()
                }
                _ => KeyResult::Consumed,
            },
            CreateSecretWizardStep::Payload => match self.payload_input.handle_key(key) {
                KeyResult::Event(TextInputEvent::Submitted(payload)) => {
                    let name = self.name_input.value().to_string();
                    let payload = if payload.is_empty() {
                        None
                    } else {
                        Some(payload)
                    };
                    SecretsMsg::Create { name, payload }.into()
                }
                KeyResult::Event(TextInputEvent::Cancelled) => {
                    SecretManagerMsg::DialogCancelled.into()
                }
                _ => KeyResult::Consumed,
            },
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        match self.step {
            CreateSecretWizardStep::Name => self.name_input.render(frame, area, theme),
            CreateSecretWizardStep::Payload => self.payload_input.render(frame, area, theme),
        }
    }
}

struct DeleteSecretDialog {
    secret: Secret,
    dialog: ConfirmDialog,
}

impl DeleteSecretDialog {
    pub fn new(secret: Secret) -> Self {
        let dialog = ConfirmDialog::new(format!(
            "Are you sure you want to delete the secret \"{}\"?",
            secret.name
        ))
        .with_title("Delete Secret")
        .with_confirm_text("Delete")
        .with_cancel_text("Cancel")
        .danger();

        Self { secret, dialog }
    }
}

impl View for DeleteSecretDialog {
    type Event = SecretManagerMsg;

    fn handle_key(&mut self, key: KeyEvent) -> KeyResult<Self::Event> {
        match self.dialog.handle_key(key) {
            KeyResult::Event(ConfirmEvent::Confirmed) => {
                SecretsMsg::Delete(self.secret.clone()).into()
            }
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
    msg: SecretsMsg,
) -> color_eyre::Result<UpdateResult> {
    match msg {
        SecretsMsg::Load => {
            if let Some(secrets) = state.get_cached_secrets() {
                state.push_view(SecretListScreen::new(secrets));
                return Ok(UpdateResult::Idle);
            }

            state.display_loading_spinner("Loading secrets...");

            Ok(FetchSecretsCmd {
                client: state.get_client()?,
                tx: state.get_msg_sender(),
            }
            .into())
        }

        SecretsMsg::Loaded(secrets) => {
            state.hide_loading_spinner();
            state.cache_secrets(&secrets);
            state.push_view(SecretListScreen::new(secrets));
            Ok(UpdateResult::Idle)
        }

        SecretsMsg::StartCreation => {
            state.display_overlay(CreateSecretWizard::new());
            Ok(UpdateResult::Idle)
        }

        SecretsMsg::Create { name, payload } => {
            state.display_loading_spinner("Creating secret...");
            state.close_overlay();

            Ok(CreateSecretCmd {
                name,
                payload,
                client: state.get_client()?,
                tx: state.get_msg_sender(),
            }
            .into())
        }

        SecretsMsg::ConfirmDelete(secret) => {
            state.display_overlay(DeleteSecretDialog::new(secret));
            Ok(UpdateResult::Idle)
        }

        SecretsMsg::Delete(secret) => {
            state.display_loading_spinner("Deleting secret...");
            state.close_overlay();

            Ok(DeleteSecretCmd {
                secret,
                client: state.get_client()?,
                tx: state.get_msg_sender(),
            }
            .into())
        }

        SecretsMsg::ViewVersions(secret) => {
            // TODO: Navigate to versions view
            Ok(UpdateResult::Idle)
        }

        SecretsMsg::ViewPayload(secret) => {
            // TODO: Payload open payload
            Ok(UpdateResult::Idle)
        }

        SecretsMsg::ViewLabels(labels) => {
            // TODO: View labels
            Ok(UpdateResult::Idle)
        }

        SecretsMsg::UpdateLabels { .. } => {
            // TODO: Update labels
            Ok(UpdateResult::Idle)
        }
        SecretsMsg::ViewIamPolicy { .. } => {
            //  TODO: View IAM policy
            Ok(UpdateResult::Idle)
        }

        SecretsMsg::ViewReplicationInfo { .. } => {
            // TODO: View replication info
            Ok(UpdateResult::Idle)
        }
    }
}

/// Format labels for display in the table.
/// When a query is provided, shows the best matching label first.
fn format_labels(labels: &HashMap<String, String>, query: &str) -> String {
    if labels.is_empty() {
        return "—".to_string();
    }

    // Find the best matching label if there's a query
    let best_label = if !query.is_empty() {
        let matcher = Matcher::new();
        labels
            .iter()
            .filter(|(key, value)| matcher.matches(format!("{}:{}", key, value).as_str(), query))
            .next()
            .or_else(|| labels.iter().next())
    } else {
        labels.iter().next()
    };

    if let Some((key, value)) = best_label {
        let label = if value.is_empty() {
            key.clone()
        } else {
            format!("{}:{}", key, value)
        };

        // Truncate if too long
        if label.len() > 20 {
            let suffix = if labels.len() > 1 {
                format!("… +{}", labels.len() - 1)
            } else {
                "…".to_string()
            };
            format!("{}{}", &label[..17], suffix)
        } else if labels.len() > 1 {
            format!("{} +{}", label, labels.len() - 1)
        } else {
            label
        }
    } else {
        "—".to_string()
    }
}


pub struct SecretListView {
    table: TableView<Secret>,
}

impl SecretListView {
    pub fn new(secrets: Vec<Secret>) -> Self {
        Self {
            table: TableView::new(secrets).with_title(" Secrets "),
        }
    }

    pub fn selected_secret(&self) -> Option<&Secret> {
        self.table.selected_item()
    }
}

impl SecretManagerView for SecretListView {
    fn breadcrumbs(&self) -> Vec<String> {
        vec!["Secrets".to_string()]
    }

    fn reload(&self) -> SecretManagerMsg {
        SecretManagerMsg::LoadSecrets
    }
}

impl View for SecretListView {
    type Event = SecretsMsg;

    fn handle_key(&mut self, key: KeyEvent) -> KeyResult<Self::Event> {
        // Delegate to table first (handles search mode, navigation, etc.)
        let result = self.table.handle_key(key);
        if let KeyResult::Event(TableEvent::Activated(secret)) = result {
            return SecretManagerMsg::LoadPayload(secret, None).into();
        }
        if result.is_consumed() {
            return KeyResult::Consumed;
        }

        // Handle local shortcuts only if table didn't consume the key
        match key.code {
            KeyCode::Char('r') => SecretManagerMsg::ReloadData.into(),
            KeyCode::Char('v') => match self.selected_secret() {
                None => KeyResult::Ignored,
                Some(secret) => SecretManagerMsg::LoadVersions(secret.clone()).into(),
            },
            // Create new secret
            KeyCode::Char('n') | KeyCode::Char('c') => {
                SecretManagerMsg::ShowCreateSecretDialog.into()
            }
            // Delete selected secret
            KeyCode::Char('d') | KeyCode::Delete => match self.selected_secret() {
                None => KeyResult::Ignored,
                Some(secret) => SecretManagerMsg::ShowDeleteSecretDialog(secret.clone()).into(),
            },
            // View/edit labels
            KeyCode::Char('l') => match self.selected_secret() {
                None => KeyResult::Ignored,
                Some(secret) => SecretManagerMsg::ShowLabels(secret.clone()).into(),
            },
            // View IAM policy
            KeyCode::Char('i') => match self.selected_secret() {
                None => KeyResult::Ignored,
                Some(secret) => SecretManagerMsg::ShowIamPolicy(secret.clone()).into(),
            },
            // View replication info
            KeyCode::Char('R') => match self.selected_secret() {
                None => KeyResult::Ignored,
                Some(secret) => SecretManagerMsg::ShowReplicationInfo(secret.clone()).into(),
            },
            _ => KeyResult::Ignored,
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        self.table.render(frame, area, theme);
    }
}

// === Dialogs ===

pub struct CreateSecretNameDialog {
    input: TextInputView,
}

impl CreateSecretNameDialog {
    pub fn new() -> Self {
        Self {
            input: TextInputView::new("Secret Name").with_placeholder("my-secret"),
        }
    }
}

impl View for CreateSecretNameDialog {
    type Event = SecretManagerMsg;

    fn handle_key(&mut self, key: KeyEvent) -> KeyResult<Self::Event> {
        match self.input.handle_key(key) {
            KeyResult::Event(TextInputEvent::Submitted(name)) if !name.is_empty() => {
                SecretManagerMsg::CreateSecretStep2 { name }.into()
            }
            KeyResult::Event(_) => SecretManagerMsg::DialogCancelled.into(),
            _ => KeyResult::Consumed,
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        self.input.render(frame, area, theme);
    }
}

pub struct CreateSecretPayloadDialog {
    name: String,
    input: TextInputView,
}

impl CreateSecretPayloadDialog {
    pub fn new(name: String) -> Self {
        Self {
            name,
            input: TextInputView::new("Initial Payload (optional)"),
        }
    }
}

impl View for CreateSecretPayloadDialog {
    type Event = SecretManagerMsg;

    fn handle_key(&mut self, key: KeyEvent) -> KeyResult<Self::Event> {
        match self.input.handle_key(key) {
            KeyResult::Event(TextInputEvent::Submitted(payload)) => {
                let payload = if payload.is_empty() {
                    None
                } else {
                    Some(payload)
                };
                SecretManagerMsg::CreateSecret {
                    name: self.name.clone(),
                    payload,
                }
                .into()
            }
            KeyResult::Event(TextInputEvent::Cancelled) => SecretManagerMsg::DialogCancelled.into(),
            _ => KeyResult::Consumed,
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        self.input.render(frame, area, theme);
    }
}

// === Commands ===

/// Fetch the list of secrets.
struct FetchSecretsCmd {
    client: SecretManagerClient,
    tx: UnboundedSender<SecretManagerMsg>,
}

#[async_trait]
impl Command for FetchSecretsCmd {
    fn name(&self) -> &'static str {
        "Loading secrets"
    }

    async fn execute(self: Box<Self>) -> color_eyre::Result<()> {
        let secrets = self.client.list_secrets().await?;
        self.tx.send(SecretsMsg::Loaded(secrets).into())?;
        Ok(())
    }
}

/// Create a new secret.
struct CreateSecretCmd {
    client: SecretManagerClient,
    name: String,
    payload: Option<String>,
    tx: UnboundedSender<SecretManagerMsg>,
}

#[async_trait]
impl Command for CreateSecretCmd {
    fn name(&self) -> &'static str {
        "Creating secret"
    }

    async fn execute(self: Box<Self>) -> color_eyre::Result<()> {
        let secret = if let Some(payload) = self.payload {
            self.client
                .create_secret_with_payload(&self.name, payload.as_bytes())
                .await?
        } else {
            self.client.create_secret(&self.name).await?
        };
        self.tx.send(SecretManagerMsg::SecretCreated(secret))?;
        Ok(())
    }
}

/// Delete a secret.
struct DeleteSecretCmd {
    client: SecretManagerClient,
    secret: Secret,
    tx: UnboundedSender<SecretManagerMsg>,
}

#[async_trait]
impl Command for DeleteSecretCmd {
    fn name(&self) -> &'static str {
        "Deleting secret"
    }

    async fn execute(self: Box<Self>) -> color_eyre::Result<()> {
        self.client.delete_secret(&self.secret.name).await?;
        self.tx
            .send(SecretManagerMsg::SecretDeleted(self.secret.name))?;
        Ok(())
    }
}
