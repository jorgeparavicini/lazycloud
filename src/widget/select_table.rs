use crossterm::event::KeyEvent;
use ratatui::layout::{Constraint, Rect};
use ratatui::prelude::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, Cell, Row, Table, TableState};
use ratatui::Frame;

pub enum TableEvent<'a, T> {
    Changed(&'a T),
    Activated(&'a T),
}

/// Column definition for a table.
pub struct Column<'a> {
    pub header: &'a str,
    pub constraint: Constraint,
}

impl<'a> Column<'a> {
    pub fn new(header: &'a str, constraint: Constraint) -> Self {
        Self { header, constraint }
    }
}

/// A selectable table widget with keyboard navigation.
pub struct SelectTable<T> {
    items: Vec<T>,
    state: TableState,
    title: Option<String>,
}

impl<T> SelectTable<T> {
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

    pub fn handle_key_event(&mut self, key: KeyEvent) -> Option<TableEvent<'_, T>> {
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
                    Some(TableEvent::Activated(&self.items[selected]))
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

    /// Render the table with the given columns and row renderer.
    pub fn render<'a, F>(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        columns: &[Column<'a>],
        row_renderer: F,
    ) where
        F: Fn(&T) -> Vec<String>,
    {
        let header_cells: Vec<Cell> = columns
            .iter()
            .map(|c| {
                Cell::from(c.header).style(
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )
            })
            .collect();
        let header = Row::new(header_cells).height(1);

        let rows: Vec<Row> = self
            .items
            .iter()
            .map(|item| {
                let cells: Vec<Cell> = row_renderer(item)
                    .into_iter()
                    .map(Cell::from)
                    .collect();
                Row::new(cells)
            })
            .collect();

        let widths: Vec<Constraint> = columns.iter().map(|c| c.constraint).collect();

        let mut table = Table::new(rows, widths)
            .header(header)
            .row_highlight_style(
                Style::default()
                    .bg(Color::Blue)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶ ");

        if let Some(title) = &self.title {
            let block = Block::default()
                .borders(Borders::ALL)
                .title(title.as_str());
            table = table.block(block);
        }

        frame.render_stateful_widget(table, area, &mut self.state);
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

    fn get_change_event(&self, before: Option<usize>) -> Option<TableEvent<'_, T>> {
        if let Some(selected) = self.state.selected() {
            if Some(selected) != before {
                return Some(TableEvent::Changed(&self.items[selected]));
            }
        }
        None
    }
}
