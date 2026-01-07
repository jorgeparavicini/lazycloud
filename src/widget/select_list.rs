use crossterm::event::KeyEvent;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::prelude::{Color, Modifier, Style};
use ratatui::widgets::{List, ListItem, ListState};
use std::fmt::Display;

pub enum ListEvent<'a, T> {
    Changed(&'a T),
    Activated(&'a T),
}

pub struct SelectList<T: Display> {
    items: Vec<T>,
    state: ListState,
}

impl<T: Display> SelectList<T> {
    pub fn new(items: Vec<T>) -> Self {
        let mut state = ListState::default();
        if !items.is_empty() {
            state.select(Some(0));
        }
        Self { items, state }
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) -> Option<ListEvent<'_, T>> {
        use crossterm::event::KeyCode::*;

        let before = self.state.selected();

        match key.code {
            Down | Char('j')  => {
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
                // Move down by 5 items or to the end
                let step = 5;
                let new_index = match self.state.selected() {
                    Some(i) => usize::min(i + step, self.items.len() - 1),
                    None => 0,
                };
                self.state.select(Some(new_index));
                self.get_change_event(before)
            }
            PageUp => {
                // Move up by 5 items or to the start
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
                    Some(ListEvent::Activated(&self.items[selected]))
                } else {
                    None
                }
            }
            _ => None,
        }
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

    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        let items = self
            .items
            .iter()
            .map(|i| ListItem::new(i.to_string()))
            .collect::<Vec<ListItem>>();

        let list = List::new(items)
            .highlight_style(
                Style::default()
                    .bg(Color::Blue)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(">> ");

        frame.render_stateful_widget(list, area, &mut self.state)
    }

    fn get_change_event(&self, before: Option<usize>) -> Option<ListEvent<'_, T>> {
        if let Some(selected) = self.state.selected() && Some(selected) != before {
            Some(ListEvent::Changed(&self.items[selected]))
        } else {
            None
        }
    }
}
