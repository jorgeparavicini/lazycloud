use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph};

use crate::Theme;
use crate::ui::{Component, EventResult, Result};

pub enum TextInputEvent {
    Submitted(String),
    Cancelled,
}

pub struct TextInput {
    label: String,
    value: String,
    cursor: usize,
    placeholder: Option<String>,
    masked: bool,
}

impl TextInput {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            value: String::new(),
            cursor: 0,
            placeholder: None,
            masked: false,
        }
    }

    #[allow(dead_code)]
    pub fn with_value(mut self, value: impl Into<String>) -> Self {
        self.value = value.into();
        self.cursor = self.value.len();
        self
    }

    pub fn with_placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    #[allow(dead_code)]
    pub const fn masked(mut self) -> Self {
        self.masked = true;
        self
    }

    pub fn value(&self) -> &str {
        &self.value
    }

    fn insert_char(&mut self, c: char) {
        self.value.insert(self.cursor, c);
        self.cursor += 1;
    }

    fn delete_char_before_cursor(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
            self.value.remove(self.cursor);
        }
    }

    fn delete_char_at_cursor(&mut self) {
        if self.cursor < self.value.len() {
            self.value.remove(self.cursor);
        }
    }

    const fn move_cursor_left(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    const fn move_cursor_right(&mut self) {
        if self.cursor < self.value.len() {
            self.cursor += 1;
        }
    }

    const fn move_cursor_start(&mut self) {
        self.cursor = 0;
    }

    const fn move_cursor_end(&mut self) {
        self.cursor = self.value.len();
    }

    fn delete_word_before_cursor(&mut self) {
        // Find the start of the previous word
        let mut pos = self.cursor;
        // Skip trailing spaces
        while pos > 0 && self.value.chars().nth(pos - 1) == Some(' ') {
            pos -= 1;
        }
        // Skip word characters
        while pos > 0 && self.value.chars().nth(pos - 1) != Some(' ') {
            pos -= 1;
        }
        // Delete from pos to cursor
        self.value.drain(pos..self.cursor);
        self.cursor = pos;
    }

    fn clear_line(&mut self) {
        self.value.clear();
        self.cursor = 0;
    }
}

impl Component for TextInput {
    type Output = TextInputEvent;

    fn handle_key(&mut self, key: KeyEvent) -> Result<EventResult<Self::Output>> {
        Ok(match (key.code, key.modifiers) {
            // Submit
            (KeyCode::Enter, _) => TextInputEvent::Submitted(self.value.clone()).into(),

            // Cancel
            (KeyCode::Esc, _) => TextInputEvent::Cancelled.into(),

            // Delete
            (KeyCode::Backspace, KeyModifiers::ALT) => {
                self.delete_word_before_cursor();
                EventResult::Consumed
            }
            (KeyCode::Backspace, _) => {
                self.delete_char_before_cursor();
                EventResult::Consumed
            }
            (KeyCode::Delete, _) => {
                self.delete_char_at_cursor();
                EventResult::Consumed
            }

            // Navigation
            (KeyCode::Left, _) => {
                self.move_cursor_left();
                EventResult::Consumed
            }
            (KeyCode::Right, _) => {
                self.move_cursor_right();
                EventResult::Consumed
            }
            (KeyCode::Home, _) | (KeyCode::Char('a'), KeyModifiers::CONTROL) => {
                self.move_cursor_start();
                EventResult::Consumed
            }
            (KeyCode::End, _) | (KeyCode::Char('e'), KeyModifiers::CONTROL) => {
                self.move_cursor_end();
                EventResult::Consumed
            }

            // Clear line
            (KeyCode::Char('u'), KeyModifiers::CONTROL) => {
                self.clear_line();
                EventResult::Consumed
            }

            // Character input
            (KeyCode::Char(c), KeyModifiers::NONE | KeyModifiers::SHIFT) => {
                self.insert_char(c);
                EventResult::Consumed
            }

            _ => EventResult::Consumed, // Consume all keys to prevent propagation
        })
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        // Calculate centered popup area - smaller for single input
        let popup_area = area.centered(Constraint::Percentage(50), Constraint::Length(5));

        // Clear the area behind the popup
        frame.render_widget(Clear, popup_area);

        // Build the display value
        let display_value = if self.masked {
            "*".repeat(self.value.len())
        } else {
            self.value.clone()
        };

        // Create the input line with cursor
        let (before_cursor, after_cursor) = display_value.split_at(if self.masked {
            self.cursor
        } else {
            self.cursor.min(display_value.len())
        });

        let cursor_char = after_cursor.chars().next().unwrap_or(' ');
        let after_cursor_rest: String = after_cursor.chars().skip(1).collect();

        let input_style = Style::default().fg(theme.text());
        let cursor_style = Style::default()
            .fg(theme.base())
            .bg(theme.text())
            .add_modifier(Modifier::BOLD);
        let placeholder_style = Style::default().fg(theme.overlay0());

        let line = if self.value.is_empty() && self.placeholder.is_some() {
            // Show placeholder with cursor at start
            Line::from(vec![
                Span::styled(" ", cursor_style),
                Span::styled(
                    self.placeholder.as_ref().unwrap().clone(),
                    placeholder_style,
                ),
            ])
        } else {
            Line::from(vec![
                Span::styled(before_cursor.to_string(), input_style),
                Span::styled(cursor_char.to_string(), cursor_style),
                Span::styled(after_cursor_rest, input_style),
            ])
        };

        let title = format!(" {} (Enter to confirm, Esc to cancel) ", self.label);
        let block = Block::default()
            .title(title)
            .title_style(
                Style::default()
                    .fg(theme.mauve())
                    .add_modifier(Modifier::BOLD),
            )
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.lavender()))
            .style(Style::default().bg(theme.base()));

        let paragraph = Paragraph::new(line).block(block);

        frame.render_widget(paragraph, popup_area);
    }
}
