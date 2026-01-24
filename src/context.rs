use color_eyre::eyre::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crossterm::event::KeyEvent;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::ListItem;
use crate::ui::{EventResult, List, ListEvent, ListRow, Screen};
use crate::config::KeyResolver;
use crate::provider::Provider;
use crate::Theme;

const CONTEXTS_FILE: &str = "contexts.toml";

/// Cloud context containing connection and authentication details.
///
/// Each variant holds provider-specific configuration needed to
/// authenticate and interact with that cloud provider's APIs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CloudContext {
    Gcp(GcpContext),
}

/// GCP connection context enriched with lazycloud-specific configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GcpContext {
    pub display_name: String,
    pub project_id: String,
    pub account: String,
    pub region: Option<String>,
    pub zone: Option<String>,
    pub auth: AuthMethod,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuthMethod {
    ApplicationDefault,
    ServiceAccount(String),
}

impl CloudContext {
    /// Get the provider for this context.
    pub fn provider(&self) -> Provider {
        match self {
            CloudContext::Gcp(_) => Provider::Gcp,
        }
    }

    /// Get a short display name for this context.
    pub fn name(&self) -> &str {
        match self {
            CloudContext::Gcp(ctx) => &ctx.display_name,
        }
    }
}

impl std::fmt::Display for CloudContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

pub fn load_contexts() -> Vec<CloudContext> {
    if let Some(config_dir) = get_config_dir() {
        let path = config_dir.join(CONTEXTS_FILE);
        if let Ok(data) = std::fs::read_to_string(path) {
            if let Ok(contexts) = toml::from_str::<Vec<CloudContext>>(&data) {
                return contexts;
            }
        }
    }
    Vec::new()
}

pub fn save_contexts(contexts: &[CloudContext]) -> Result<()> {
    if let Some(config_dir) = get_config_dir() {
        std::fs::create_dir_all(&config_dir)?;
        let path = config_dir.join(CONTEXTS_FILE);
        let data = toml::to_string_pretty(contexts)?;
        std::fs::write(path, data)?;
    }
    Ok(())
}

pub fn reconcile_contexts() -> Result<Vec<CloudContext>> {
    let mut contexts = load_contexts();
    let discovered_configs = crate::provider::gcp::config::discover_gcloud_configs();

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

fn get_config_dir() -> Option<std::path::PathBuf> {
    dirs::config_dir().map(|dir| dir.join(crate::config::CONFIG_FOLDER))
}

// === UI ===


impl ListRow for CloudContext {
    fn render_row(&self, theme: &Theme) -> ListItem<'static> {
        ListItem::new(self.to_string()).style(Style::default().fg(theme.text()))
    }
}

pub struct ContextSelectorView {
    context_list: List<CloudContext>,
}

impl ContextSelectorView {
    pub fn new(resolver: Arc<KeyResolver>) -> Self {
        Self::with_contexts(load_contexts(), resolver)
    }

    pub fn with_contexts(contexts: Vec<CloudContext>, resolver: Arc<KeyResolver>) -> Self {
        Self {
            context_list: List::new(contexts, resolver),
        }
    }
}

impl Screen for ContextSelectorView {
    type Output = CloudContext;

    fn handle_key(&mut self, key: KeyEvent) -> Result<EventResult<Self::Output>> {
        let result = self.context_list.handle_key(key)?;
        Ok(match result {
            EventResult::Event(ListEvent::Activated(context)) => context.into(),
            EventResult::Consumed | EventResult::Event(_) => EventResult::Consumed,
            EventResult::Ignored => EventResult::Ignored,
        })
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        self.context_list.render(frame, area, theme);
    }
}
