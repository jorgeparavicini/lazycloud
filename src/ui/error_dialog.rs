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

/// Whether an error is a likely bug or an expected, user-actionable problem.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    /// A likely bug — worth reporting to the developers.
    Unexpected,
    /// An expected problem the user can act on (e.g. auth not configured).
    Expected,
}

pub struct ErrorDialog {
    kind: ErrorKind,
    message: String,
}

impl ErrorDialog {
    /// A dialog for an unexpected error (likely a bug).
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::Unexpected,
            message: message.into(),
        }
    }

    /// Mark this as an expected, user-actionable problem: it drops the
    /// "report to the developers" guidance and uses a calmer presentation.
    pub const fn expected(mut self) -> Self {
        self.kind = ErrorKind::Expected;
        self
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
        let popup_area = area.centered(Constraint::Percentage(65), Constraint::Percentage(50));

        frame.render_widget(Clear, popup_area);

        let message_style = Style::default().fg(theme.text());
        let hint_style = Style::default().fg(theme.overlay1());

        let (accent, title) = match self.kind {
            ErrorKind::Unexpected => (theme.error(), " Error "),
            ErrorKind::Expected => (theme.warning(), " Action Required "),
        };

        let mut lines: Vec<Line> = Vec::new();

        if self.kind == ErrorKind::Unexpected {
            lines.push(Line::from(Span::styled(
                "An unexpected error occurred",
                Style::default().fg(accent).add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(""));
        }

        // Preserve the message's own line breaks (e.g. an actionable hint on its
        // own line, an indented command) rather than collapsing to one blob.
        for message_line in self.message.split('\n') {
            lines.push(Line::from(Span::styled(
                message_line.to_string(),
                message_style,
            )));
        }
        lines.push(Line::from(""));

        if self.kind == ErrorKind::Unexpected {
            lines.push(Line::from(Span::styled(
                "Please report this issue to the developers",
                hint_style,
            )));
            lines.push(Line::from(Span::styled(
                "The current service will be terminated",
                hint_style,
            )));
            lines.push(Line::from(""));
        }

        lines.push(Line::from(Span::styled(
            "Press Enter or Esc to dismiss",
            hint_style,
        )));

        // Vertically center by padding the top with blank lines. `inner` height
        // excludes the two border rows.
        let inner_height = popup_area.height.saturating_sub(2) as usize;
        let top_pad = inner_height.saturating_sub(lines.len()) / 2;
        for _ in 0..top_pad {
            lines.insert(0, Line::from(""));
        }

        let block = theme
            .popup_block(title)
            .title_style(Style::default().fg(accent).add_modifier(Modifier::BOLD))
            .border_style(Style::default().fg(accent));

        // Split on the message's own newlines above so each part is a separate
        // centered line, rather than one blob wrapped across the width.
        let paragraph = Paragraph::new(lines)
            .block(block)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });

        frame.render_widget(paragraph, popup_area);
    }
}
