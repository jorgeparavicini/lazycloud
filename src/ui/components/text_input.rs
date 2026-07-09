use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, Paragraph};

use crate::Theme;
use crate::ui::{Component, EventResult, Result};

pub enum TextInputEvent {
    Submitted(String),
    Cancelled,
}

pub struct TextInput {
    label: String,
    value: String,
    /// Cursor position as a character index into `value`.
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
        self.cursor = self.value.chars().count();
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

    fn char_count(&self) -> usize {
        self.value.chars().count()
    }

    /// Byte offset in `value` for a character index.
    fn byte_index_of(&self, char_idx: usize) -> usize {
        self.value
            .char_indices()
            .nth(char_idx)
            .map_or(self.value.len(), |(i, _)| i)
    }

    fn insert_char(&mut self, c: char) {
        let idx = self.byte_index_of(self.cursor);
        self.value.insert(idx, c);
        self.cursor += 1;
    }

    fn insert_str(&mut self, s: &str) {
        let idx = self.byte_index_of(self.cursor);
        self.value.insert_str(idx, s);
        self.cursor += s.chars().count();
    }

    fn delete_char_before_cursor(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
            let idx = self.byte_index_of(self.cursor);
            self.value.remove(idx);
        }
    }

    fn delete_char_at_cursor(&mut self) {
        if self.cursor < self.char_count() {
            let idx = self.byte_index_of(self.cursor);
            self.value.remove(idx);
        }
    }

    const fn move_cursor_left(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    fn move_cursor_right(&mut self) {
        if self.cursor < self.char_count() {
            self.cursor += 1;
        }
    }

    const fn move_cursor_start(&mut self) {
        self.cursor = 0;
    }

    fn move_cursor_end(&mut self) {
        self.cursor = self.char_count();
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
        let start = self.byte_index_of(pos);
        let end = self.byte_index_of(self.cursor);
        self.value.drain(start..end);
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

    fn handle_paste(&mut self, text: &str) -> Result<EventResult<Self::Output>> {
        // Normalize line endings and drop the trailing newline most clipboard
        // copies carry, so a paste never auto-submits or ends with stray
        // whitespace. Interior newlines are kept: the value must stay faithful
        // for payloads even though the input renders a single line.
        let text = text.replace("\r\n", "\n").replace('\r', "\n");
        self.insert_str(text.trim_end_matches('\n'));
        Ok(EventResult::Consumed)
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        // Calculate centered popup area - smaller for single input
        let popup_area = area.centered(Constraint::Percentage(50), Constraint::Length(5));

        // Clear the area behind the popup
        frame.render_widget(Clear, popup_area);

        // Build the display value
        let display_value = if self.masked {
            "*".repeat(self.char_count())
        } else {
            self.value.clone()
        };

        // Create the input line with cursor (split at a char boundary)
        let split_at = display_value
            .char_indices()
            .nth(self.cursor)
            .map_or(display_value.len(), |(i, _)| i);
        let (before_cursor, after_cursor) = display_value.split_at(split_at);

        let cursor_char = after_cursor.chars().next().unwrap_or(' ');
        let after_cursor_rest: String = after_cursor.chars().skip(1).collect();

        let input_style = Style::default().fg(theme.text());
        let cursor_style = Style::default()
            .fg(theme.bg())
            .bg(theme.text())
            .add_modifier(Modifier::BOLD);
        let placeholder_style = Style::default().fg(theme.overlay0());

        let line = if self.value.is_empty() {
            self.placeholder.as_ref().map_or_else(
                || Line::from(Span::styled(" ", cursor_style)),
                |placeholder| {
                    Line::from(vec![
                        Span::styled(" ", cursor_style),
                        Span::styled(placeholder.clone(), placeholder_style),
                    ])
                },
            )
        } else {
            Line::from(vec![
                Span::styled(before_cursor.to_string(), input_style),
                Span::styled(cursor_char.to_string(), cursor_style),
                Span::styled(after_cursor_rest, input_style),
            ])
        };

        let title = format!(" {} (Enter to confirm, Esc to cancel) ", self.label);
        let block = theme.popup_block(&title);

        let paragraph = Paragraph::new(line).block(block);

        frame.render_widget(paragraph, popup_area);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paste(input: &mut TextInput, text: &str) {
        input.handle_paste(text).unwrap();
    }

    #[test]
    fn paste_inserts_text_at_cursor() {
        let mut input = TextInput::new("test").with_value("ad");
        input.move_cursor_left();
        paste(&mut input, "bc");
        assert_eq!(input.value(), "abcd");
    }

    #[test]
    fn paste_strips_trailing_newline() {
        let mut input = TextInput::new("test");
        paste(&mut input, "my-secret-value\n");
        assert_eq!(input.value(), "my-secret-value");
        paste(&mut input, "\r\n");
        assert_eq!(input.value(), "my-secret-value");
    }

    #[test]
    fn paste_normalizes_crlf_but_keeps_interior_newlines() {
        let mut input = TextInput::new("test");
        paste(&mut input, "line1\r\nline2\r\n");
        assert_eq!(input.value(), "line1\nline2");
    }

    #[test]
    fn paste_multibyte_then_edit_does_not_panic() {
        let mut input = TextInput::new("test");
        paste(&mut input, "pässwörd-🔑");
        assert_eq!(input.value(), "pässwörd-🔑");
        input.delete_char_before_cursor();
        assert_eq!(input.value(), "pässwörd-");
        input.move_cursor_start();
        input.delete_char_at_cursor();
        input.insert_char('P');
        assert_eq!(input.value(), "Pässwörd-");
    }

    #[test]
    fn typing_multibyte_chars_uses_char_boundaries() {
        let mut input = TextInput::new("test");
        input.insert_char('é');
        input.insert_char('x');
        input.move_cursor_left();
        input.move_cursor_left();
        input.insert_char('a');
        assert_eq!(input.value(), "aéx");
    }
}
