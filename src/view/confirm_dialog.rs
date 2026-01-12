use crate::view::{KeyResult, View};
use crate::Theme;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Alignment, Constraint, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame,
};

/// Event emitted by [`ConfirmDialog`].
pub enum ConfirmEvent {
    /// User confirmed the action.
    Confirmed,
    /// User canceled the action.
    Cancelled,
}

/// Style/severity of the confirmation dialog.
#[derive(Default, Clone, Copy)]
pub enum ConfirmStyle {
    /// Normal confirmation (neutral color).
    #[default]
    Normal,
    /// Dangerous/destructive action (red warning).
    Danger,
}

/// A confirmation dialog popup view.
pub struct ConfirmDialog {
    /// The title for the dialog.
    title: String,
    /// The message to display.
    message: String,
    /// The confirm button text.
    confirm_text: String,
    /// The cancel button text.
    cancel_text: String,
    /// Style of the dialog.
    style: ConfirmStyle,
}

impl ConfirmDialog {
    /// Create a new confirmation dialog with the given message.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            title: "Confirm".to_string(),
            message: message.into(),
            confirm_text: "Yes".to_string(),
            cancel_text: "No".to_string(),
            style: ConfirmStyle::Normal,
        }
    }

    /// Set the dialog title.
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// Set custom confirm button text.
    pub fn with_confirm_text(mut self, text: impl Into<String>) -> Self {
        self.confirm_text = text.into();
        self
    }

    /// Set custom cancel button text.
    pub fn with_cancel_text(mut self, text: impl Into<String>) -> Self {
        self.cancel_text = text.into();
        self
    }

    /// Set the dialog style to dangerous (red warning).
    pub fn danger(mut self) -> Self {
        self.style = ConfirmStyle::Danger;
        self
    }
}

impl View for ConfirmDialog {
    type Event = ConfirmEvent;

    fn handle_key(&mut self, key: KeyEvent) -> KeyResult<Self::Event> {
        match key.code {
            // Confirm
            KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                ConfirmEvent::Confirmed.into()
            }

            // Cancel
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                ConfirmEvent::Cancelled.into()
            }

            // Consume all other keys to prevent propagation
            _ => KeyResult::Consumed,
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        // Calculate centered popup area
        let popup_area = area.centered(Constraint::Percentage(50), Constraint::Length(7));

        // Clear the area behind the popup
        frame.render_widget(Clear, popup_area);

        // Choose colors based on style
        let (title_color, border_color, confirm_color) = match self.style {
            ConfirmStyle::Normal => (theme.mauve(), theme.lavender(), theme.green()),
            ConfirmStyle::Danger => (theme.red(), theme.red(), theme.red()),
        };

        // Build the content
        let message_style = Style::default().fg(theme.text());
        let key_style = Style::default()
            .fg(theme.peach())
            .add_modifier(Modifier::BOLD);
        let confirm_style = Style::default()
            .fg(confirm_color)
            .add_modifier(Modifier::BOLD);
        let cancel_style = Style::default()
            .fg(theme.overlay1())
            .add_modifier(Modifier::BOLD);

        let lines = vec![
            Line::from(""),
            Line::from(Span::styled(self.message.clone(), message_style)),
            Line::from(""),
            Line::from(vec![
                Span::styled("[y]", key_style),
                Span::raw(" "),
                Span::styled(self.confirm_text.clone(), confirm_style),
                Span::raw("    "),
                Span::styled("[n]", key_style),
                Span::raw(" "),
                Span::styled(self.cancel_text.clone(), cancel_style),
            ]),
        ];

        let title = format!(" {} ", self.title);
        let block = Block::default()
            .title(title)
            .title_style(Style::default().fg(title_color).add_modifier(Modifier::BOLD))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(border_color))
            .style(Style::default().bg(theme.base()));

        let paragraph = Paragraph::new(lines)
            .block(block)
            .alignment(Alignment::Center);

        frame.render_widget(paragraph, popup_area);
    }
}
