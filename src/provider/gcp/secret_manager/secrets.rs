use std::collections::HashMap;
use std::fmt::Display;

use async_trait::async_trait;
use crossterm::event::{KeyCode, KeyEvent};
use google_cloud_secretmanager_v1::model;
use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Cell, Paragraph};
use tokio::sync::mpsc::UnboundedSender;

use crate::Theme;
use crate::app::AppMessage;
use crate::commands::{Command, CopyToClipboardCmd};
use crate::provider::gcp::secret_manager::SecretManager;
use crate::provider::gcp::secret_manager::client::{ClientError, SecretManagerClient};
use crate::provider::gcp::secret_manager::payload::PayloadMsg;
use crate::provider::gcp::secret_manager::service::SecretManagerMsg;
use crate::provider::gcp::secret_manager::versions::VersionsMsg;
use crate::search::Matcher;
use crate::service::ServiceMsg;
use crate::ui::{
    ColumnDef, Component, ConfirmDialog, ConfirmEvent, EventResult, Keybinding, Modal, Result,
    Screen, Table, TableEvent, TableRow, TextInput, TextInputEvent,
};
use crate::utility::format_timestamp;
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

impl Secret {
    pub fn from_proto(secret_id: &str, proto: model::Secret) -> Self {
        let replication = if let Some(replication) = &proto.replication
            && let Some(replication) = &replication.replication
        {
            match replication {
                model::replication::Replication::UserManaged(user_managed) => {
                    let locations = user_managed
                        .replicas
                        .iter()
                        .map(|r| r.location.clone())
                        .collect();
                    ReplicationConfig::UserManaged { locations }
                }
                _ => ReplicationConfig::Automatic,
            }
        } else {
            ReplicationConfig::Automatic
        };

        Self {
            name: secret_id.to_string(),
            replication,
            created_at: proto
                .create_time
                .as_ref()
                .map_or_else(|| "Unknown".to_string(), |t| format_timestamp(t.seconds())),
            expire_time: proto.expire_time().map(|t| format_timestamp(t.seconds())),
            labels: proto.labels,
        }
    }
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
    pub fn short_display(&self) -> String {
        match self {
            Self::Automatic => "Automatic".to_string(),
            Self::UserManaged { locations } if locations.len() == 1 => locations[0].clone(),
            Self::UserManaged { locations } => {
                format!("{} regions", locations.len())
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct IamPolicy {
    pub bindings: Vec<IamBinding>,
}

#[derive(Debug, Clone)]
pub struct IamBinding {
    pub role: String,
    pub members: Vec<String>,
}

impl TableRow for IamBinding {
    fn columns() -> &'static [ColumnDef] {
        static COLUMNS: &[ColumnDef] = &[
            ColumnDef::new("Role", Constraint::Min(30)),
            ColumnDef::new("Members", Constraint::Min(40)),
        ];
        COLUMNS
    }

    fn render_cells(&self, _theme: &Theme) -> Vec<Cell<'static>> {
        // Format members as comma-separated list, truncated if too long
        let members_str = if self.members.is_empty() {
            "(none)".to_string()
        } else if self.members.len() <= 3 {
            self.members.join(", ")
        } else {
            format!(
                "{}, ... (+{} more)",
                self.members[..2].join(", "),
                self.members.len() - 2
            )
        };

        vec![Cell::from(self.role.clone()), Cell::from(members_str)]
    }

    fn matches(&self, query: &str) -> bool {
        let matcher = Matcher::new();
        matcher.matches(&self.role, query) || self.members.iter().any(|m| matcher.matches(m, query))
    }
}

#[derive(Clone, Debug)]
pub struct LabelEntry {
    pub key: String,
    pub value: String,
}

impl TableRow for LabelEntry {
    fn columns() -> &'static [ColumnDef] {
        static COLUMNS: &[ColumnDef] = &[
            ColumnDef::new("Key", Constraint::Min(20)),
            ColumnDef::new("Value", Constraint::Min(30)),
        ];
        COLUMNS
    }

    fn render_cells(&self, _theme: &Theme) -> Vec<Cell<'static>> {
        vec![Cell::from(self.key.clone()), Cell::from(self.value.clone())]
    }

    fn matches(&self, query: &str) -> bool {
        let matcher = Matcher::new();
        matcher.matches(&self.key, query) || matcher.matches(&self.value, query)
    }
}

