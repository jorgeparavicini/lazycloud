//! Service selector for choosing a cloud service.

use crate::model::CloudContext;
use crate::registry::{ServiceId, ServiceProvider, ServiceRegistry};
use crate::widget::ListEvent::Activated;
use crate::widget::SelectList;
use crossterm::event::KeyEvent;
use ratatui::{layout::Rect, Frame};
use std::sync::Arc;

/// Wrapper for displaying service providers in the list.
struct ServiceItem {
    provider: Arc<dyn ServiceProvider>,
}

impl std::fmt::Display for ServiceItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(icon) = self.provider.icon() {
            write!(f, "{} {}", icon, self.provider.display_name())
        } else {
            write!(f, "{}", self.provider.display_name())
        }
    }
}

/// Widget for selecting a cloud service.
pub struct ServiceSelector {
    service_list: SelectList<ServiceItem>,
}

impl ServiceSelector {
    /// Create a new service selector for the given context.
    pub fn new(registry: Arc<ServiceRegistry>, context: CloudContext) -> Self {
        let services: Vec<ServiceItem> = registry
            .available_services(&context)
            .into_iter()
            .map(|provider| ServiceItem { provider })
            .collect();

        Self {
            service_list: SelectList::new(services),
        }
    }

    /// Handle a key event. Returns the selected service ID if Enter was pressed.
    pub fn handle_key_event(&mut self, key: KeyEvent) -> Option<ServiceId> {
        if let Some(Activated(item)) = self.service_list.handle_key_event(key) {
            return Some(item.provider.service_id());
        }
        None
    }

    /// Render the service selector.
    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        self.service_list.render(frame, area);
    }
}
