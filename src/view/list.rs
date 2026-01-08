use crate::view::{KeyResult, View};
use crate::Theme;
use crossterm::event::KeyEvent;
use ratatui::layout::Rect;
use ratatui::prelude::{Modifier, Style};
use ratatui::widgets::{List, ListItem, ListState};
use ratatui::Frame;

/// Event emitted by [`ListView`].
pub enum ListEvent<T> {
    /// Selection changed to a new item.
    Changed(T),
    /// Item was activated (Enter pressed).
    Activated(T),
}

/// Trait for items that can be displayed in a list.
pub trait ListRow {
    /// Render this item as a list item with full styling control.
    fn render_row(&self, theme: &Theme) -> ListItem<'static>;
}

/// A selectable list view with keyboard navigation.
pub struct ListView<T: ListRow + Clone> {
    items: Vec<T>,
    state: ListState,
}

impl<T: ListRow + Clone> ListView<T> {
    pub fn new(items: Vec<T>) -> Self {
        let mut state = ListState::default();
        if !items.is_empty() {
            state.select(Some(0));
        }
        Self { items, state }
    }

    pub fn selected(&self) -> Option<&T> {
        self.state.selected().and_then(|i| self.items.get(i))
    }

    pub fn set_items(&mut self, items: Vec<T>) {
        self.items = items;

        if self.items.is_empty() {
            self.state.select(None);
        } else if let Some(i) = self.state.selected() {
            if i >= self.items.len() {
                self.state.select(Some(self.items.len() - 1));
            }
        } else {
            self.state.select(Some(0));
        }
    }

    fn get_change_event(&self, before: Option<usize>) -> KeyResult<ListEvent<T>> {
        if let Some(selected) = self.state.selected() {
            if Some(selected) != before {
                return ListEvent::Changed(self.items[selected].clone()).into();
            }
        }
        KeyResult::Consumed
    }
}

impl<T: ListRow + Clone> View for ListView<T> {
    type Event = ListEvent<T>;

    fn handle_key(&mut self, key: KeyEvent) -> KeyResult<Self::Event> {
        use crossterm::event::KeyCode::*;

        let before = self.state.selected();

        match key.code {
            Down | Char('j') => {
                self.state.select_next();
                self.get_change_event(before)
            }
            Up | Char('k') => {
                self.state.select_previous();
                self.get_change_event(before)
            }
            Home | Char('g') => {
                self.state.select_first();
                self.get_change_event(before)
            }
            End | Char('G') => {
                self.state.select_last();
                self.get_change_event(before)
            }
            PageDown => {
                let step = 5;
                let new_index = match self.state.selected() {
                    Some(i) => usize::min(i + step, self.items.len().saturating_sub(1)),
                    None => 0,
                };
                self.state.select(Some(new_index));
                self.get_change_event(before)
            }
            PageUp => {
                let step = 5;
                let new_index = match self.state.selected() {
                    Some(i) => i.saturating_sub(step),
                    None => 0,
                };
                self.state.select(Some(new_index));
                self.get_change_event(before)
            }
            Enter => {
                if let Some(selected) = self.state.selected() {
                    ListEvent::Activated(self.items[selected].clone()).into()
                } else {
                    KeyResult::Ignored
                }
            }
            _ => KeyResult::Ignored,
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let items: Vec<ListItem> = self
            .items
            .iter()
            .map(|i| i.render_row(theme))
            .collect();

        let list = List::new(items)
            .highlight_style(
                Style::default()
                    .bg(theme.selection_bg())
                    .fg(theme.lavender())
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶ ");

        frame.render_stateful_widget(list, area, &mut self.state)
    }
}
