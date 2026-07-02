use std::path::PathBuf;
use std::time::Duration;

use color_eyre::eyre::{Result, eyre};
use crossterm::event::{KeyCode, KeyEvent};
use google_cloud_auth::credentials::{Credentials, mds, service_account};
use http::Extensions;
use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Cell, Clear, List, ListItem, ListState, Paragraph};
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, warn};

use crate::Theme;
use crate::config::config_dir;
use crate::provider::Provider;
use crate::provider::gcp::discover_gcloud_configs;
use crate::search::Matcher;
use crate::ui::{
    ColumnDef,
    Component,
    EventResult,
    Keybinding,
    Screen,
    Table,
    TableEvent,
    TableRow,
    TextInput,
    TextInputEvent,
};

const CONTEXTS_FILE: &str = "contexts.json";

/// Cloud context containing connection and authentication details.
///
/// Each variant holds provider-specific configuration needed to
/// authenticate and interact with that cloud provider's APIs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CloudContext {
    Gcp(GcpContext),
}

/// GCP connection context enriched with lazycloud-specific configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GcpContext {
    pub display_name: String,
    pub project_id: String,
    pub account: String,
    pub region: Option<String>,
    pub zone: Option<String>,
    pub auth: AuthMethod,
}

/// How a [`GcpContext`] obtains its Google credentials.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthMethod {
    /// Application Default Credentials: `GOOGLE_APPLICATION_CREDENTIALS`, then the
    /// gcloud ADC file, then the metadata server — resolved in that order.
    ApplicationDefault,
    /// A service account JSON key file on disk.
    ServiceAccountKey { path: PathBuf },
    /// The GCE / GKE / Cloud Run metadata server, explicitly (skips ADC file lookup).
    MetadataServer,
}

impl AuthMethod {
    /// A concise label for display in tables.
    pub const fn label(&self) -> &'static str {
        match self {
            Self::ApplicationDefault => "ADC",
            Self::ServiceAccountKey { .. } => "SA key",
            Self::MetadataServer => "Metadata",
        }
    }
}

/// Failure modes when building or validating credentials.
///
/// The messages are user-facing: they are surfaced directly in the error dialog.
#[derive(Debug, thiserror::Error)]
pub enum CredentialError {
    #[error("Failed to build credentials: {0}")]
    Build(String),

    #[error("Failed to read service account key '{path}': {source}")]
    KeyFile {
        path: String,
        source: std::io::Error,
    },

    #[error("Service account key '{path}' is not valid JSON: {source}")]
    KeyParse {
        path: String,
        source: serde_json::Error,
    },

    #[error(
        "Could not reach a Google credential source within {}s.\n\n\
         If you're running locally, log in with:\n    gcloud auth application-default login",
        .0.as_secs()
    )]
    Timeout(Duration),

    #[error(
        "Could not obtain Google credentials.\n\n\
         Details: {0}\n\n\
         If you're running locally, log in with:\n    gcloud auth application-default login"
    )]
    Unusable(String),
}

impl GcpContext {
    /// How long to wait for the credential preflight before giving up.
    ///
    /// Kept short so a missing/unreachable credential source (e.g. the metadata
    /// server when not on GCP) fails fast instead of hanging on retries.
    const PREFLIGHT_TIMEOUT: Duration = Duration::from_secs(5);

    /// Build the credentials for this context and eagerly validate them.
    ///
    /// Validation fetches auth headers once (with a short timeout), so a broken
    /// or missing credential source produces an immediate, actionable error
    /// rather than an indefinite hang on the first RPC. The fetched token is
    /// cached, so this does not cost an extra round-trip in the happy path.
    pub async fn create_credentials(&self) -> Result<Credentials, CredentialError> {
        let credentials = self.build_credentials()?;
        self.validate(&credentials).await?;
        Ok(credentials)
    }

