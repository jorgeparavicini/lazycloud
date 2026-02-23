use std::sync::Arc;

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, Paragraph};

use crate::Theme;
use crate::config::{DialogAction, KeyResolver};
use crate::ui::{Component, EventResult, Result};

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

#[derive(Default, Clone, Copy, PartialEq, Eq)]
enum Focus {
    #[default]
    Confirm,
    Cancel,
}

impl Focus {
    const fn toggle(self) -> Self {
        match self {
            Self::Confirm => Self::Cancel,
            Self::Cancel => Self::Confirm,
        }
    }
}

pub struct ConfirmDialog {
    title: String,
    message: String,
    confirm_text: String,
    cancel_text: String,
    style: ConfirmStyle,
    focus: Focus,
    resolver: Arc<KeyResolver>,
}

impl ConfirmDialog {
    pub fn new(message: impl Into<String>, resolver: Arc<KeyResolver>) -> Self {
        Self {
            title: "Confirm".to_string(),
            message: message.into(),
            confirm_text: "Yes".to_string(),
            cancel_text: "No".to_string(),
            style: ConfirmStyle::Normal,
            focus: Focus::Confirm,
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

    pub const fn danger(mut self) -> Self {
        self.style = ConfirmStyle::Danger;
        self.focus = Focus::Cancel;
        self
    }
}

impl Component for ConfirmDialog {
    type Output = ConfirmEvent;

    fn handle_key(&mut self, key: KeyEvent) -> Result<EventResult<Self::Output>> {
        // Focus navigation
        match key.code {
            KeyCode::Left | KeyCode::Right | KeyCode::Tab | KeyCode::BackTab => {
                self.focus = self.focus.toggle();
                return Ok(EventResult::Consumed);
            }
            KeyCode::Enter => {
                return Ok(match self.focus {
                    Focus::Confirm => ConfirmEvent::Confirmed.into(),
                    Focus::Cancel => ConfirmEvent::Cancelled.into(),
                });
            }
            _ => {}
        }

        // Direct hotkeys
        if self.resolver.matches_dialog(&key, DialogAction::Confirm) {
            return Ok(ConfirmEvent::Confirmed.into());
        }
        if self.resolver.matches_dialog(&key, DialogAction::Cancel) {
            return Ok(ConfirmEvent::Cancelled.into());
        }
        // Consume all other keys to prevent propagation
        Ok(EventResult::Consumed)
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        // Calculate centered popup area
        let popup_area = area.centered(Constraint::Percentage(50), Constraint::Length(7));

        // Clear the area behind the popup
        frame.render_widget(Clear, popup_area);

        // Choose colors based on style
        let (title_color, border_color, confirm_color) = match self.style {
            ConfirmStyle::Normal => (theme.secondary(), theme.highlight(), theme.success()),
            ConfirmStyle::Danger => (theme.error(), theme.error(), theme.error()),
        };

        // Build the content
        let message_style = Style::default().fg(theme.text());
        let key_style = theme.key_style();
        let dim_key_style = Style::default().fg(theme.overlay0());

        let focused_confirm = self.focus == Focus::Confirm;

        // Focused button: bg highlight + bold text
        // Unfocused button: dimmed
        let confirm_key_style = if focused_confirm { key_style } else { dim_key_style };
        let confirm_style = if focused_confirm {
            Style::default()
                .fg(confirm_color)
                .bg(theme.surface2())
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.overlay0())
        };

        let cancel_key_style = if focused_confirm { dim_key_style } else { key_style };
        let cancel_style = if focused_confirm {
            Style::default().fg(theme.overlay0())
        } else {
            Style::default()
                .fg(theme.text())
                .bg(theme.surface2())
                .add_modifier(Modifier::BOLD)
        };

        let lines = vec![
            Line::from(""),
            Line::from(Span::styled(self.message.clone(), message_style)),
            Line::from(""),
            Line::from(vec![
                Span::styled("[y]", confirm_key_style),
                Span::raw(" "),
                Span::styled(format!(" {} ", self.confirm_text), confirm_style),
                Span::raw("    "),
                Span::styled("[n]", cancel_key_style),
                Span::raw(" "),
                Span::styled(format!(" {} ", self.cancel_text), cancel_style),
            ]),
        ];

        let title = format!(" {} ", self.title);
        let block = theme.popup_block(&title)
            .title_style(
                Style::default()
                    .fg(title_color)
                    .add_modifier(Modifier::BOLD),
            )
            .border_style(Style::default().fg(border_color));

        let paragraph = Paragraph::new(lines)
            .block(block)
            .alignment(Alignment::Center);

        frame.render_widget(paragraph, popup_area);
    }
}
