use std::collections::HashMap;
use std::fmt::Display;
use std::sync::Arc;

use async_trait::async_trait;
use crossterm::event::KeyEvent;
use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Cell, Paragraph};
use tokio::sync::mpsc::UnboundedSender;

use crate::Theme;
use crate::component::{
    ColumnDef,
    ConfirmDialogComponent,
    ConfirmEvent,
    Keybinding,
    TableComponent,
    TableEvent,
    TableRow,
    TextInputComponent,
    TextInputEvent,
};
use crate::config::{KeyResolver, SearchAction, SecretsAction};
use crate::core::command::CopyToClipboardCmd;
use crate::core::{Command, UpdateResult};
use crate::provider::gcp::secret_manager::SecretManager;
use crate::provider::gcp::secret_manager::client::SecretManagerClient;
use crate::provider::gcp::secret_manager::payload::PayloadMsg;
use crate::provider::gcp::secret_manager::service::SecretManagerMsg;
use crate::provider::gcp::secret_manager::versions::VersionsMsg;
use crate::search::Matcher;
use crate::ui::{Component, Handled, Modal, Result, Screen};

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
        SecretManagerMsg::Secret(msg)
    }
}

impl From<SecretsMsg> for Handled<SecretManagerMsg> {
    fn from(msg: SecretsMsg) -> Self {
        Handled::Event(SecretManagerMsg::Secret(msg))
    }
}

// === Screens ===

pub struct SecretListScreen {
    table: TableComponent<Secret>,
    resolver: Arc<KeyResolver>,
}

impl SecretListScreen {
    pub fn new(secrets: Vec<Secret>, resolver: Arc<KeyResolver>) -> Self {
        Self {
            table: TableComponent::new(secrets, resolver.clone()).with_title(" Secrets "),
            resolver,
        }
    }
}

impl Screen for SecretListScreen {
    type Msg = SecretManagerMsg;

    fn handle_key(&mut self, key: KeyEvent) -> Result<Handled<Self::Msg>> {
        let result = self.table.handle_key(key)?;

        if let Handled::Event(TableEvent::Activated(secret)) = result {
            return Ok(SecretsMsg::ViewPayload(secret).into());
        }
        if result.is_consumed() {
            return Ok(Handled::Consumed);
        }

        if self.resolver.matches_secrets(&key, SecretsAction::Reload) {
            return Ok(SecretsMsg::Load.into());
        }
        if self.resolver.matches_secrets(&key, SecretsAction::New) {
            return Ok(SecretsMsg::StartCreation.into());
        }
        if self.resolver.matches_secrets(&key, SecretsAction::Copy) {
            if let Some(secret) = self.table.selected_item() {
                return Ok(SecretsMsg::CopyPayload(secret.clone()).into());
            }
        }
        if self.resolver.matches_secrets(&key, SecretsAction::Delete) {
            if let Some(secret) = self.table.selected_item() {
                return Ok(SecretsMsg::ConfirmDelete(secret.clone()).into());
            }
        }
        if self.resolver.matches_secrets(&key, SecretsAction::Versions) {
            if let Some(secret) = self.table.selected_item() {
                return Ok(SecretsMsg::ViewVersions(secret.clone()).into());
            }
        }
        if self.resolver.matches_secrets(&key, SecretsAction::Labels) {
            if let Some(secret) = self.table.selected_item() {
                return Ok(SecretsMsg::ViewLabels(secret.clone()).into());
            }
        }
        if self.resolver.matches_secrets(&key, SecretsAction::Iam) {
            if let Some(secret) = self.table.selected_item() {
                return Ok(SecretsMsg::ViewIamPolicy(secret.clone()).into());
            }
        }
        if self
            .resolver
            .matches_secrets(&key, SecretsAction::Replication)
        {
            if let Some(secret) = self.table.selected_item() {
                return Ok(SecretsMsg::ViewReplicationInfo(secret.clone()).into());
            }
        }

        Ok(Handled::Ignored)
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        self.table.render(frame, area, theme);
    }

