//! Theme selector widget for choosing application themes.

use crate::theme::{available_themes, ThemeInfo};
use crate::widget::{ListEvent, SelectList};
use crate::Theme;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Rect},
    style::{Modifier, Style},
    widgets::{Block, BorderType, Borders, Clear},
    Frame,
};

/// Widget for selecting the application theme.
pub struct ThemeSelector {
    list: SelectList<ThemeInfo>,
}

impl ThemeSelector {
    pub fn new() -> Self {
        let themes = available_themes();
        Self {
            list: SelectList::new(themes),
        }
    }

    /// Handle a key event. Returns the selected theme if Enter was pressed,
    /// or None if Esc/t was pressed to cancel.
    pub fn handle_key_event(&mut self, key: KeyEvent) -> ThemeSelectorEvent {
        // Handle escape/toggle to close
        if matches!(key.code, KeyCode::Esc | KeyCode::Char('t')) {
            return ThemeSelectorEvent::Cancelled;
        }

        // Delegate to list
        if let Some(ListEvent::Activated(info)) = self.list.handle_key_event(key) {
            return ThemeSelectorEvent::Selected(info.theme);
        }

        ThemeSelectorEvent::None
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
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

impl Default for ThemeSelector {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of handling a key event in the theme selector.
pub enum ThemeSelectorEvent {
    /// No action taken
    None,
    /// User cancelled selection
    Cancelled,
    /// User selected a theme
    Selected(Theme),
}