    /// Construct the credentials for the configured [`AuthMethod`] without any
    /// network validation.
    fn build_credentials(&self) -> Result<Credentials, CredentialError> {
        let build = |e: google_cloud_auth::build_errors::Error| CredentialError::Build(e.to_string());
        match &self.auth {
            AuthMethod::ApplicationDefault => {
                google_cloud_auth::credentials::Builder::default()
                    .build()
                    .map_err(build)
            }
            AuthMethod::ServiceAccountKey { path } => {
                let display = path.display().to_string();
                let contents =
                    std::fs::read_to_string(path).map_err(|source| CredentialError::KeyFile {
                        path: display.clone(),
                        source,
                    })?;
                let json = serde_json::from_str(&contents).map_err(|source| {
                    CredentialError::KeyParse {
                        path: display,
                        source,
                    }
                })?;
                service_account::Builder::new(json).build().map_err(build)
            }
            AuthMethod::MetadataServer => mds::Builder::default().build().map_err(build),
        }
    }

    /// Eagerly fetch auth headers to confirm the credentials actually work,
    /// bounded by [`Self::PREFLIGHT_TIMEOUT`].
    async fn validate(&self, credentials: &Credentials) -> Result<(), CredentialError> {
        debug!(auth = ?self.auth, "Validating credentials");
        match tokio::time::timeout(
            Self::PREFLIGHT_TIMEOUT,
            credentials.headers(Extensions::new()),
        )
        .await
        {
            Ok(Ok(_)) => {
                info!("Credentials validated");
                Ok(())
            }
            Ok(Err(e)) => {
                warn!(error = %e, "Credential validation failed");
                Err(CredentialError::Unusable(concise_error(&e.to_string())))
            }
            Err(_) => {
                warn!(timeout_secs = Self::PREFLIGHT_TIMEOUT.as_secs(), "Credential validation timed out");
                Err(CredentialError::Timeout(Self::PREFLIGHT_TIMEOUT))
            }
        }
    }
}

/// Keep only the first, meaningful sentence of a credential error.
///
/// The `google-cloud-auth` errors chain awkward advisory sentences
/// (e.g. "...transient errors. Subsequent calls with this credential might
/// succeed. but future attempts may succeed") that read as gibberish. The
/// leading sentence carries the actual cause.
fn concise_error(message: &str) -> String {
    message
        .split_once(". ")
        .map_or(message, |(first, _)| first)
        .trim()
        .to_string()
}

impl CloudContext {
    /// Get the provider for this context.
    pub const fn provider(&self) -> Provider {
        match self {
            Self::Gcp(_) => Provider::Gcp,
        }
    }

    /// Get a short display name for this context.
    pub fn name(&self) -> &str {
        match self {
            Self::Gcp(ctx) => &ctx.display_name,
        }
    }

    /// The credential method this context authenticates with.
    pub const fn auth(&self) -> &AuthMethod {
        match self {
            Self::Gcp(ctx) => &ctx.auth,
        }
    }

    /// Replace the credential method for this context.
    pub fn set_auth(&mut self, auth: AuthMethod) {
        match self {
            Self::Gcp(ctx) => ctx.auth = auth,
        }
    }

    /// Copy the credential method from another context of the same provider.
    ///
    /// Used when a context is updated from gcloud: gcloud does not know about
    /// the lazycloud-specific auth method, so it must be preserved.
    fn inherit_auth(&mut self, other: &Self) {
        match (self, other) {
            (Self::Gcp(a), Self::Gcp(b)) => a.auth = b.auth.clone(),
        }
    }

    /// Whether two contexts differ in any gcloud-sourced field (everything
    /// except the lazycloud-specific [`AuthMethod`]).
    fn differs_from_gcloud(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Gcp(a), Self::Gcp(b)) => {
                a.project_id != b.project_id
                    || a.account != b.account
                    || a.region != b.region
                    || a.zone != b.zone
            }
        }
    }
}

impl std::fmt::Display for CloudContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// How a discovered context relates to what lazycloud has stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncKind {
    /// Present in gcloud but not stored yet.
    New,
    /// Stored and present in gcloud, but gcloud-sourced fields differ.
    Modified,
    /// Stored but no longer present in gcloud.
    Removed,
}

impl SyncKind {
    pub const fn title(self) -> &'static str {
        match self {
            Self::New => "New",
            Self::Modified => "Modified",
            Self::Removed => "Removed",
        }
    }
}