    fn keybindings(&self) -> Vec<Keybinding> {
        vec![
            Keybinding::hint(
                self.resolver.display_secrets(SecretsAction::ViewPayload),
                "Payload",
            ),
            Keybinding::hint(self.resolver.display_secrets(SecretsAction::Copy), "Copy"),
            Keybinding::hint(
                self.resolver.display_secrets(SecretsAction::Versions),
                "Versions",
            ),
            Keybinding::hint(self.resolver.display_secrets(SecretsAction::New), "New"),
            Keybinding::hint(
                self.resolver.display_secrets(SecretsAction::Delete),
                "Delete",
            ),
            Keybinding::hint(self.resolver.display_search(SearchAction::Toggle), "Search"),
            Keybinding::new(
                self.resolver.display_secrets(SecretsAction::Labels),
                "Labels",
            ),
            Keybinding::new(self.resolver.display_secrets(SecretsAction::Iam), "IAM"),
            Keybinding::new(
                self.resolver.display_secrets(SecretsAction::Replication),
                "Replication",
            ),
            Keybinding::new(
                self.resolver.display_secrets(SecretsAction::Reload),
                "Reload",
            ),
        ]
    }
}

pub struct LabelsScreen {
    secret: Secret,
    table: TableComponent<LabelEntry>,
    resolver: Arc<KeyResolver>,
}

impl LabelsScreen {
    pub fn new(secret: Secret, resolver: Arc<KeyResolver>) -> Self {
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
            table: TableComponent::new(labels, resolver.clone()).with_title(title),
            resolver,
        }
    }
}

impl Screen for LabelsScreen {
    type Msg = SecretManagerMsg;

    fn handle_key(&mut self, key: KeyEvent) -> Result<Handled<Self::Msg>> {
        let result = self.table.handle_key(key)?;
        if let Handled::Event(TableEvent::Activated(_)) = result {
            return Ok(Handled::Consumed);
        }
        if result.is_consumed() {
            return Ok(Handled::Consumed);
        }

        if self.resolver.matches_secrets(&key, SecretsAction::Reload) {
            return Ok(SecretsMsg::ViewLabels(self.secret.clone()).into());
        }

        Ok(Handled::Ignored)
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        self.table.render(frame, area, theme);
    }

    fn keybindings(&self) -> Vec<Keybinding> {
        vec![
            Keybinding::hint(self.resolver.display_search(SearchAction::Toggle), "Search"),
            Keybinding::new(
                self.resolver.display_secrets(SecretsAction::Reload),
                "Reload",
            ),
        ]
    }
}

pub struct IamPolicyScreen {
    secret: Secret,
    table: TableComponent<IamBinding>,
    resolver: Arc<KeyResolver>,
}

impl IamPolicyScreen {
    pub fn new(secret: Secret, policy: IamPolicy, resolver: Arc<KeyResolver>) -> Self {
        let title = format!(" {} - IAM Policy ", secret.name);
        Self {
            secret,
            table: TableComponent::new(policy.bindings, resolver.clone()).with_title(title),
            resolver,
        }
    }
}

impl Screen for IamPolicyScreen {
    type Msg = SecretManagerMsg;

    fn handle_key(&mut self, key: KeyEvent) -> Result<Handled<Self::Msg>> {
        let result = self.table.handle_key(key)?;
        if result.is_consumed() {
            return Ok(Handled::Consumed);
        }

        if self.resolver.matches_secrets(&key, SecretsAction::Reload) {
            return Ok(SecretsMsg::ViewIamPolicy(self.secret.clone()).into());
        }

        Ok(Handled::Ignored)
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        self.table.render(frame, area, theme);
    }

    fn keybindings(&self) -> Vec<Keybinding> {
        vec![
            Keybinding::hint(self.resolver.display_search(SearchAction::Toggle), "Search"),
            Keybinding::new(
                self.resolver.display_secrets(SecretsAction::Reload),
                "Reload",
            ),
        ]
    }
}

