use std::sync::Arc;
use crate::commands::Command;
use crate::components::{Component, Keybinding};
use crate::core::event::Event;
use color_eyre::Result;
use crossterm::event::KeyEvent;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::prelude::Style;
use ratatui::widgets::ListItem;
use crate::config::KeyResolver;
use crate::registry::{ServiceId, ServiceProvider, ServiceRegistry};
use crate::Theme;

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
        ServiceMsg::Run(vec![Box::new(value)])
    }
}

pub enum InputResult {
    Ignored,
    Consumed,
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
pub trait Service: Component<Output = ServiceMsg> {
    /// Initialize the service by queuing startup message(s).
    ///
    /// Called once when the service becomes active. Queue your initial
    /// message (e.g., `Initialize`) here. The App will call `update()`
    /// immediately after to process it.
    fn init(&mut self) {}

    /// Clean up when the service is closing.
    fn destroy(&mut self) {}

    /// Process all queued messages and return the result.
    ///
    /// Called by the App:
    /// - After `init()`
    /// - After `handle_input()` returns `EventResult::Consumed`
    /// - After any commands completed
    fn update(&mut self) -> Result<ServiceMsg>;

    /// Breadcrumb segments for the navigation bar.
    fn breadcrumbs(&self) -> Vec<String>;

    /// Returns the keybindings for the current view in this service.
    /// This is used to populate the help screen.
    fn keybindings(&self) -> Vec<Keybinding> {
        vec![]
    }
}


#[derive(Clone)]
struct ServiceItem {
    provider: Arc<dyn ServiceProvider>,
}

impl ListRow for ServiceItem {
    fn render_row(&self, theme: &Theme) -> ListItem<'static> {
        let text = if let Some(icon) = self.provider.icon() {
            format!("{} {}", icon, self.provider.display_name())
        } else {
            self.provider.display_name().to_string()
        };
        ListItem::new(text).style(Style::default().fg(theme.text()))
    }
}

pub struct ServiceSelectorView {
    service_list: ListComponent<ServiceItem>,
}

impl ServiceSelectorView {
    pub fn new(
        registry: Arc<ServiceRegistry>,
        context: CloudContext,
        resolver: Arc<KeyResolver>,
    ) -> Self {
        let services: Vec<ServiceItem> = registry
            .available_services(&context)
            .into_iter()
            .map(|provider| ServiceItem { provider })
            .collect();

        Self {
            service_list: ListComponent::new(services, resolver),
        }
    }
}

impl Component for ServiceSelectorView {
    type Output = ServiceId;

    fn handle_key(&mut self, key: KeyEvent) -> std::result::Result<Handled<Self::Output>> {
        let result = self.service_list.handle_key(key)?;
        Ok(match result {
            Handled::Event(ListEvent::Activated(item)) => item.provider.service_id().into(),
            Handled::Consumed | Handled::Event(_) => Handled::Consumed,
            Handled::Ignored => Handled::Ignored,
        })
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        self.service_list.render(frame, area, theme);
    }
}