/// A single difference between the stored contexts and gcloud's configurations.
#[derive(Debug, Clone)]
pub struct SyncEntry {
    pub kind: SyncKind,
    /// For `New`/`Modified`: the incoming gcloud-derived context.
    /// For `Removed`: the stored context that gcloud no longer has.
    pub incoming: CloudContext,
    /// For `Modified` only: the currently stored context (to show the diff and
    /// preserve its auth method on update).
    pub existing: Option<CloudContext>,
}

/// A user's decision to apply the action implied by a [`SyncEntry`]'s kind
/// (add / update / remove). Only applied entries are turned into decisions.
#[derive(Debug, Clone)]
pub struct SyncDecision {
    pub kind: SyncKind,
    pub name: String,
    pub incoming: CloudContext,
}

/// Counts of what an [`ContextManager::apply_sync`] actually changed.
#[derive(Debug, Default, Clone, Copy)]
pub struct SyncSummary {
    pub added: usize,
    pub updated: usize,
    pub removed: usize,
}

impl SyncSummary {
    pub const fn is_empty(self) -> bool {
        self.added == 0 && self.updated == 0 && self.removed == 0
    }
}

pub struct ContextManager {
    contexts: Vec<CloudContext>,
}

impl ContextManager {
    /// Create a new manager and load saved contexts.
    /// If no saved contexts exist, auto-discovers from the different providers.
    pub fn new() -> Self {
        let mut contexts = Self::load_contexts();

        if contexts.is_empty() {
            debug!("No saved contexts found, discovering from gcloud");
            contexts = Self::discover_all();
            if !contexts.is_empty() {
                let manager = Self { contexts };
                if let Err(err) = manager.save_contexts() {
                    error!(%err, "Failed to save discovered contexts");
                }
                return manager;
            }
        }

        Self { contexts }
    }

    /// Compute the difference between the stored contexts and gcloud's current
    /// configurations: new, modified, and removed entries (in that order).
    ///
    /// Auth methods are lazycloud-specific and never participate in the diff.
    pub fn diff_gcloud(&self) -> Vec<SyncEntry> {
        let discovered = Self::discover_all();
        let mut entries = Vec::new();

        // New + Modified: walk gcloud's configs against what we have stored.
        for incoming in &discovered {
            match self.contexts.iter().find(|c| c.name() == incoming.name()) {
                None => entries.push(SyncEntry {
                    kind: SyncKind::New,
                    incoming: incoming.clone(),
                    existing: None,
                }),
                Some(existing) if existing.differs_from_gcloud(incoming) => entries.push(SyncEntry {
                    kind: SyncKind::Modified,
                    incoming: incoming.clone(),
                    existing: Some(existing.clone()),
                }),
                Some(_) => {}
            }
        }

        // Removed: stored contexts gcloud no longer knows about.
        for existing in &self.contexts {
            if !discovered.iter().any(|c| c.name() == existing.name()) {
                entries.push(SyncEntry {
                    kind: SyncKind::Removed,
                    incoming: existing.clone(),
                    existing: None,
                });
            }
        }

        entries
    }

    /// Find a context by name (case-insensitive).
    pub fn find_by_name(&self, name: &str) -> Result<CloudContext> {
        self.contexts
            .iter()
            .find(|c| c.name().eq_ignore_ascii_case(name))
            .cloned()
            .ok_or_else(|| {
                let available: Vec<_> = self.contexts.iter().map(CloudContext::name).collect();
                error!(name, ?available, "Context lookup failed");
                eyre!(
                    "Context '{}' not found. Available: {}",
                    name,
                    available.join(", ")
                )
            })
    }

    /// Get all saved contexts.
    pub fn get_all(&self) -> Vec<CloudContext> {
        self.contexts.clone()
    }

    /// Get contexts filtered by provider.
    pub fn get_by_provider(&self, provider: Provider) -> Vec<CloudContext> {
        self.contexts
            .iter()
            .filter(|c| c.provider() == provider)
            .cloned()
            .collect()
    }

