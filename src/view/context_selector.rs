use crate::model::context::{get_available_contexts, CloudContext};
use crate::view::{KeyResult, ListEvent, ListRow, ListView, View};
use crate::Theme;
use crossterm::event::KeyEvent;
use ratatui::{
    layout::Rect,
    style::Style,
    widgets::ListItem,
    Frame,
};

impl ListRow for CloudContext {
    fn render_row(&self, theme: &Theme) -> ListItem<'static> {
        ListItem::new(self.to_string()).style(Style::default().fg(theme.text()))
    }
}

/// View for selecting a cloud context.
pub struct ContextSelectorView {
    context_list: ListView<CloudContext>,
}

impl ContextSelectorView {
    pub fn new() -> Self {
        let contexts = get_available_contexts();
        Self {
            context_list: ListView::new(contexts),
        }
    }
}

impl Default for ContextSelectorView {
    fn default() -> Self {
        Self::new()
    }
}

impl View for ContextSelectorView {
    type Event = CloudContext;

    fn handle_key(&mut self, key: KeyEvent) -> KeyResult<Self::Event> {
        let result = self.context_list.handle_key(key);
        if let KeyResult::Event(ListEvent::Activated(context)) = result {
            return context.into();
        }
        if result.is_consumed() {
            KeyResult::Consumed
        } else {
            KeyResult::Ignored
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        self.context_list.render(frame, area, theme);
    }
}
