use crate::ui::{Component, Handled, Result};
use crate::Theme;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::prelude::{Modifier, Style};
use ratatui::widgets::{Block, BorderType, Borders, Cell, Paragraph, Row, Table, TableState};
use ratatui::Frame;

pub enum TableEvent<T> {
    Changed(T),
    Activated(T),
    SearchChanged(String),
}

pub struct ColumnDef {
    pub header: &'static str,
    pub constraint: Constraint,
}

impl ColumnDef {
    pub const fn new(header: &'static str, constraint: Constraint) -> Self {
        Self { header, constraint }
    }
}

pub trait TableRow {
    fn columns() -> &'static [ColumnDef];
    fn render_cells(&self, theme: &Theme) -> Vec<Cell<'static>>;

    /// Override to render cells differently based on the current search query.
    fn render_cells_with_query(&self, theme: &Theme, query: &str) -> Vec<Cell<'static>> {
        _ = query;
        self.render_cells(theme)
    }

    /// Return true if this row matches the search query for local filtering.
    fn matches(&self, query: &str) -> bool;
}

pub struct TableComponent<T: TableRow + Clone> {
    items: Vec<T>,
    filtered_indices: Vec<usize>,
    state: TableState,
    title: Option<String>,
    searching: bool,
    query: String,
}

