use crate::theme::{available_themes, ThemeInfo};
use crate::view::{KeyResult, ListEvent, ListRow, ListView, View};
use crate::Theme;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Rect},
    style::{Modifier, Style},
    widgets::{Block, BorderType, Borders, Clear, ListItem},
    Frame,
};

impl ListRow for ThemeInfo {
    fn render_row(&self, theme: &Theme) -> ListItem<'static> {
        ListItem::new(self.name.to_string()).style(Style::default().fg(theme.text()))
    }
}

/// Event emitted by [`ThemeSelectorView`].
pub enum ThemeEvent {
    /// User cancelled selection.
    Cancelled,
    /// User selected a theme.
    Selected(Theme),
}

/// View for selecting the application theme.
pub struct ThemeSelectorView {
    list: ListView<ThemeInfo>,
}

impl ThemeSelectorView {
    pub fn new() -> Self {
        let themes = available_themes();
        Self {
            list: ListView::new(themes),
        }
    }
}

impl Default for ThemeSelectorView {
    fn default() -> Self {
        Self::new()
    }
}

impl View for ThemeSelectorView {
    type Event = ThemeEvent;

    fn handle_key(&mut self, key: KeyEvent) -> KeyResult<Self::Event> {
        // Handle escape/toggle to close
        if matches!(key.code, KeyCode::Esc | KeyCode::Char('t')) {
            return ThemeEvent::Cancelled.into();
        }

        // Delegate to list
        let result = self.list.handle_key(key);
        if let KeyResult::Event(ListEvent::Activated(info)) = result {
            return ThemeEvent::Selected(info.theme).into();
        }

        // Propagate consumed state from list
        if result.is_consumed() {
            KeyResult::Consumed
        } else {
            KeyResult::Ignored
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        // Calculate centered popup area
        let popup_area = area.centered(Constraint::Percentage(40), Constraint::Percentage(50));

        // Clear the area behind the popup
        frame.render_widget(Clear, popup_area);

        // Render block background
        let block = Block::default()
            .title(" Select Theme (Enter to confirm, Esc to cancel) ")
            .title_style(
                Style::default()
                    .fg(theme.mauve())
                    .add_modifier(Modifier::BOLD),
            )
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.lavender()))
            .style(Style::default().bg(theme.base()));

        let inner = block.inner(popup_area);
        frame.render_widget(block, popup_area);

        // Render the list inside
        self.list.render(frame, inner, theme);
    }
}