    /// Apply the user's sync decisions and persist. Modified contexts keep
    /// their existing auth method (gcloud does not track it).
    pub fn apply_sync(&mut self, decisions: Vec<SyncDecision>) -> Result<SyncSummary> {
        let mut summary = SyncSummary::default();

        for decision in decisions {
            match decision.kind {
                SyncKind::New => {
                    self.contexts.push(decision.incoming);
                    summary.added += 1;
                }
                SyncKind::Modified => {
                    if let Some(existing) = self
                        .contexts
                        .iter_mut()
                        .find(|c| c.name() == decision.name)
                    {
                        let mut updated = decision.incoming;
                        updated.inherit_auth(existing);
                        *existing = updated;
                        summary.updated += 1;
                    }
                }
                SyncKind::Removed => {
                    if let Some(pos) = self.contexts.iter().position(|c| c.name() == decision.name) {
                        self.contexts.remove(pos);
                        summary.removed += 1;
                    }
                }
            }
        }

        if !summary.is_empty() {
            self.save_contexts()?;
        }
        Ok(summary)
    }

    /// Change the auth method for the named context and persist.
    pub fn set_auth(&mut self, name: &str, auth: AuthMethod) -> Result<()> {
        let context = self
            .contexts
            .iter_mut()
            .find(|c| c.name() == name)
            .ok_or_else(|| eyre!("Context '{name}' not found"))?;
        context.set_auth(auth);
        self.save_contexts()
    }

    fn load_contexts() -> Vec<CloudContext> {
        if let Some(config_dir) = config_dir() {
            let path = config_dir.join(CONTEXTS_FILE);
            match std::fs::read_to_string(&path) {
                Ok(data) => match serde_json::from_str::<Vec<CloudContext>>(&data) {
                    Ok(contexts) => {
                        info!(path = %path.display(), count = contexts.len(), "Loaded contexts");
                        return contexts;
                    }
                    Err(err) => {
                        error!(path = %path.display(), %err, "Failed to parse contexts file");
                    }
                },
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                    debug!(path = %path.display(), "Contexts file not found");
                }
                Err(err) => error!(path = %path.display(), %err, "Failed to read contexts file"),
            }
        }
        Vec::new()
    }

    fn save_contexts(&self) -> Result<()> {
        if let Some(config_dir) = config_dir() {
            std::fs::create_dir_all(&config_dir)?;
            let path = config_dir.join(CONTEXTS_FILE);
            let data = serde_json::to_string_pretty(&self.contexts)?;
            std::fs::write(&path, data)?;
            info!(path = %path.display(), count = self.contexts.len(), "Saved contexts");
        }
        Ok(())
    }

    fn discover_all() -> Vec<CloudContext> {
        discover_gcloud_configs()
            .into_iter()
            .map(|config| {
                CloudContext::Gcp(GcpContext {
                    display_name: config.name,
                    project_id: config.core.project,
                    account: config.core.account,
                    region: config.compute.region,
                    zone: config.compute.zone,
                    auth: AuthMethod::ApplicationDefault,
                })
            })
            .collect()
    }
}

// === UI ===

impl TableRow for CloudContext {
    fn columns() -> &'static [ColumnDef] {
        static COLUMNS: &[ColumnDef] = &[
            ColumnDef::new("Name", Constraint::Min(20)),
            ColumnDef::new("Provider", Constraint::Length(10)),
            ColumnDef::new("Project", Constraint::Min(20)),
            ColumnDef::new("Account", Constraint::Min(24)),
            ColumnDef::new("Region", Constraint::Length(20)),
            ColumnDef::new("Auth", Constraint::Length(10)),
        ];
        COLUMNS
    }

    fn render_cells(&self, _theme: &Theme) -> Vec<Cell<'static>> {
        match self {
            Self::Gcp(ctx) => vec![
                Cell::from(ctx.display_name.clone()),
                Cell::from("GCP"),
                Cell::from(ctx.project_id.clone()),
                Cell::from(ctx.account.clone()),
                Cell::from(
                    ctx.region
                        .clone()
                        .or_else(|| ctx.zone.clone())
                        .unwrap_or_else(|| "—".to_string()),
                ),
                Cell::from(ctx.auth.label()),
            ],
        }
    }

    fn matches(&self, query: &str) -> bool {
        let matcher = Matcher::new();
        match self {
            Self::Gcp(ctx) => {
                matcher.matches(&ctx.display_name, query)
                    || matcher.matches(&ctx.project_id, query)
                    || matcher.matches(&ctx.account, query)
                    || ctx
                        .region
                        .as_ref()
                        .is_some_and(|r| matcher.matches(r, query))
                    || ctx.zone.as_ref().is_some_and(|z| matcher.matches(z, query))
            }
        }
    }
}