pub struct ReplicationScreen {
    secret: Secret,
    replication: ReplicationConfig,
    resolver: Arc<KeyResolver>,
}

impl ReplicationScreen {
    pub fn new(secret: Secret, replication: ReplicationConfig, resolver: Arc<KeyResolver>) -> Self {
        Self {
            secret,
            replication,
            resolver,
        }
    }
}

impl Screen for ReplicationScreen {
    type Msg = SecretManagerMsg;

    fn handle_key(&mut self, key: KeyEvent) -> Result<Handled<Self::Msg>> {
        if self.resolver.matches_secrets(&key, SecretsAction::Reload) {
            return Ok(SecretsMsg::ViewReplicationInfo(self.secret.clone()).into());
        }
        Ok(Handled::Ignored)
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let title = format!(" {} - Replication ", self.secret.name);

        let label_style = Style::default()
            .fg(theme.subtext0())
            .add_modifier(Modifier::BOLD);
        let value_style = Style::default().fg(theme.text());
        let location_style = Style::default().fg(theme.green());

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

        let block = Block::default()
            .title(title)
            .title_style(
                Style::default()
                    .fg(theme.mauve())
                    .add_modifier(Modifier::BOLD),
            )
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.surface1()))
            .style(Style::default().bg(theme.base()));

        let paragraph = Paragraph::new(lines).block(block);

        frame.render_widget(paragraph, area);
    }

    fn keybindings(&self) -> Vec<Keybinding> {
        vec![Keybinding::new(
            self.resolver.display_secrets(SecretsAction::Reload),
            "Reload",
        )]
    }
}

// === Wizards & Dialogs ===

enum CreateSecretWizardStep {
    Name,
    Payload,
}

pub struct CreateSecretWizard {
    step: CreateSecretWizardStep,
    name_input: TextInputComponent,
    payload_input: TextInputComponent,
}

impl CreateSecretWizard {
    pub fn new() -> Self {
        Self {
            step: CreateSecretWizardStep::Name,
            name_input: TextInputComponent::new("Secret Name").with_placeholder("my-secret"),
            payload_input: TextInputComponent::new("Initial Payload (optional)"),
        }
    }
}

impl Modal for CreateSecretWizard {
    type Msg = SecretManagerMsg;

