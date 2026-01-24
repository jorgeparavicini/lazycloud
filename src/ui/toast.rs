use std::collections::VecDeque;
use std::time::{Duration, Instant};

use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use crate::Theme;
use super::{Component, EventResult, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastType {
    Success,
    Info,
}

pub struct Toast {
    message: String,
    toast_type: ToastType,
    created_at: Instant,
    duration: Duration,
}

impl Toast {
    pub fn new(message: impl Into<String>, toast_type: ToastType) -> Self {
        Self {
            message: message.into(),
            toast_type,
            created_at: Instant::now(),
            duration: Duration::from_secs(3),
        }
    }

    pub fn success(message: impl Into<String>) -> Self {
        Self::new(message, ToastType::Success)
    }

    pub fn info(message: impl Into<String>) -> Self {
        Self::new(message, ToastType::Info)
    }

    fn is_expired(&self) -> bool {
        self.created_at.elapsed() >= self.duration
    }
}

pub struct ToastManager {
    toasts: VecDeque<Toast>,
    max_visible: usize,
}

impl Default for ToastManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ToastManager {
    pub fn new() -> Self {
        Self {
            toasts: VecDeque::new(),
            max_visible: 3,
        }
    }

    pub fn show(&mut self, toast: Toast) {
        self.toasts.push_back(toast);
        // Keep only max_visible toasts
        while self.toasts.len() > self.max_visible {
            self.toasts.pop_front();
        }
    }
}

impl Component for ToastManager {
    type Output = ();

    fn handle_key(
        &mut self,
        _key: crossterm::event::KeyEvent,
    ) -> Result<EventResult<Self::Output>> {
        Ok(EventResult::Ignored)
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        if self.toasts.is_empty() {
            return;
        }

        let toast_height = 3u16;
        let toast_width = 50u16.min(area.width.saturating_sub(4));
        let spacing = 1u16;

        // Stack toasts from bottom-right, going upward
        let visible_toasts: Vec<_> = self.toasts.iter().collect();

        for (i, toast) in visible_toasts.iter().enumerate() {
            let y_offset = (i as u16) * (toast_height + spacing);
            let y = area.y + area.height.saturating_sub(toast_height + y_offset + 1);
            let x = area.x + area.width.saturating_sub(toast_width + 2);

            if y < area.y {
                break; // No more room
            }

            let toast_area = Rect::new(x, y, toast_width, toast_height);

            let (border_color, icon) = match toast.toast_type {
                ToastType::Success => (theme.green(), "✓"),
                ToastType::Info => (theme.blue(), "ℹ"),
            };

            frame.render_widget(Clear, toast_area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color))
                .style(Style::default().bg(theme.surface0()));

            let inner = block.inner(toast_area);
            frame.render_widget(block, toast_area);

            // Center content vertically and horizontally
            let content_area = Layout::default()
                .constraints([Constraint::Fill(1)])
                .split(inner)[0];

            let text = format!("{} {}", icon, toast.message);
            let paragraph = Paragraph::new(text)
                .style(
                    Style::default()
                        .fg(theme.text())
                        .add_modifier(Modifier::BOLD),
                )
                .alignment(Alignment::Center);

            frame.render_widget(paragraph, content_area);
        }
    }

    fn handle_tick(&mut self) {
        self.toasts.retain(|t| !t.is_expired());
    }
}