// === Messages ===

#[derive(Debug, Clone)]
pub enum SecretsMsg {
    Load,
    Loaded(Vec<Secret>),

    StartCreation,
    Create {
        name: String,
        payload: Option<String>,
    },
    Created(Secret),

    ConfirmDelete(Secret),
    Delete(Secret),
    Deleted(String),

    ViewLabels(Secret),
    UpdateLabels {
        secret: Secret,
        labels: HashMap<String, String>,
    },
    LabelsUpdated(Secret),

    ViewIamPolicy(Secret),
    IamPolicyLoaded {
        secret: Secret,
        policy: IamPolicy,
    },

    ViewReplicationInfo(Secret),
    ReplicationInfoLoaded {
        secret: Secret,
        replication: ReplicationConfig,
    },

    ViewVersions(Secret),
    ViewPayload(Secret),

    CopyPayload(Secret),
    PayloadLoaded {
        data: String,
        secret_name: String,
    },
}

impl From<SecretsMsg> for SecretManagerMsg {
    fn from(msg: SecretsMsg) -> Self {
        Self::Secret(msg)
    }
}

impl From<SecretsMsg> for EventResult<SecretManagerMsg> {
    fn from(msg: SecretsMsg) -> Self {
        Self::Event(SecretManagerMsg::Secret(msg))
    }
}

// === Screens ===

pub struct SecretListScreen {
    table: Table<Secret>,
}

impl SecretListScreen {
    pub fn new(secrets: Vec<Secret>) -> Self {
        Self {
            table: Table::new(secrets).with_title(" Secrets "),
        }
    }
}

impl Screen for SecretListScreen {
    type Output = SecretManagerMsg;

    fn handle_key(&mut self, key: KeyEvent) -> Result<EventResult<Self::Output>> {
        let result = self.table.handle_key(key)?;

        if let EventResult::Event(TableEvent::Activated(secret)) = result {
            return Ok(SecretsMsg::ViewPayload(secret).into());
        }
        if result.is_consumed() {
            return Ok(EventResult::Consumed);
        }

        if key.code == KeyCode::Char('r') {
            return Ok(SecretsMsg::Load.into());
        }
        if key.code == KeyCode::Char('n') {
            return Ok(SecretsMsg::StartCreation.into());
        }
        if key.code == KeyCode::Char('y')
            && let Some(secret) = self.table.selected_item()
        {
            return Ok(SecretsMsg::CopyPayload(secret.clone()).into());
        }
        if matches!(key.code, KeyCode::Char('d') | KeyCode::Delete)
            && let Some(secret) = self.table.selected_item()
        {
            return Ok(SecretsMsg::ConfirmDelete(secret.clone()).into());
        }
        if key.code == KeyCode::Char('v')
            && let Some(secret) = self.table.selected_item()
        {
            return Ok(SecretsMsg::ViewVersions(secret.clone()).into());
        }
        if key.code == KeyCode::Char('l')
            && let Some(secret) = self.table.selected_item()
        {
            return Ok(SecretsMsg::ViewLabels(secret.clone()).into());
        }
        if key.code == KeyCode::Char('i')
            && let Some(secret) = self.table.selected_item()
        {
            return Ok(SecretsMsg::ViewIamPolicy(secret.clone()).into());
        }
        if key.code == KeyCode::Char('R')
            && let Some(secret) = self.table.selected_item()
        {
            return Ok(SecretsMsg::ViewReplicationInfo(secret.clone()).into());
        }

        Ok(EventResult::Ignored)
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        self.table.render(frame, area, theme);
    }

    fn keybindings(&self) -> Vec<Keybinding> {
        vec![
            Keybinding::hint("Enter", "Payload"),
            Keybinding::hint("y", "Copy"),
            Keybinding::hint("v", "Versions"),
            Keybinding::hint("n", "New"),
            Keybinding::hint("d", "Delete"),
            Keybinding::hint("/", "Search"),
            Keybinding::new("l", "Labels"),
            Keybinding::new("i", "IAM"),
            Keybinding::new("R", "Replication"),
            Keybinding::new("r", "Reload"),
        ]
    }
}

pub struct LabelsScreen {
    secret: Secret,
    table: Table<LabelEntry>,
}

