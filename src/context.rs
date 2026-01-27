use std::sync::Arc;

use color_eyre::eyre::{Result, eyre};
use crossterm::event::KeyEvent;
use google_cloud_auth::credentials::Credentials;
use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use ratatui::widgets::Cell;
use serde::{Deserialize, Serialize};

use crate::Theme;
use crate::config::{KeyResolver, config_dir};
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
    pub fn create_credentials(&self) -> Result<Credentials> {
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

pub fn load_contexts() -> Vec<CloudContext> {
    if let Some(config_dir) = config_dir() {
        let path = config_dir.join(CONTEXTS_FILE);
        if let Ok(data) = std::fs::read_to_string(path)
            && let Ok(contexts) = serde_json::from_str::<Vec<CloudContext>>(&data)
        {
            return contexts;
        }
    }
    Vec::new()
}

pub fn save_contexts(contexts: &[CloudContext]) -> Result<()> {
    if let Some(config_dir) = config_dir() {
        std::fs::create_dir_all(&config_dir)?;
        let path = config_dir.join(CONTEXTS_FILE);
        let data = serde_json::to_string_pretty(contexts)?;
        std::fs::write(path, data)?;
    }
    Ok(())
}

pub fn find_by_name(contexts: &[CloudContext], name: &str) -> Result<CloudContext> {
    contexts
        .iter()
        .find(|c| c.name().eq_ignore_ascii_case(name))
        .cloned()
        .ok_or_else(|| {
            let available: Vec<_> = contexts.iter().map(CloudContext::name).collect();
            eyre!(
                "Context '{}' not found. Available: {}",
                name,
                available.join(", ")
            )
        })
}

pub fn reconcile_contexts() -> Result<Vec<CloudContext>> {
    let mut contexts = load_contexts();
    let discovered_configs = discover_gcloud_configs();

    for config in discovered_configs {
        if !contexts.iter().any(|ctx| match ctx {
            CloudContext::Gcp(existing) => existing.display_name == config.name,
        }) {
            contexts.push(CloudContext::Gcp(GcpContext {
                display_name: config.name,
                project_id: config.core.project,
                account: config.core.account,
                region: config.compute.region,
                zone: config.compute.zone,
                auth: AuthMethod::ApplicationDefault,
            }));
        }
    }

    save_contexts(&contexts)?;

    Ok(contexts)
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

pub struct ContextSelectorView {
    table: Table<CloudContext>,
}

impl ContextSelectorView {
    pub fn new(resolver: Arc<KeyResolver>) -> Result<Self> {
        Ok(Self::with_contexts(reconcile_contexts()?, resolver))
    }

    pub fn with_contexts(contexts: Vec<CloudContext>, resolver: Arc<KeyResolver>) -> Self {
        Self {
            table: Table::new(contexts, resolver).with_title(" Contexts "),
        }
    }
}

impl Screen for ContextSelectorView {
    type Output = CloudContext;

    fn handle_key(&mut self, key: KeyEvent) -> Result<EventResult<Self::Output>> {
        let result = self.table.handle_key(key)?;
        Ok(match result {
            EventResult::Event(TableEvent::Activated(context)) => context.into(),
            EventResult::Consumed | EventResult::Event(_) => EventResult::Consumed,
            EventResult::Ignored => EventResult::Ignored,
        })
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        self.table.render(frame, area, theme);
    }
}
