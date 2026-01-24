use std::sync::Arc;

use crossterm::event::KeyEvent;
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap};

use crate::Theme;
use crate::config::{DialogAction, KeyResolver};
use crate::ui::{Component, EventResult, Result};

pub enum ErrorDialogEvent {
    Dismissed,
}

pub struct ErrorDialog {
    message: String,
    resolver: Arc<KeyResolver>,
}

impl ErrorDialog {
    pub fn new(message: impl Into<String>, resolver: Arc<KeyResolver>) -> Self {
        Self {
            message: message.into(),
            resolver,
        }
    }
}

impl Component for ErrorDialog {
    type Output = ErrorDialogEvent;

    fn handle_key(&mut self, key: KeyEvent) -> Result<EventResult<Self::Output>> {
        if self.resolver.matches_dialog(&key, DialogAction::Dismiss) {
            return Ok(ErrorDialogEvent::Dismissed.into());
        }
        Ok(EventResult::Consumed)
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let popup_area = area.centered(Constraint::Percentage(60), Constraint::Percentage(40));

        frame.render_widget(Clear, popup_area);

        let title_style = Style::default()
            .fg(theme.red())
            .add_modifier(Modifier::BOLD);
        let message_style = Style::default().fg(theme.text());
        let hint_style = Style::default().fg(theme.overlay1());

        let lines = vec![
            Line::from(""),
            Line::from(Span::styled(&self.message, message_style)),
            Line::from(""),
            Line::from(Span::styled("Press Enter or Esc to dismiss", hint_style)),
        ];

        let block = Block::default()
            .title(" Error ")
            .title_style(title_style)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.red()))
            .style(Style::default().bg(theme.base()));

        let paragraph = Paragraph::new(lines)
            .block(block)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });

        frame.render_widget(paragraph, popup_area);
    }
}