pub enum ContextSelectorEvent {
    Selected(CloudContext),
    /// Synchronize the stored contexts with gcloud.
    Refresh,
    /// Edit the auth method of the given context.
    EditAuth(CloudContext),
}

pub struct ContextSelectorView {
    table: Table<CloudContext>,
}

impl ContextSelectorView {
    /// Create with provided contexts.
    pub fn new(contexts: Vec<CloudContext>) -> Self {
        Self {
            table: Table::new(contexts).with_title(" Contexts "),
        }
    }
}

impl Screen for ContextSelectorView {
    type Output = ContextSelectorEvent;

    fn handle_key(&mut self, key: KeyEvent) -> Result<EventResult<Self::Output>> {
        // Delegate to the table first so its own keys (navigation, search) win;
        // only unhandled keys become selector-level actions. This also avoids
        // hijacking characters typed into the search field.
        let result = self.table.handle_key(key)?;
        Ok(match result {
            EventResult::Event(TableEvent::Activated(context)) => {
                ContextSelectorEvent::Selected(context).into()
            }
            EventResult::Ignored => match key.code {
                KeyCode::Char('r') => ContextSelectorEvent::Refresh.into(),
                KeyCode::Char('e') => self.table.selected_item().map_or(
                    EventResult::Ignored,
                    |ctx| ContextSelectorEvent::EditAuth(ctx.clone()).into(),
                ),
                _ => EventResult::Ignored,
            },
            _ => EventResult::Consumed,
        })
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        self.table.render(frame, area, theme);
    }

    fn keybindings(&self) -> Vec<Keybinding> {
        vec![
            Keybinding::primary("Enter", "Select"),
            Keybinding::primary("e", "Edit auth"),
            Keybinding::primary("r", "Sync gcloud"),
            Keybinding::secondary("/", "Search"),
        ]
    }
}

// === Context Sync Popup ===

pub enum ContextSyncEvent {
    /// Apply the chosen add/update/remove decisions.
    Apply(Vec<SyncDecision>),
    /// Dismiss without changing anything.
    Cancel,
}

struct SyncItem {
    entry: SyncEntry,
    /// Whether to apply this entry's action (add / update / remove).
    apply: bool,
}

/// A rendered row: either a section header (with its item count) or a
/// selectable item referencing an index into `items`.
enum SyncRow {
    Header(SyncKind, usize),
    Item(usize),
}

pub struct ContextSyncPopup {
    items: Vec<SyncItem>,
    rows: Vec<SyncRow>,
    state: ListState,
}

impl ContextSyncPopup {
    pub fn new(entries: Vec<SyncEntry>) -> Self {
        let items: Vec<SyncItem> = entries
            .into_iter()
            .map(|entry| {
                // Additions/updates default to on; removals default to off so a
                // context isn't deleted without the user opting in.
                let apply = matches!(entry.kind, SyncKind::New | SyncKind::Modified);
                SyncItem { entry, apply }
            })
            .collect();

        let mut rows = Vec::new();
        for kind in [SyncKind::New, SyncKind::Modified, SyncKind::Removed] {
            let indices: Vec<usize> = items
                .iter()
                .enumerate()
                .filter(|(_, it)| it.entry.kind == kind)
                .map(|(i, _)| i)
                .collect();
            if indices.is_empty() {
                continue;
            }
            rows.push(SyncRow::Header(kind, indices.len()));
            rows.extend(indices.into_iter().map(SyncRow::Item));
        }

        let mut state = ListState::default();
        if let Some(pos) = rows
            .iter()
            .position(|r| matches!(r, SyncRow::Item(_)))
        {
            state.select(Some(pos));
        }

        Self { items, rows, state }
    }