impl LabelsScreen {
    pub fn new(secret: Secret) -> Self {
        let labels: Vec<LabelEntry> = secret
            .labels
            .iter()
            .map(|(k, v)| LabelEntry {
                key: k.clone(),
                value: v.clone(),
            })
            .collect();

        let title = format!(" {} - Labels ", secret.name);
        Self {
            secret,
            table: Table::new(labels).with_title(title),
        }
    }
}

impl Screen for LabelsScreen {
    type Output = SecretManagerMsg;

    fn handle_key(&mut self, key: KeyEvent) -> Result<EventResult<Self::Output>> {
        let result = self.table.handle_key(key)?;
        if let EventResult::Event(TableEvent::Activated(_)) = result {
            return Ok(EventResult::Consumed);
        }
        if result.is_consumed() {
            return Ok(EventResult::Consumed);
        }

        if key.code == KeyCode::Char('r') {
            return Ok(SecretsMsg::ViewLabels(self.secret.clone()).into());
        }

        Ok(EventResult::Ignored)
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        self.table.render(frame, area, theme);
    }

    fn keybindings(&self) -> Vec<Keybinding> {
        vec![
            Keybinding::hint("/", "Search"),
            Keybinding::new("r", "Reload"),
        ]
    }
}

pub struct IamPolicyScreen {
    secret: Secret,
    table: Table<IamBinding>,
}

impl IamPolicyScreen {
    pub fn new(secret: Secret, policy: IamPolicy) -> Self {
        let title = format!(" {} - IAM Policy ", secret.name);
        Self {
            secret,
            table: Table::new(policy.bindings).with_title(title),
        }
    }
}

impl Screen for IamPolicyScreen {
    type Output = SecretManagerMsg;

    fn handle_key(&mut self, key: KeyEvent) -> Result<EventResult<Self::Output>> {
        let result = self.table.handle_key(key)?;
        if result.is_consumed() {
            return Ok(EventResult::Consumed);
        }

        if key.code == KeyCode::Char('r') {
            return Ok(SecretsMsg::ViewIamPolicy(self.secret.clone()).into());
        }

        Ok(EventResult::Ignored)
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        self.table.render(frame, area, theme);
    }

    fn keybindings(&self) -> Vec<Keybinding> {
        vec![
            Keybinding::hint("/", "Search"),
            Keybinding::new("r", "Reload"),
        ]
    }
}

pub struct ReplicationScreen {
    secret: Secret,
    replication: ReplicationConfig,
}

impl ReplicationScreen {
    pub const fn new(
        secret: Secret,
        replication: ReplicationConfig,
    ) -> Self {
        Self {
            secret,
            replication,
        }
    }
}

impl Screen for ReplicationScreen {
    type Output = SecretManagerMsg;

    fn handle_key(&mut self, key: KeyEvent) -> Result<EventResult<Self::Output>> {
        if key.code == KeyCode::Char('r') {
            return Ok(SecretsMsg::ViewReplicationInfo(self.secret.clone()).into());
        }
        Ok(EventResult::Ignored)
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let title = format!(" {} - Replication ", self.secret.name);

        let label_style = Style::default()
            .fg(theme.text_muted())
            .add_modifier(Modifier::BOLD);
        let value_style = Style::default().fg(theme.text());
        let location_style = Style::default().fg(theme.success());

        let lines = match &self.replication {
            ReplicationConfig::Automatic => {
                vec![
                    Line::from(""),
                    Line::from(vec![
                        Span::styled("Type: ", label_style),
                        Span::styled("Automatic", value_style),
                    ]),
                    Line::from(""),
                    Line::from(Span::styled(
                        "Secret is automatically replicated across all GCP regions.",
                        Style::default().fg(theme.overlay1()),
                    )),
                ]
            }
            ReplicationConfig::UserManaged { locations } => {
                let mut lines = vec![
                    Line::from(""),
                    Line::from(vec![
                        Span::styled("Type: ", label_style),
                        Span::styled("User-Managed", value_style),
                    ]),
                    Line::from(""),
                    Line::from(Span::styled("Locations:", label_style)),
                ];

                for location in locations {
                    lines.push(Line::from(vec![
                        Span::raw("  - "),
                        Span::styled(location.clone(), location_style),
                    ]));
                }

                if locations.is_empty() {
                    lines.push(Line::from(Span::styled(
                        "  (no locations configured)",
                        Style::default().fg(theme.overlay1()),
                    )));
                }

                lines
            }
        };

        let block = theme
            .block()
            .title(title)
            .title_style(theme.title_style())
            .style(Style::default().bg(theme.bg()));

        let paragraph = Paragraph::new(lines).block(block);

        frame.render_widget(paragraph, area);
    }

