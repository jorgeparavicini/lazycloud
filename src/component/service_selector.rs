use crate::component::{ListComponent, ListEvent, ListRow};
use crate::config::KeyResolver;
use crate::model::CloudContext;
use crate::registry::{ServiceId, ServiceProvider, ServiceRegistry};
use crate::ui::{Component, Handled, Result};
use crate::Theme;
use crossterm::event::KeyEvent;
use ratatui::{layout::Rect, style::Style, widgets::ListItem, Frame};
use std::sync::Arc;

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

    fn handle_key(&mut self, key: KeyEvent) -> Result<Handled<Self::Output>> {
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
