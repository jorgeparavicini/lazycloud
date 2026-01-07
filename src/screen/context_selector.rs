//! Context selector for choosing a cloud context.

use crate::model::context::{get_available_contexts, CloudContext};
use crate::widget::ListEvent::Activated;
use crate::widget::SelectList;
use crossterm::event::KeyEvent;
use ratatui::{layout::Rect, Frame};

/// Widget for selecting a cloud context.
pub struct ContextSelector {
    context_list: SelectList<CloudContext>,
}

impl ContextSelector {
    pub fn new() -> Self {
        let contexts = get_available_contexts();
        Self {
            context_list: SelectList::new(contexts),
        }
    }

    /// Handle a key event. Returns the selected context if Enter was pressed.
    pub fn handle_key_event(&mut self, key: KeyEvent) -> Option<CloudContext> {
        if let Some(Activated(context)) = self.context_list.handle_key_event(key) {
            return Some(context.clone());
        }
        None
    }

    /// Render the context selector.
    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        self.context_list.render(frame, area);
    }
}

impl Default for ContextSelector {
    fn default() -> Self {
        Self::new()
    }
}
