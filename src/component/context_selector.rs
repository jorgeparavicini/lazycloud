use crate::Theme;
use crate::component::{ListComponent, ListEvent, ListRow};
use crate::config::KeyResolver;
use crate::model::context::{CloudContext, get_available_contexts};
use crate::ui::{Component, Handled, Result};
use crossterm::event::KeyEvent;
use ratatui::{Frame, layout::Rect, style::Style, widgets::ListItem};
use std::sync::Arc;

impl ListRow for CloudContext {
    fn render_row(&self, theme: &Theme) -> ListItem<'static> {
        ListItem::new(self.to_string()).style(Style::default().fg(theme.text()))
    }
}

pub struct ContextSelectorView {
    context_list: ListComponent<CloudContext>,
}

impl ContextSelectorView {
    pub fn new(resolver: Arc<KeyResolver>) -> Self {
        let contexts = get_available_contexts();
        Self {
            context_list: ListComponent::new(contexts, resolver),
        }
    }
}

impl Component for ContextSelectorView {
    type Output = CloudContext;

    fn handle_key(&mut self, key: KeyEvent) -> Result<Handled<Self::Output>> {
        let result = self.context_list.handle_key(key)?;
        Ok(match result {
            Handled::Event(ListEvent::Activated(context)) => context.into(),
            Handled::Consumed | Handled::Event(_) => Handled::Consumed,
            Handled::Ignored => Handled::Ignored,
        })
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        self.context_list.render(frame, area, theme);
    }
}
