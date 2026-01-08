use crate::Theme;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame,
};

/// A keybinding entry for the help overlay.
pub struct Keybinding {
    pub key: &'static str,
    pub description: &'static str,
}

impl Keybinding {
    pub const fn new(key: &'static str, description: &'static str) -> Self {
        Self { key, description }
    }
}

/// Help overlay that displays keybindings in a centered popup.
pub struct HelpOverlay;

impl HelpOverlay {
    pub fn new() -> Self {
        Self
    }

    /// Handle a key event. Returns whether the overlay should close.
    pub fn handle_key_event(&mut self, key: KeyEvent) -> HelpOverlayEvent {
        match key.code {
            KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q') => HelpOverlayEvent::Close,
            _ => HelpOverlayEvent::None,
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, keybindings: &[Keybinding], theme: &Theme) {
        // Calculate centered popup area
        let popup_area = area.centered(Constraint::Percentage(60), Constraint::Percentage(70));

        // Clear the area behind the popup
        frame.render_widget(Clear, popup_area);

        // Build keybinding lines
        let key_style = Style::default()
            .fg(theme.peach())
            .add_modifier(Modifier::BOLD);
        let desc_style = Style::default().fg(theme.text());

        let lines: Vec<Line> = keybindings
            .iter()
            .map(|kb| {
                Line::from(vec![
                    Span::styled(format!("{:>12}", kb.key), key_style),
                    Span::raw("  "),
                    Span::styled(kb.description, desc_style),
                ])
            })
            .collect();

        let block = Block::default()
            .title(" Help (press ? or Esc to close) ")
            .title_style(Style::default().fg(theme.mauve()).add_modifier(Modifier::BOLD))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.lavender()))
            .style(Style::default().bg(theme.base()));

        let paragraph = Paragraph::new(lines).block(block);

        frame.render_widget(paragraph, popup_area);
    }
}

impl Default for HelpOverlay {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of handling a key event in the help overlay.
pub enum HelpOverlayEvent {
    /// No action taken
    None,
    /// User wants to close the overlay
    Close,
}