impl<T: TableRow + Clone> TableComponent<T> {
    pub fn new(items: Vec<T>) -> Self {
        let filtered_indices: Vec<usize> = (0..items.len()).collect();
        let mut state = TableState::default();
        if !filtered_indices.is_empty() {
            state.select(Some(0));
        }
        Self {
            items,
            filtered_indices,
            state,
            title: None,
            searching: false,
            query: String::new(),
        }
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn selected_item(&self) -> Option<&T> {
        if let Some(selected) = self.state.selected() {
            if let Some(&idx) = self.filtered_indices.get(selected) {
                return self.items.get(idx);
            }
        }
        None
    }

    fn update_filter(&mut self) {
        self.filtered_indices = self
            .items
            .iter()
            .enumerate()
            .filter(|(_, item)| self.query.is_empty() || item.matches(&self.query))
            .map(|(i, _)| i)
            .collect();

        // Reset selection to first item if current selection is invalid
        if self.filtered_indices.is_empty() {
            self.state.select(None);
        } else if self.state.selected().map_or(true, |i| i >= self.filtered_indices.len()) {
            self.state.select(Some(0));
        }
    }

    fn select_next(&mut self) {
        if self.filtered_indices.is_empty() {
            return;
        }
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.filtered_indices.len() - 1 {
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
        if self.filtered_indices.is_empty() {
            return;
        }
        let i = match self.state.selected() {
            Some(i) => i.saturating_sub(1),
            None => 0,
        };
        self.state.select(Some(i));
    }

    fn select_first(&mut self) {
        if !self.filtered_indices.is_empty() {
            self.state.select(Some(0));
        }
    }

    fn select_last(&mut self) {
        if !self.filtered_indices.is_empty() {
            self.state.select(Some(self.filtered_indices.len() - 1));
        }
    }

    fn get_change_event(&self, before: Option<usize>) -> Handled<TableEvent<T>> {
        if let Some(selected) = self.state.selected() {
            if Some(selected) != before {
                if let Some(&idx) = self.filtered_indices.get(selected) {
                    return TableEvent::Changed(self.items[idx].clone()).into();
                }
            }
        }
        Handled::Consumed
    }

    fn handle_search_key(&mut self, key: KeyEvent) -> Result<Handled<TableEvent<T>>> {
        Ok(match key.code {
            KeyCode::Esc => {
                // Exit search mode and clear filter
                self.searching = false;
                let had_query = !self.query.is_empty();
                self.query.clear();
                self.update_filter();
                if had_query {
                    TableEvent::SearchChanged(String::new()).into()
                } else {
                    Handled::Consumed
                }
            }
            KeyCode::Enter => {
                // Exit search mode but keep filter
                self.searching = false;
                Handled::Consumed
            }
            KeyCode::Backspace => {
                self.query.pop();
                self.update_filter();
                TableEvent::SearchChanged(self.query.clone()).into()
            }
            KeyCode::Char(c) => {
                self.query.push(c);
                self.update_filter();
                TableEvent::SearchChanged(self.query.clone()).into()
            }
            // Consume all other keys in search mode
            _ => Handled::Consumed,
        })
    }

    fn handle_navigation_key(&mut self, key: KeyEvent) -> Result<Handled<TableEvent<T>>> {
        let before = self.state.selected();

        Ok(match key.code {
            KeyCode::Down | KeyCode::Char('j') => {
                self.select_next();
                self.get_change_event(before)
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.select_previous();
                self.get_change_event(before)
            }
            KeyCode::Home | KeyCode::Char('g') => {
                self.select_first();
                self.get_change_event(before)
            }
            KeyCode::End | KeyCode::Char('G') => {
                self.select_last();
                self.get_change_event(before)
            }
            KeyCode::PageDown => {
                let step = 10;
                let new_index = match self.state.selected() {
                    Some(i) if !self.filtered_indices.is_empty() => {
                        usize::min(i + step, self.filtered_indices.len() - 1)
                    }
                    _ => 0,
                };
                if !self.filtered_indices.is_empty() {
                    self.state.select(Some(new_index));
                }
                self.get_change_event(before)
            }
            KeyCode::PageUp => {
                let step = 10;
                let new_index = match self.state.selected() {
                    Some(i) => i.saturating_sub(step),
                    None => 0,
                };
                if !self.filtered_indices.is_empty() {
                    self.state.select(Some(new_index));
                }
                self.get_change_event(before)
            }
            KeyCode::Enter => {
                if let Some(selected) = self.state.selected() {
                    self.filtered_indices
                        .get(selected)
                        .map(|&idx| TableEvent::Activated(self.items[idx].clone()).into())
                        .unwrap_or(Handled::Ignored)
                } else {
                    Handled::Ignored
                }
            }
            KeyCode::Char('/') => {
                self.searching = true;
                Handled::Consumed
            }
            KeyCode::Esc if !self.query.is_empty() => {
                // Clear filter when not searching
                self.query.clear();
                self.update_filter();
                Handled::Consumed
            }
            _ => Handled::Ignored,
        })
    }
}

impl<T: TableRow + Clone> Component for TableComponent<T> {
    type Output = TableEvent<T>;

    fn handle_key(&mut self, key: KeyEvent) -> Result<Handled<Self::Output>> {
        if self.searching {
            self.handle_search_key(key)
        } else {
            self.handle_navigation_key(key)
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        // If searching or has active filter, reserve space for search bar
        let has_search_bar = self.searching || !self.query.is_empty();
        let (table_area, search_area) = if has_search_bar {
            let chunks = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).split(area);
            (chunks[0], Some(chunks[1]))
        } else {
            (area, None)
        };

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
            .filtered_indices
            .iter()
            .map(|&idx| {
                Row::new(self.items[idx].render_cells_with_query(theme, &self.query))
                    .style(Style::default().fg(theme.text()))
            })
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

        frame.render_stateful_widget(table, table_area, &mut self.state);

        // Render search bar if needed
        if let Some(search_area) = search_area {
            let search_text = if self.searching {
                format!("/{}_", self.query)
            } else {
                format!("/{} ({} matches)", self.query, self.filtered_indices.len())
            };

            let search_style = if self.searching {
                Style::default().fg(theme.yellow())
            } else {
                Style::default().fg(theme.subtext0())
            };

            let search_bar = Paragraph::new(search_text).style(search_style);
            frame.render_widget(search_bar, search_area);
        }
    }
}
