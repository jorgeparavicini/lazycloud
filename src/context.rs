use color_eyre::eyre::{Result, eyre};
use crossterm::event::{KeyCode, KeyEvent};
use google_cloud_auth::build_errors;
use google_cloud_auth::credentials::Credentials;
use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Cell, Clear, List, ListItem, ListState};
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info};

use crate::Theme;
use crate::config::config_dir;
use crate::provider::Provider;
use crate::provider::gcp::discover_gcloud_configs;
use crate::search::Matcher;
use crate::ui::{ColumnDef, Component, EventResult, Screen, Table, TableEvent, TableRow};

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthMethod {
    ApplicationDefault,
}

impl GcpContext {
    pub fn create_credentials(&self) -> Result<Credentials, build_errors::Error> {
        match &self.auth {
            AuthMethod::ApplicationDefault => {
                Ok(google_cloud_auth::credentials::Builder::default().build()?)
            }
        }
    }
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
}

impl std::fmt::Display for CloudContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
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

    /// Discover contexts from gcloud that aren't saved yet.
    pub fn discover_new(&self) -> Vec<CloudContext> {
        let discovered = Self::discover_all();
        discovered
            .into_iter()
            .filter(|ctx| {
                !self
                    .contexts
                    .iter()
                    .any(|existing| existing.name() == ctx.name())
            })
            .collect()
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

    /// Add new contexts and save to disk.
    pub fn add_contexts(&mut self, contexts: Vec<CloudContext>) -> Result<()> {
        self.contexts.extend(contexts);
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
    Refresh,
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
        if key.code == KeyCode::Char('r') {
            return Ok(ContextSelectorEvent::Refresh.into());
        }

        let result = self.table.handle_key(key)?;
        Ok(match result {
            EventResult::Event(TableEvent::Activated(context)) => {
                ContextSelectorEvent::Selected(context).into()
            }
            EventResult::Consumed | EventResult::Event(_) => EventResult::Consumed,
            EventResult::Ignored => EventResult::Ignored,
        })
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        self.table.render(frame, area, theme);
    }
}

// === Context Merge Popup ===

pub enum ContextMergeEvent {
    Import(Vec<CloudContext>),
    Skip,
}

struct SelectableContext {
    context: CloudContext,
    selected: bool,
}

pub struct ContextMergePopup {
    items: Vec<SelectableContext>,
    state: ListState,
}

impl ContextMergePopup {
    pub fn new(contexts: Vec<CloudContext>) -> Self {
        let items: Vec<SelectableContext> = contexts
            .into_iter()
            .map(|context| SelectableContext {
                context,
                selected: true, // Default to selected
            })
            .collect();

        let mut state = ListState::default();
        if !items.is_empty() {
            state.select(Some(0));
        }

        Self {
            items,
            state,
        }
    }

    fn toggle_current(&mut self) {
        if let Some(idx) = self.state.selected()
            && let Some(item) = self.items.get_mut(idx)
        {
            item.selected = !item.selected;
        }
    }

    fn select_all(&mut self) {
        for item in &mut self.items {
            item.selected = true;
        }
    }

    fn select_none(&mut self) {
        for item in &mut self.items {
            item.selected = false;
        }
    }

    fn get_selected_contexts(&self) -> Vec<CloudContext> {
        self.items
            .iter()
            .filter(|item| item.selected)
            .map(|item| item.context.clone())
            .collect()
    }

    const fn move_up(&mut self) {
        if self.items.is_empty() {
            return;
        }
        let i = match self.state.selected() {
            Some(i) => i.saturating_sub(1),
            None => 0,
        };
        self.state.select(Some(i));
    }

    const fn move_down(&mut self) {
        if self.items.is_empty() {
            return;
        }
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
                    i
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }
}

impl Component for ContextMergePopup {
    type Output = ContextMergeEvent;

    fn handle_key(&mut self, key: KeyEvent) -> Result<EventResult<Self::Output>> {
        match key.code {
            KeyCode::Esc => return Ok(ContextMergeEvent::Skip.into()),
            KeyCode::Enter => {
                let selected = self.get_selected_contexts();
                return Ok(ContextMergeEvent::Import(selected).into());
            }
            KeyCode::Char(' ') => self.toggle_current(),
            KeyCode::Char('a') => self.select_all(),
            KeyCode::Char('n') => self.select_none(),
            _ => {
                if matches!(key.code, KeyCode::Char('k') | KeyCode::Up) {
                    self.move_up();
                } else if matches!(key.code, KeyCode::Char('j') | KeyCode::Down) {
                    self.move_down();
                }
            }
        }
        Ok(EventResult::Consumed)
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let popup_area = area.centered(Constraint::Percentage(70), Constraint::Percentage(60));

        frame.render_widget(Clear, popup_area);

        let block = theme.popup_block(" Import New Contexts ")
            .border_style(Style::default().fg(theme.border()));

        let items: Vec<ListItem> = self
            .items
            .iter()
            .map(|item| {
                let checkbox = if item.selected { "[x]" } else { "[ ]" };
                let name = item.context.name();
                let project = match &item.context {
                    CloudContext::Gcp(ctx) => &ctx.project_id,
                };
                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!("{checkbox} "),
                        Style::default().fg(if item.selected {
                            theme.success()
                        } else {
                            theme.overlay1()
                        }),
                    ),
                    Span::styled(name.to_string(), Style::default().fg(theme.text())),
                    Span::styled(
                        format!(" ({project})"),
                        Style::default().fg(theme.text_muted()),
                    ),
                ]))
            })
            .collect();

        let list = List::new(items)
            .block(block)
            .highlight_style(theme.highlight_style())
            .highlight_symbol("▶ ");

        frame.render_stateful_widget(list, popup_area, &mut self.state);

        // Render hint at bottom
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
            Span::styled(" import  ", Style::default().fg(theme.text_muted())),
            Span::styled("Esc", Style::default().fg(theme.secondary())),
            Span::styled(" skip", Style::default().fg(theme.text_muted())),
        ]);
        frame.render_widget(hint, hint_area);
    }
}
