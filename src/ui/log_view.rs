use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, Paragraph};
use tracing::Level;

use crate::Theme;
use crate::logging::LogBuffer;
use crate::ui::{Component, EventResult, Result};

pub enum LogViewEvent {
    Close,
}

/// A scrollable, auto-tailing overlay that displays captured logs live.
pub struct LogView {
    buffer: LogBuffer,
    /// Index of the topmost visible line.
    offset: usize,
    /// When true, the view sticks to the newest logs as they arrive.
    follow: bool,
    /// Largest valid offset from the last render, used to clamp scrolling.
    last_max_offset: usize,
}

impl LogView {
    #[must_use]
    pub const fn new(buffer: LogBuffer) -> Self {
        Self {
            buffer,
            offset: 0,
            follow: true,
            last_max_offset: 0,
        }
    }

    const fn scroll_up(&mut self, amount: usize) {
        self.follow = false;
        self.offset = self.offset.saturating_sub(amount);
    }

    fn scroll_down(&mut self, amount: usize) {
        self.offset = (self.offset + amount).min(self.last_max_offset);
        // Reaching the bottom re-engages tailing.
        if self.offset >= self.last_max_offset {
            self.follow = true;
        }
    }

    fn level_style(theme: &Theme, level: Level) -> Style {
        let color = match level {
            Level::ERROR => theme.error(),
            Level::WARN => theme.warning(),
            Level::INFO => theme.info(),
            Level::DEBUG => theme.success(),
            Level::TRACE => theme.overlay1(),
        };
        Style::default().fg(color)
    }
}

impl Component for LogView {
    type Output = LogViewEvent;

    fn handle_key(&mut self, key: KeyEvent) -> Result<EventResult<Self::Output>> {
        Ok(match key.code {
            KeyCode::Esc | KeyCode::Char('q' | 'L') => LogViewEvent::Close.into(),
            KeyCode::Char('k') | KeyCode::Up => {
                self.scroll_up(1);
                EventResult::Consumed
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.scroll_down(1);
                EventResult::Consumed
            }
            KeyCode::PageUp => {
                self.scroll_up(10);
                EventResult::Consumed
            }
            KeyCode::PageDown => {
                self.scroll_down(10);
                EventResult::Consumed
            }
            KeyCode::Char('g') | KeyCode::Home => {
                self.follow = false;
                self.offset = 0;
                EventResult::Consumed
            }
            KeyCode::Char('G') | KeyCode::End => {
                self.follow = true;
                EventResult::Consumed
            }
            _ => EventResult::Consumed,
        })
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let popup_area = area.centered(Constraint::Percentage(85), Constraint::Percentage(80));
        frame.render_widget(Clear, popup_area);

        let entries = self.buffer.snapshot();

        let time_style = Style::default().fg(theme.overlay1());
        let target_style = Style::default().fg(theme.text_muted());
        let msg_style = Style::default().fg(theme.text());

        let lines: Vec<Line> = if entries.is_empty() {
            vec![Line::from(Span::styled(
                "No logs captured yet.",
                Style::default().fg(theme.overlay1()),
            ))]
        } else {
            entries
                .iter()
                .map(|entry| {
                    Line::from(vec![
                        Span::styled(entry.timestamp.format("%H:%M:%S%.3f").to_string(), time_style),
                        Span::raw(" "),
                        Span::styled(
                            format!("{:>5}", entry.level.as_str()),
                            Self::level_style(theme, entry.level),
                        ),
                        Span::raw(" "),
                        Span::styled(format!("{}: ", entry.target), target_style),
                        Span::styled(entry.message.clone(), msg_style),
                    ])
                })
                .collect()
        };

        let follow_hint = if self.follow { "tailing" } else { "paused" };
        let title = format!(
            " Logs ({} entries, {follow_hint}) — j/k scroll · G tail · Esc/L close ",
            entries.len()
        );
        let block = theme.popup_block(&title);

        // Compute the viewport so we can clamp the offset and auto-tail.
        let inner_height = block.inner(popup_area).height as usize;
        let max_offset = lines.len().saturating_sub(inner_height);
        self.last_max_offset = max_offset;
        if self.follow {
            self.offset = max_offset;
        } else {
            self.offset = self.offset.min(max_offset);
        }

        let scroll_y = u16::try_from(self.offset).unwrap_or(u16::MAX);
        let paragraph = Paragraph::new(lines).block(block).scroll((scroll_y, 0));

        frame.render_widget(paragraph, popup_area);
    }
}
