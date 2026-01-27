use std::sync::Arc;

use color_eyre::Result;
use crossterm::event::KeyEvent;
use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use ratatui::widgets::Cell;

use crate::Theme;
use crate::commands::Command;
use crate::config::KeyResolver;
use crate::context::CloudContext;
use crate::registry::{ServiceId, ServiceProvider, ServiceRegistry};
use crate::search::Matcher;
use crate::ui::{ColumnDef, Component, EventResult, Keybinding, Table, TableEvent, TableRow};

pub enum ServiceMsg {
    /// No action needed
    Idle,
    /// Run one or more commands
    Run(Vec<Box<dyn Command>>),
    /// Close this service (go back to service selection)
    Close,
}

impl<T: Command> From<T> for ServiceMsg {
    fn from(value: T) -> Self {
        Self::Run(vec![Box::new(value)])
    }
}

/// A cloud service screen.
///
/// Services manage their own internal state and message queue. The App calls
/// methods in this order:
///
/// 1. `init()` - once when service becomes active
/// 2. `update()` - immediately after init to process startup messages
/// 3. For each event:
///    - `handle_tick()` if tick event
///    - `handle_input()` if input event, then `update()` if consumed
/// 4. When commands completes: `update()`
/// 5. `destroy()` - when service is closing
pub trait Service {
    /// Initialize the service by queuing startup message(s).
    fn init(&mut self) {}

    /// Clean up when the service is closing.
    fn destroy(&mut self) {}

    /// Handle a tick event for animations.
    fn handle_tick(&mut self) {}

    /// Handle a key event.
    fn handle_key(&mut self, key: KeyEvent) -> EventResult<()>;

    /// Process all queued messages and return the result.
    ///
    /// # Errors
    /// Returns an error if message processing fails.
    /// In this case, the App will display the error and the service might be in an invalid state.
    fn update(&mut self) -> Result<ServiceMsg>;

    /// Render the service to the frame.
    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme);

    /// Breadcrumb segments for the navigation bar.
    fn breadcrumbs(&self) -> Vec<String>;

    /// Returns the keybindings for the current view in this service.
    fn keybindings(&self) -> Vec<Keybinding> {
        vec![]
    }
}

#[derive(Clone)]
struct ServiceItem {
    provider: Arc<dyn ServiceProvider>,
}

impl TableRow for ServiceItem {
    fn columns() -> &'static [ColumnDef] {
        static COLUMNS: &[ColumnDef] = &[
            ColumnDef::new("Name", Constraint::Min(20)),
            ColumnDef::new("Provider", Constraint::Length(10)),
            ColumnDef::new("Description", Constraint::Min(30)),
        ];
        COLUMNS
    }

    fn render_cells(&self, _theme: &Theme) -> Vec<Cell<'static>> {
        let name = self.provider.icon().map_or_else(
            || self.provider.display_name().to_string(),
            |icon| format!("{icon} {}", self.provider.display_name()),
        );
        vec![
            Cell::from(name),
            Cell::from(format!("{}", self.provider.provider())),
            Cell::from(self.provider.description().to_string()),
        ]
    }

    fn matches(&self, query: &str) -> bool {
        let matcher = Matcher::new();
        matcher.matches(self.provider.display_name(), query)
            || matcher.matches(self.provider.service_key(), query)
            || matcher.matches(self.provider.description(), query)
    }
}

pub struct ServiceSelectorView {
    table: Table<ServiceItem>,
}

impl ServiceSelectorView {
    #[must_use]
    pub fn new(
        registry: &Arc<ServiceRegistry>,
        context: &CloudContext,
        resolver: Arc<KeyResolver>,
    ) -> Self {
        let services: Vec<ServiceItem> = registry
            .available_services(context)
            .into_iter()
            .map(|provider| ServiceItem { provider })
            .collect();

        Self {
            table: Table::new(services, resolver).with_title(" Services "),
        }
    }
}

impl Component for ServiceSelectorView {
    type Output = ServiceId;

    fn handle_key(&mut self, key: KeyEvent) -> Result<EventResult<Self::Output>> {
        let result = self.table.handle_key(key)?;
        Ok(match result {
            EventResult::Event(TableEvent::Activated(item)) => item.provider.service_id().into(),
            EventResult::Consumed | EventResult::Event(_) => EventResult::Consumed,
            EventResult::Ignored => EventResult::Ignored,
        })
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        self.table.render(frame, area, theme);
    }
}
