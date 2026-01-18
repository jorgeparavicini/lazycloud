use crate::config::{DialogAction, KeyResolver};
use crate::ui::{Component, Handled, Result};
use crate::Theme;
use crossterm::event::KeyEvent;
use ratatui::{
    layout::{Alignment, Constraint, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame,
};
use std::sync::Arc;

pub enum ConfirmEvent {
    Confirmed,
    Cancelled,
}

#[derive(Default, Clone, Copy)]
pub enum ConfirmStyle {
    #[default]
    Normal,
    /// Shows red warning styling.
    Danger,
}

pub struct ConfirmDialogComponent {
    title: String,
    message: String,
    confirm_text: String,
    cancel_text: String,
    style: ConfirmStyle,
    resolver: Arc<KeyResolver>,
}

impl ConfirmDialogComponent {
    pub fn new(message: impl Into<String>, resolver: Arc<KeyResolver>) -> Self {
        Self {
            title: "Confirm".to_string(),
            message: message.into(),
            confirm_text: "Yes".to_string(),
            cancel_text: "No".to_string(),
            style: ConfirmStyle::Normal,
            resolver,
        }
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    pub fn with_confirm_text(mut self, text: impl Into<String>) -> Self {
        self.confirm_text = text.into();
        self
    }

    pub fn with_cancel_text(mut self, text: impl Into<String>) -> Self {
        self.cancel_text = text.into();
        self
    }

    pub fn danger(mut self) -> Self {
        self.style = ConfirmStyle::Danger;
        self
    }
}

impl Component for ConfirmDialogComponent {
    type Output = ConfirmEvent;

    fn handle_key(&mut self, key: KeyEvent) -> Result<Handled<Self::Output>> {
        if self.resolver.matches_dialog(&key, DialogAction::Confirm) {
            return Ok(ConfirmEvent::Confirmed.into());
        }
        if self.resolver.matches_dialog(&key, DialogAction::Cancel) {
            return Ok(ConfirmEvent::Cancelled.into());
        }
        // Consume all other keys to prevent propagation
        Ok(Handled::Consumed)
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
