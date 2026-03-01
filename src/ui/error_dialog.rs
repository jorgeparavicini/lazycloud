use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, Paragraph, Wrap};

use crate::Theme;
use crate::ui::{Component, EventResult, Result};

pub enum ErrorDialogEvent {
    Dismissed,
}

pub struct ErrorDialog {
    message: String,
}

impl ErrorDialog {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl Component for ErrorDialog {
    type Output = ErrorDialogEvent;

    fn handle_key(&mut self, key: KeyEvent) -> Result<EventResult<Self::Output>> {
        if matches!(key.code, KeyCode::Enter | KeyCode::Esc | KeyCode::Char('q')) {
            return Ok(ErrorDialogEvent::Dismissed.into());
        }
        Ok(EventResult::Consumed)
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let popup_area = area.centered(Constraint::Percentage(60), Constraint::Percentage(40));

        frame.render_widget(Clear, popup_area);

        let message_style = Style::default().fg(theme.text());
        let hint_style = Style::default().fg(theme.overlay1());

        let lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                "An unexpected error occurred",
                Style::default()
                    .fg(theme.error())
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(&self.message, message_style)),
            Line::from(""),
            Line::from(Span::styled(
                "Please report this issue to the developers",
                hint_style,
            )),
            Line::from(Span::styled(
                "The current service will be terminated",
                hint_style,
            )),
            Line::from(""),
            Line::from(Span::styled("Press Enter or Esc to dismiss", hint_style)),
        ];

        let block = theme
            .popup_block(" Error ")
            .title_style(
                Style::default()
                    .fg(theme.error())
                    .add_modifier(Modifier::BOLD),
            )
            .border_style(Style::default().fg(theme.error()));

        let paragraph = Paragraph::new(lines)
            .block(block)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });

        frame.render_widget(paragraph, popup_area);
    }
}