    fn move_selection(&mut self, forward: bool) {
        let Some(mut i) = self.state.selected() else {
            return;
        };
        loop {
            if forward {
                if i + 1 >= self.rows.len() {
                    return;
                }
                i += 1;
            } else {
                if i == 0 {
                    return;
                }
                i -= 1;
            }
            if matches!(self.rows[i], SyncRow::Item(_)) {
                self.state.select(Some(i));
                return;
            }
        }
    }

    fn toggle_current(&mut self) {
        if let Some(row) = self.state.selected()
            && let Some(SyncRow::Item(idx)) = self.rows.get(row)
        {
            let idx = *idx;
            self.items[idx].apply = !self.items[idx].apply;
        }
    }

    fn set_all(&mut self, apply: bool) {
        for item in &mut self.items {
            item.apply = apply;
        }
    }

    fn decisions(&self) -> Vec<SyncDecision> {
        self.items
            .iter()
            .filter(|it| it.apply)
            .map(|it| SyncDecision {
                kind: it.entry.kind,
                name: it.entry.incoming.name().to_string(),
                incoming: it.entry.incoming.clone(),
            })
            .collect()
    }
}

impl Component for ContextSyncPopup {
    type Output = ContextSyncEvent;

    fn handle_key(&mut self, key: KeyEvent) -> Result<EventResult<Self::Output>> {
        match key.code {
            KeyCode::Esc => return Ok(ContextSyncEvent::Cancel.into()),
            KeyCode::Enter => return Ok(ContextSyncEvent::Apply(self.decisions()).into()),
            KeyCode::Char(' ') => self.toggle_current(),
            KeyCode::Char('a') => self.set_all(true),
            KeyCode::Char('n') => self.set_all(false),
            KeyCode::Char('k') | KeyCode::Up => self.move_selection(false),
            KeyCode::Char('j') | KeyCode::Down => self.move_selection(true),
            _ => {}
        }
        Ok(EventResult::Consumed)
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let popup_area = area.centered(Constraint::Percentage(80), Constraint::Percentage(70));
        frame.render_widget(Clear, popup_area);

        let block = theme
            .popup_block(" Sync Contexts with gcloud ")
            .border_style(Style::default().fg(theme.border()));

        let list_items: Vec<ListItem> = self
            .rows
            .iter()
            .map(|row| match row {
                SyncRow::Header(kind, count) => ListItem::new(Line::from(Span::styled(
                    format!("── {} ({count}) ──", kind.title()),
                    Style::default()
                        .fg(theme.text_muted())
                        .add_modifier(Modifier::BOLD),
                ))),
                SyncRow::Item(idx) => Self::item_line(&self.items[*idx], theme),
            })
            .collect();

        let list = List::new(list_items)
            .block(block)
            .highlight_style(theme.highlight_style())
            .highlight_symbol("▶ ");

        frame.render_stateful_widget(list, popup_area, &mut self.state);

        let hint_area = Rect::new(
            popup_area.x + 2,
            popup_area.y + popup_area.height.saturating_sub(2),
            popup_area.width.saturating_sub(4),
            1,
        );
        let hint = Line::from(vec![
            Span::styled("Space", Style::default().fg(theme.secondary())),
            Span::styled(" toggle  ", Style::default().fg(theme.text_muted())),
            Span::styled("a", Style::default().fg(theme.secondary())),
            Span::styled(" all  ", Style::default().fg(theme.text_muted())),
            Span::styled("n", Style::default().fg(theme.secondary())),
            Span::styled(" none  ", Style::default().fg(theme.text_muted())),
            Span::styled("Enter", Style::default().fg(theme.secondary())),
            Span::styled(" apply  ", Style::default().fg(theme.text_muted())),
            Span::styled("Esc", Style::default().fg(theme.secondary())),
            Span::styled(" cancel", Style::default().fg(theme.text_muted())),
        ]);
        frame.render_widget(Paragraph::new(hint), hint_area);
    }
}