    fn handle_key(&mut self, key: KeyEvent) -> Result<Handled<Self::Msg>> {
        Ok(match self.step {
            CreateSecretWizardStep::Name => match self.name_input.handle_key(key)? {
                Handled::Event(TextInputEvent::Submitted(name)) if !name.is_empty() => {
                    self.step = CreateSecretWizardStep::Payload;
                    Handled::Consumed
                }
                Handled::Event(TextInputEvent::Cancelled) => {
                    SecretManagerMsg::DialogCancelled.into()
                }
                _ => Handled::Consumed,
            },
            CreateSecretWizardStep::Payload => match self.payload_input.handle_key(key)? {
                Handled::Event(TextInputEvent::Submitted(payload)) => {
                    let name = self.name_input.value().to_string();
                    let payload = if payload.is_empty() {
                        None
                    } else {
                        Some(payload)
                    };
                    SecretsMsg::Create { name, payload }.into()
                }
                Handled::Event(TextInputEvent::Cancelled) => {
                    SecretManagerMsg::DialogCancelled.into()
                }
                _ => Handled::Consumed,
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
    dialog: ConfirmDialogComponent,
}

impl DeleteSecretDialog {
    pub fn new(secret: Secret, resolver: Arc<KeyResolver>) -> Self {
        let dialog = ConfirmDialogComponent::new(
            format!(
                "Are you sure you want to delete the secret \"{}\"?",
                secret.name
            ),
            resolver,
        )
        .with_title("Delete Secret")
        .with_confirm_text("Delete")
        .with_cancel_text("Cancel")
        .danger();

        Self { secret, dialog }
    }
}

impl Modal for DeleteSecretDialog {
    type Msg = SecretManagerMsg;

    fn handle_key(&mut self, key: KeyEvent) -> Result<Handled<Self::Msg>> {
        Ok(match self.dialog.handle_key(key)? {
            Handled::Event(ConfirmEvent::Confirmed) => {
                SecretsMsg::Delete(self.secret.clone()).into()
            }
            Handled::Event(ConfirmEvent::Cancelled) => SecretManagerMsg::DialogCancelled.into(),
            _ => Handled::Consumed,
        })
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
    let resolver = state.get_resolver();

    match msg {
        SecretsMsg::Load => {
            if let Some(secrets) = state.get_cached_secrets() {
                state.push_view(SecretListScreen::new(secrets, resolver));
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
            state.push_view(SecretListScreen::new(secrets, resolver));
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

        SecretsMsg::Created(_secret) => {
            state.invalidate_secrets_cache();
            state.queue(SecretsMsg::Load.into());
            Ok(UpdateResult::Idle)
        }

        SecretsMsg::ConfirmDelete(secret) => {
            state.display_overlay(DeleteSecretDialog::new(secret, resolver));
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

        SecretsMsg::Deleted(_name) => {
            state.invalidate_secrets_cache();
            state.pop_to_root();
            state.queue(SecretsMsg::Load.into());
            Ok(UpdateResult::Idle)
        }

        SecretsMsg::ViewVersions(secret) => {
            state.queue(VersionsMsg::Load(secret).into());
            Ok(UpdateResult::Idle)
        }

        SecretsMsg::ViewPayload(secret) => {
            state.queue(
                PayloadMsg::Load {
                    secret,
                    version: None,
                }
                .into(),
            );
            Ok(UpdateResult::Idle)
        }

        SecretsMsg::ViewLabels(secret) => {
            state.push_view(LabelsScreen::new(secret, resolver));
            Ok(UpdateResult::Idle)
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
            state.push_view(LabelsScreen::new(secret, resolver));
            Ok(UpdateResult::Idle)
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
            state.push_view(IamPolicyScreen::new(secret, policy, resolver));
            Ok(UpdateResult::Idle)
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
            state.push_view(ReplicationScreen::new(secret, replication, resolver));
            Ok(UpdateResult::Idle)
        }

        SecretsMsg::CopyPayload(secret) => Ok(LoadPayloadCmd {
            secret,
            client: state.get_client()?,
            tx: state.get_msg_sender(),
        }
        .into()),

        SecretsMsg::PayloadLoaded { data, secret_name } => {
            let desc = format!("payload for '{}'", secret_name);
            Ok(CopyToClipboardCmd::new(data, desc, state.get_cmd_env()).into())
        }
    }
}

// === Helper Functions ===

fn format_labels(labels: &HashMap<String, String>, query: &str) -> String {
    if labels.is_empty() {
        return "—".to_string();
    }

    // Find the best matching label if there's a query
    let best_label = if !query.is_empty() {
        let matcher = Matcher::new();
        labels
            .iter()
            .find(|(key, value)| matcher.matches(format!("{}:{}", key, value).as_str(), query))
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

    async fn execute(self: Box<Self>) -> color_eyre::Result<()> {
        let secrets = self.client.list_secrets().await?;
        self.tx.send(SecretsMsg::Loaded(secrets).into())?;
        Ok(())
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

    async fn execute(self: Box<Self>) -> color_eyre::Result<()> {
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

    async fn execute(self: Box<Self>) -> color_eyre::Result<()> {
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

    async fn execute(self: Box<Self>) -> color_eyre::Result<()> {
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

    async fn execute(self: Box<Self>) -> color_eyre::Result<()> {
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

    async fn execute(self: Box<Self>) -> color_eyre::Result<()> {
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

    async fn execute(self: Box<Self>) -> color_eyre::Result<()> {
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
