use crate::model::CloudContext;
use crate::registry::{ServiceId, ServiceProvider, ServiceRegistry};
use crate::view::{ListEvent, ListRow, ListView, View};
use crate::Theme;
use crossterm::event::KeyEvent;
use ratatui::{
    layout::Rect,
    style::Style,
    widgets::ListItem,
    Frame,
};
use std::sync::Arc;

/// Wrapper for displaying service providers in the list.
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

/// View for selecting a cloud service.
pub struct ServiceSelectorView {
    service_list: ListView<ServiceItem>,
}

impl ServiceSelectorView {
    /// Create a new service selector for the given context.
    pub fn new(registry: Arc<ServiceRegistry>, context: CloudContext) -> Self {
        let services: Vec<ServiceItem> = registry
            .available_services(&context)
            .into_iter()
            .map(|provider| ServiceItem { provider })
            .collect();

        Self {
            service_list: ListView::new(services),
        }
    }
}

impl View for ServiceSelectorView {
    type Event = ServiceId;

    fn handle_key(&mut self, key: KeyEvent) -> Option<Self::Event> {
        if let Some(ListEvent::Activated(item)) = self.service_list.handle_key(key) {
            return Some(item.provider.service_id());
        }
        None
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        self.service_list.render(frame, area, theme);
    }
}