impl ContextSyncPopup {
    fn item_line<'a>(item: &SyncItem, theme: &Theme) -> ListItem<'a> {
        let (color, action) = match (item.entry.kind, item.apply) {
            (SyncKind::New, true) => (theme.success(), "add"),
            (SyncKind::New, false) => (theme.overlay1(), "skip"),
            (SyncKind::Modified, true) => (theme.warning(), "update"),
            (SyncKind::Removed, true) => (theme.error(), "remove"),
            (SyncKind::Modified | SyncKind::Removed, false) => (theme.overlay1(), "keep"),
        };
        let checkbox = if item.apply { "[x]" } else { "[ ]" };
        let name = item.entry.incoming.name();

        let detail = match item.entry.kind {
            SyncKind::Modified => item
                .entry
                .existing
                .as_ref()
                .map(|existing| gcp_change_summary(existing, &item.entry.incoming))
                .unwrap_or_default(),
            SyncKind::New | SyncKind::Removed => gcp_summary(&item.entry.incoming),
        };

        ListItem::new(Line::from(vec![
            Span::styled(format!("{checkbox} "), Style::default().fg(color)),
            Span::styled(name.to_string(), Style::default().fg(theme.text())),
            Span::styled(format!("  [{action}]  "), Style::default().fg(color)),
            Span::styled(detail, Style::default().fg(theme.text_muted())),
        ]))
    }
}

/// A one-line `project · account · location` summary for a GCP context.
fn gcp_summary(ctx: &CloudContext) -> String {
    match ctx {
        CloudContext::Gcp(c) => {
            let loc = c.region.as_deref().or(c.zone.as_deref()).unwrap_or("—");
            format!("{} · {} · {}", c.project_id, c.account, loc)
        }
    }
}

/// A summary of which gcloud-sourced fields changed, `field old→new`.
fn gcp_change_summary(existing: &CloudContext, incoming: &CloudContext) -> String {
    let opt = |o: &Option<String>| o.clone().unwrap_or_else(|| "—".to_string());
    match (existing, incoming) {
        (CloudContext::Gcp(a), CloudContext::Gcp(b)) => {
            let mut parts = Vec::new();
            if a.project_id != b.project_id {
                parts.push(format!("project {}→{}", a.project_id, b.project_id));
            }
            if a.account != b.account {
                parts.push(format!("account {}→{}", a.account, b.account));
            }
            if a.region != b.region {
                parts.push(format!("region {}→{}", opt(&a.region), opt(&b.region)));
            }
            if a.zone != b.zone {
                parts.push(format!("zone {}→{}", opt(&a.zone), opt(&b.zone)));
            }
            parts.join(", ")
        }
    }
}

// === Auth Method Editor ===

pub enum AuthEditorEvent {
    Save {
        context_name: String,
        auth: AuthMethod,
    },
    Cancel,
}

enum AuthEditorPhase {
    /// Choosing which auth method to use.
    SelectMethod,
    /// Entering the service account key file path.
    EnterKeyPath,
}

/// The selectable auth methods, in display order.
const AUTH_OPTIONS: [(&str, &str); 3] = [
    (
        "Application Default",
        "gcloud ADC, GOOGLE_APPLICATION_CREDENTIALS, or metadata server",
    ),
    (
        "Service Account Key",
        "a service account JSON key file on disk",
    ),
    (
        "Metadata Server",
        "the GCE / GKE / Cloud Run metadata server",
    ),
];

pub struct AuthMethodEditor {
    context_name: String,
    current: AuthMethod,
    state: ListState,
    path_input: TextInput,
    phase: AuthEditorPhase,
}