    fn keybindings(&self) -> Vec<Keybinding> {
        vec![Keybinding::new("r", "Reload")]
    }
}

// === Wizards & Dialogs ===

enum CreateSecretWizardStep {
    Name,
    Payload,
}

pub struct CreateSecretWizard {
    step: CreateSecretWizardStep,
    name_input: TextInput,
    payload_input: TextInput,
}

impl CreateSecretWizard {
    pub fn new() -> Self {
        Self {
            step: CreateSecretWizardStep::Name,
            name_input: TextInput::new("Secret Name").with_placeholder("my-secret"),
            payload_input: TextInput::new("Initial Payload (optional)"),
        }
    }
}

impl Modal for CreateSecretWizard {
    type Output = SecretManagerMsg;

    fn handle_key(&mut self, key: KeyEvent) -> Result<EventResult<Self::Output>> {
        Ok(match self.step {
            CreateSecretWizardStep::Name => match self.name_input.handle_key(key)? {
                EventResult::Event(TextInputEvent::Submitted(name)) if !name.is_empty() => {
                    self.step = CreateSecretWizardStep::Payload;
                    EventResult::Consumed
                }
                EventResult::Event(TextInputEvent::Cancelled) => {
                    SecretManagerMsg::DialogCancelled.into()
                }
                _ => EventResult::Consumed,
            },
            CreateSecretWizardStep::Payload => match self.payload_input.handle_key(key)? {
                EventResult::Event(TextInputEvent::Submitted(payload)) => {
                    let name = self.name_input.value().to_string();
                    let payload = if payload.is_empty() {
                        None
                    } else {
                        Some(payload)
                    };
                    SecretsMsg::Create { name, payload }.into()
                }
                EventResult::Event(TextInputEvent::Cancelled) => {
                    SecretManagerMsg::DialogCancelled.into()
                }
                _ => EventResult::Consumed,
            },
        })
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        match self.step {
            CreateSecretWizardStep::Name => self.name_input.render(frame, area, theme),
            CreateSecretWizardStep::Payload => self.payload_input.render(frame, area, theme),
        }
    }
}

pub struct DeleteSecretDialog {
    secret: Secret,
    dialog: ConfirmDialog,
}

impl DeleteSecretDialog {
    pub fn new(secret: Secret) -> Self {
        let dialog = ConfirmDialog::new(
            format!(
                "Are you sure you want to delete the secret \"{}\"?",
                secret.name
            ),
        )
        .with_title("Delete Secret")
        .with_confirm_text("Delete")
        .with_cancel_text("Cancel")
        .danger();

        Self { secret, dialog }
    }
}

impl Modal for DeleteSecretDialog {
    type Output = SecretManagerMsg;

    fn handle_key(&mut self, key: KeyEvent) -> Result<EventResult<Self::Output>> {
        Ok(match self.dialog.handle_key(key)? {
            EventResult::Event(ConfirmEvent::Confirmed) => {
                SecretsMsg::Delete(self.secret.clone()).into()
            }
            EventResult::Event(ConfirmEvent::Cancelled) => SecretManagerMsg::DialogCancelled.into(),
            _ => EventResult::Consumed,
        })
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        self.dialog.render(frame, area, theme);
    }
}

// === Update Logic ===

