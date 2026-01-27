use std::sync::Arc;

use crossterm::event::KeyEvent;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::prelude::{Modifier, Style};
use ratatui::widgets::{List as RatatuiList, ListItem, ListState};

use crate::Theme;
use crate::config::{KeyResolver, NavAction};
use crate::ui::{Component, EventResult, Result};

pub enum ListEvent<T> {
    Changed(T),
    Activated(T),
}

pub trait ListRow {
    fn render_row(&self, theme: &Theme) -> ListItem<'static>;
}

pub struct List<T: ListRow + Clone> {
    items: Vec<T>,
    state: ListState,
    resolver: Arc<KeyResolver>,
}

impl<T: ListRow + Clone> List<T> {
    pub fn new(items: Vec<T>, resolver: Arc<KeyResolver>) -> Self {
        let mut state = ListState::default();
        if !items.is_empty() {
            state.select(Some(0));
        }
        Self {
            items,
            state,
            resolver,
        }
    }

    #[allow(dead_code)]
    pub fn selected(&self) -> Option<&T> {
        self.state.selected().and_then(|i| self.items.get(i))
    }

    #[allow(dead_code)]
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

    fn get_change_event(&self, before: Option<usize>) -> EventResult<ListEvent<T>> {
        if let Some(selected) = self.state.selected()
            && Some(selected) != before
        {
            return ListEvent::Changed(self.items[selected].clone()).into();
        }
        EventResult::Consumed
    }
}

impl<T: ListRow + Clone> Component for List<T> {
    type Output = ListEvent<T>;

    fn handle_key(&mut self, key: KeyEvent) -> Result<EventResult<Self::Output>> {
        let before = self.state.selected();

        if self.resolver.matches_nav(&key, NavAction::Down) {
            self.state.select_next();
            return Ok(self.get_change_event(before));
        }
        if self.resolver.matches_nav(&key, NavAction::Up) {
            self.state.select_previous();
            return Ok(self.get_change_event(before));
        }
        if self.resolver.matches_nav(&key, NavAction::Home) {
            self.state.select_first();
            return Ok(self.get_change_event(before));
        }
        if self.resolver.matches_nav(&key, NavAction::End) {
            self.state.select_last();
            return Ok(self.get_change_event(before));
        }
        if self.resolver.matches_nav(&key, NavAction::PageDown) {
            let step = 5;
            let new_index = match self.state.selected() {
                Some(i) => usize::min(i + step, self.items.len().saturating_sub(1)),
                None => 0,
            };
            self.state.select(Some(new_index));
            return Ok(self.get_change_event(before));
        }
        if self.resolver.matches_nav(&key, NavAction::PageUp) {
            let step = 5;
            let new_index = self.state.selected().map_or(0, |i| i.saturating_sub(step));
            self.state.select(Some(new_index));
            return Ok(self.get_change_event(before));
        }
        if self.resolver.matches_nav(&key, NavAction::Select) {
            if let Some(selected) = self.state.selected() {
                return Ok(ListEvent::Activated(self.items[selected].clone()).into());
            }
            return Ok(EventResult::Ignored);
        }

        Ok(EventResult::Ignored)
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let items: Vec<ListItem> = self.items.iter().map(|i| i.render_row(theme)).collect();

        let list = RatatuiList::new(items)
            .highlight_style(
                Style::default()
                    .bg(theme.selection_bg())
                    .fg(theme.lavender())
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶ ");

        frame.render_stateful_widget(list, area, &mut self.state);
    }
}