impl AuthMethodEditor {
    pub fn new(context: &CloudContext) -> Self {
        let current = context.auth().clone();
        let selected = match current {
            AuthMethod::ApplicationDefault => 0,
            AuthMethod::ServiceAccountKey { .. } => 1,
            AuthMethod::MetadataServer => 2,
        };
        let mut state = ListState::default();
        state.select(Some(selected));

        let path_input = match &current {
            AuthMethod::ServiceAccountKey { path } => TextInput::new("Service account key path")
                .with_value(path.display().to_string()),
            _ => TextInput::new("Service account key path").with_placeholder("/path/to/key.json"),
        };

        Self {
            context_name: context.name().to_string(),
            current,
            state,
            path_input,
            phase: AuthEditorPhase::SelectMethod,
        }
    }

    fn save(&self, auth: AuthMethod) -> EventResult<AuthEditorEvent> {
        AuthEditorEvent::Save {
            context_name: self.context_name.clone(),
            auth,
        }
        .into()
    }

    fn move_selection(&mut self, forward: bool) {
        let i = self.state.selected().unwrap_or(0);
        let next = if forward {
            (i + 1).min(AUTH_OPTIONS.len() - 1)
        } else {
            i.saturating_sub(1)
        };
        self.state.select(Some(next));
    }
}

impl Component for AuthMethodEditor {
    type Output = AuthEditorEvent;

    fn handle_key(&mut self, key: KeyEvent) -> Result<EventResult<Self::Output>> {
        match self.phase {
            AuthEditorPhase::SelectMethod => Ok(match key.code {
                KeyCode::Esc => AuthEditorEvent::Cancel.into(),
                KeyCode::Char('k') | KeyCode::Up => {
                    self.move_selection(false);
                    EventResult::Consumed
                }
                KeyCode::Char('j') | KeyCode::Down => {
                    self.move_selection(true);
                    EventResult::Consumed
                }
                KeyCode::Enter => match self.state.selected().unwrap_or(0) {
                    0 => self.save(AuthMethod::ApplicationDefault),
                    1 => {
                        self.phase = AuthEditorPhase::EnterKeyPath;
                        EventResult::Consumed
                    }
                    _ => self.save(AuthMethod::MetadataServer),
                },
                _ => EventResult::Consumed,
            }),
            AuthEditorPhase::EnterKeyPath => Ok(match self.path_input.handle_key(key)? {
                EventResult::Event(TextInputEvent::Submitted(path)) if !path.trim().is_empty() => {
                    self.save(AuthMethod::ServiceAccountKey {
                        path: PathBuf::from(path.trim()),
                    })
                }
                EventResult::Event(TextInputEvent::Cancelled) => {
                    // Back to method selection rather than closing outright.
                    self.phase = AuthEditorPhase::SelectMethod;
                    EventResult::Consumed
                }
                _ => EventResult::Consumed,
            }),
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        if matches!(self.phase, AuthEditorPhase::EnterKeyPath) {
            self.path_input.render(frame, area, theme);
            return;
        }

        let popup_area = area.centered(Constraint::Percentage(60), Constraint::Percentage(50));
        frame.render_widget(Clear, popup_area);

        let title = format!(" Auth Method — {} ", self.context_name);
        let block = theme
            .popup_block(&title)
            .border_style(Style::default().fg(theme.border()));

        let current_idx = match self.current {
            AuthMethod::ApplicationDefault => 0,
            AuthMethod::ServiceAccountKey { .. } => 1,
            AuthMethod::MetadataServer => 2,
        };

        let items: Vec<ListItem> = AUTH_OPTIONS
            .iter()
            .enumerate()
            .map(|(i, (name, desc))| {
                let marker = if i == current_idx { "● " } else { "  " };
                ListItem::new(vec![
                    Line::from(vec![
                        Span::styled(marker, Style::default().fg(theme.success())),
                        Span::styled(
                            (*name).to_string(),
                            Style::default()
                                .fg(theme.text())
                                .add_modifier(Modifier::BOLD),
                        ),
                    ]),
                    Line::from(Span::styled(
                        format!("    {desc}"),
                        Style::default().fg(theme.text_muted()),
                    )),
                ])
            })
            .collect();

        let list = List::new(items)
            .block(block)
            .highlight_style(theme.highlight_style())
            .highlight_symbol("▶ ");

        frame.render_stateful_widget(list, popup_area, &mut self.state);
    }
}