// Flat message dispatcher — splitting reduces readability
#[allow(clippy::too_many_lines)]
pub(super) fn update(state: &mut SecretManager, msg: SecretsMsg) -> Result<ServiceMsg> {
    match msg {
        SecretsMsg::Load => {
            if let Some(secrets) = state.get_cached_secrets() {
                state.push_view(SecretListScreen::new(secrets));
                return Ok(ServiceMsg::Idle);
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
            Ok(ServiceMsg::Idle)
        }

        SecretsMsg::StartCreation => {
            state.display_overlay(CreateSecretWizard::new());
            Ok(ServiceMsg::Idle)
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

        SecretsMsg::Created(_secret) => {
            state.invalidate_secrets_cache();
            state.queue(SecretsMsg::Load.into());
            Ok(ServiceMsg::Idle)
        }

        SecretsMsg::ConfirmDelete(secret) => {
            state.display_overlay(DeleteSecretDialog::new(secret));
            Ok(ServiceMsg::Idle)
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

        SecretsMsg::Deleted(_name) => {
            state.invalidate_secrets_cache();
            state.pop_to_root();
            state.queue(SecretsMsg::Load.into());
            Ok(ServiceMsg::Idle)
        }

        SecretsMsg::ViewVersions(secret) => {
            state.queue(VersionsMsg::Load(secret).into());
            Ok(ServiceMsg::Idle)
        }

        SecretsMsg::ViewPayload(secret) => {
            state.queue(
                PayloadMsg::Load {
                    secret,
                    version: None,
                }
                .into(),
            );
            Ok(ServiceMsg::Idle)
        }

        SecretsMsg::ViewLabels(secret) => {
            state.push_view(LabelsScreen::new(secret));
            Ok(ServiceMsg::Idle)
        }

        SecretsMsg::UpdateLabels { secret, labels } => {
            state.display_loading_spinner("Updating labels...");

            Ok(UpdateLabelsCmd {
                secret,
                labels,
                client: state.get_client()?,
                tx: state.get_msg_sender(),
            }
            .into())
        }

        SecretsMsg::LabelsUpdated(secret) => {
            state.hide_loading_spinner();
            state.invalidate_secrets_cache();
            state.pop_view();
            state.push_view(LabelsScreen::new(secret));
            Ok(ServiceMsg::Idle)
        }

        SecretsMsg::ViewIamPolicy(secret) => {
            state.display_loading_spinner("Loading IAM policy...");

            Ok(FetchIamPolicyCmd {
                secret,
                client: state.get_client()?,
                tx: state.get_msg_sender(),
            }
            .into())
        }

        SecretsMsg::IamPolicyLoaded { secret, policy } => {
            state.hide_loading_spinner();
            state.push_view(IamPolicyScreen::new(secret, policy));
            Ok(ServiceMsg::Idle)
        }

        SecretsMsg::ViewReplicationInfo(secret) => {
            state.display_loading_spinner("Loading replication info...");

            Ok(FetchSecretMetadataCmd {
                secret,
                client: state.get_client()?,
                tx: state.get_msg_sender(),
            }
            .into())
        }

        SecretsMsg::ReplicationInfoLoaded {
            secret,
            replication,
        } => {
            state.hide_loading_spinner();
            state.push_view(ReplicationScreen::new(secret, replication));
            Ok(ServiceMsg::Idle)
        }

        SecretsMsg::CopyPayload(secret) => Ok(LoadPayloadCmd {
            secret,
            client: state.get_client()?,
            tx: state.get_msg_sender(),
        }
        .into()),

        SecretsMsg::PayloadLoaded { data, secret_name } => {
            let desc = format!("payload for '{secret_name}'");
            Ok(CopyToClipboardCmd::new(data, desc).into())
        }
    }
}

// === Helper Functions ===

fn format_labels(labels: &HashMap<String, String>, query: &str) -> String {
    if labels.is_empty() {
        return "—".to_string();
    }

    // Find the best matching label if there's a query
    let best_label = if query.is_empty() {
        labels.iter().next()
    } else {
        let matcher = Matcher::new();
        labels
            .iter()
            .find(|(key, value)| matcher.matches(format!("{key}:{value}").as_str(), query))
            .or_else(|| labels.iter().next())
    };

    if let Some((key, value)) = best_label {
        let label = if value.is_empty() {
            key.clone()
        } else {
            format!("{key}:{value}")
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

// === Commands ===

struct FetchSecretsCmd {
    client: SecretManagerClient,
    tx: UnboundedSender<SecretManagerMsg>,
}

#[async_trait]
impl Command for FetchSecretsCmd {
    fn name(&self) -> String {
        "Loading secrets".to_string()
    }

    async fn execute(self: Box<Self>, _action_tx: UnboundedSender<AppMessage>) -> Result<()> {
        match self.client.list_secrets().await {
            Ok(secrets) => {
                self.tx.send(SecretsMsg::Loaded(secrets).into())?;
                Ok(())
            }
            Err(ClientError::ApiDisabled) => {
                self.tx.send(SecretManagerMsg::ApiDisabled)?;
                Ok(())
            }
            Err(e) => Err(e.into()),
        }
    }
}

struct CreateSecretCmd {
    client: SecretManagerClient,
    name: String,
    payload: Option<String>,
    tx: UnboundedSender<SecretManagerMsg>,
}

#[async_trait]
impl Command for CreateSecretCmd {
    fn name(&self) -> String {
        format!("Creating '{}'", self.name)
    }

    async fn execute(self: Box<Self>, _action_tx: UnboundedSender<AppMessage>) -> Result<()> {
        let secret = if let Some(payload) = self.payload {
            self.client
                .create_secret_with_payload(&self.name, payload.as_bytes())
                .await?
        } else {
            self.client.create_secret(&self.name).await?
        };
        self.tx.send(SecretsMsg::Created(secret).into())?;
        Ok(())
    }
}

struct DeleteSecretCmd {
    client: SecretManagerClient,
    secret: Secret,
    tx: UnboundedSender<SecretManagerMsg>,
}

#[async_trait]
impl Command for DeleteSecretCmd {
    fn name(&self) -> String {
        format!("Deleting '{}'", self.secret.name)
    }

    async fn execute(self: Box<Self>, _action_tx: UnboundedSender<AppMessage>) -> Result<()> {
        self.client.delete_secret(&self.secret.name).await?;
        self.tx.send(SecretsMsg::Deleted(self.secret.name).into())?;
        Ok(())
    }
}

struct UpdateLabelsCmd {
    client: SecretManagerClient,
    secret: Secret,
    labels: HashMap<String, String>,
    tx: UnboundedSender<SecretManagerMsg>,
}

#[async_trait]
impl Command for UpdateLabelsCmd {
    fn name(&self) -> String {
        format!("Updating labels on '{}'", self.secret.name)
    }

    async fn execute(self: Box<Self>, _action_tx: UnboundedSender<AppMessage>) -> Result<()> {
        let secret = self
            .client
            .update_labels(&self.secret.name, self.labels)
            .await?;
        self.tx.send(SecretsMsg::LabelsUpdated(secret).into())?;
        Ok(())
    }
}

struct FetchIamPolicyCmd {
    client: SecretManagerClient,
    secret: Secret,
    tx: UnboundedSender<SecretManagerMsg>,
}

#[async_trait]
impl Command for FetchIamPolicyCmd {
    fn name(&self) -> String {
        format!("Loading IAM for '{}'", self.secret.name)
    }

    async fn execute(self: Box<Self>, _action_tx: UnboundedSender<AppMessage>) -> Result<()> {
        let policy = self.client.get_iam_policy(&self.secret.name).await?;
        self.tx.send(
            SecretsMsg::IamPolicyLoaded {
                secret: self.secret,
                policy,
            }
            .into(),
        )?;
        Ok(())
    }
}

struct FetchSecretMetadataCmd {
    client: SecretManagerClient,
    secret: Secret,
    tx: UnboundedSender<SecretManagerMsg>,
}

#[async_trait]
impl Command for FetchSecretMetadataCmd {
    fn name(&self) -> String {
        format!("Loading metadata for '{}'", self.secret.name)
    }

    async fn execute(self: Box<Self>, _action_tx: UnboundedSender<AppMessage>) -> Result<()> {
        let secret = self.client.get_secret(&self.secret.name).await?;
        let replication = secret.replication.clone();
        self.tx.send(
            SecretsMsg::ReplicationInfoLoaded {
                secret,
                replication,
            }
            .into(),
        )?;
        Ok(())
    }
}

struct LoadPayloadCmd {
    client: SecretManagerClient,
    secret: Secret,
    tx: UnboundedSender<SecretManagerMsg>,
}

#[async_trait]
impl Command for LoadPayloadCmd {
    fn name(&self) -> String {
        format!("Loading payload for '{}'", self.secret.name)
    }

    async fn execute(self: Box<Self>, _action_tx: UnboundedSender<AppMessage>) -> Result<()> {
        let payload = self.client.access_latest_version(&self.secret.name).await?;
        self.tx.send(
            SecretsMsg::PayloadLoaded {
                data: payload.data,
                secret_name: self.secret.name,
            }
            .into(),
        )?;
        Ok(())
    }
}
