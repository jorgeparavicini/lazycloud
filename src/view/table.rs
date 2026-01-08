use crate::view::View;
use crate::Theme;
use crossterm::event::KeyEvent;
use ratatui::layout::{Constraint, Rect};
use ratatui::prelude::{Modifier, Style};
use ratatui::widgets::{Block, BorderType, Borders, Cell, Row, Table, TableState};
use ratatui::Frame;

/// Event emitted by [`TableView`].
pub enum TableEvent<T> {
    /// Selection changed to a new item.
    Changed(T),
    /// Item was activated (Enter pressed).
    Activated(T),
}

/// Column definition for a table.
pub struct ColumnDef {
    pub header: &'static str,
    pub constraint: Constraint,
}

impl ColumnDef {
    pub const fn new(header: &'static str, constraint: Constraint) -> Self {
        Self { header, constraint }
    }
}

/// Trait for items that can be displayed in a table.
pub trait TableRow {
    /// Column definitions for this row type.
    fn columns() -> &'static [ColumnDef];

    /// Render this row's cells with full styling control.
    fn render_cells(&self, theme: &Theme) -> Vec<Cell<'static>>;
}

/// A selectable table view with keyboard navigation.
pub struct TableView<T: TableRow + Clone> {
    items: Vec<T>,
    state: TableState,
    title: Option<String>,
}

impl<T: TableRow + Clone> TableView<T> {
    pub fn new(items: Vec<T>) -> Self {
        let mut state = TableState::default();
        if !items.is_empty() {
            state.select(Some(0));
        }
        Self {
            items,
            state,
            title: None,
        }
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn selected(&self) -> Option<&T> {
        self.state.selected().and_then(|i| self.items.get(i))
    }

    fn select_next(&mut self) {
        if self.items.is_empty() {
            return;
        }
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
                    i
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    fn select_previous(&mut self) {
        if self.items.is_empty() {
            return;
        }
        let i = match self.state.selected() {
            Some(i) => i.saturating_sub(1),
            None => 0,
        };
        self.state.select(Some(i));
    }

    fn select_first(&mut self) {
        if !self.items.is_empty() {
            self.state.select(Some(0));
        }
    }

    fn select_last(&mut self) {
        if !self.items.is_empty() {
            self.state.select(Some(self.items.len() - 1));
        }
    }

    fn get_change_event(&self, before: Option<usize>) -> Option<TableEvent<T>> {
        if let Some(selected) = self.state.selected() {
            if Some(selected) != before {
                return Some(TableEvent::Changed(self.items[selected].clone()));
            }
        }
        None
    }
}

impl<T: TableRow + Clone> View for TableView<T> {
    type Event = TableEvent<T>;

    fn handle_key(&mut self, key: KeyEvent) -> Option<Self::Event> {
        use crossterm::event::KeyCode::*;

        let before = self.state.selected();

        match key.code {
            Down | Char('j') => {
                self.select_next();
                self.get_change_event(before)
            }
            Up | Char('k') => {
                self.select_previous();
                self.get_change_event(before)
            }
            Home | Char('g') => {
                self.select_first();
                self.get_change_event(before)
            }
            End | Char('G') => {
                self.select_last();
                self.get_change_event(before)
            }
            PageDown => {
                let step = 10;
                let new_index = match self.state.selected() {
                    Some(i) if !self.items.is_empty() => {
                        usize::min(i + step, self.items.len() - 1)
                    }
                    _ => 0,
                };
                if !self.items.is_empty() {
                    self.state.select(Some(new_index));
                }
                self.get_change_event(before)
            }
            PageUp => {
                let step = 10;
                let new_index = match self.state.selected() {
                    Some(i) => i.saturating_sub(step),
                    None => 0,
                };
                if !self.items.is_empty() {
                    self.state.select(Some(new_index));
                }
                self.get_change_event(before)
            }
            Enter => {
                if let Some(selected) = self.state.selected() {
                    Some(TableEvent::Activated(self.items[selected].clone()))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let columns = T::columns();

        let header_cells: Vec<Cell> = columns
            .iter()
            .map(|c| {
                Cell::from(c.header).style(
                    Style::default()
                        .fg(theme.header())
                        .add_modifier(Modifier::BOLD),
                )
            })
            .collect();
        let header = Row::new(header_cells)
            .height(1)
            .style(Style::default().bg(theme.surface0()));

        let rows: Vec<Row> = self
            .items
            .iter()
            .map(|item| Row::new(item.render_cells(theme)).style(Style::default().fg(theme.text())))
            .collect();

        let widths: Vec<Constraint> = columns.iter().map(|c| c.constraint).collect();

        let mut table = Table::new(rows, widths)
            .header(header)
            .row_highlight_style(
                Style::default()
                    .bg(theme.selection_bg())
                    .fg(theme.lavender())
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶ ");

        if let Some(title) = &self.title {
            let block = Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme.border()))
                .title(title.as_str())
                .title_style(Style::default().fg(theme.mauve()).add_modifier(Modifier::BOLD));
            table = table.block(block);
        }

        frame.render_stateful_widget(table, area, &mut self.state);
    }
}
