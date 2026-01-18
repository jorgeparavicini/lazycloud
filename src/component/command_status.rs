use crate::Theme;
use crate::ui::Component;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
};
use std::collections::VecDeque;
use std::time::{Duration, Instant};
use throbber_widgets_tui::{BRAILLE_SIX, Throbber, ThrobberState, WhichUse};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CommandId(u64);

#[derive(Debug)]
struct RunningCommand {
    id: CommandId,
    name: String,
    started_at: Instant,
}

#[derive(Debug)]
struct CompletedCommand {
    name: String,
    success: bool,
    duration: Duration,
    completed_at: Instant,
}

pub struct CommandStatusView {
    running: Vec<RunningCommand>,
    history: VecDeque<CompletedCommand>,
    next_id: u64,
    max_history: usize,
    expanded: bool,
    throbber_state: ThrobberState,
}

impl CommandStatusView {
    pub fn new() -> Self {
        Self {
            running: Vec::new(),
            history: VecDeque::new(),
            next_id: 0,
            max_history: 10,
            expanded: false,
            throbber_state: ThrobberState::default(),
        }
    }

    pub fn start(&mut self, name: String) -> CommandId {
        let id = CommandId(self.next_id);
        self.next_id += 1;
        self.running.push(RunningCommand {
            id,
            name,
            started_at: Instant::now(),
        });
        id
    }

    pub fn complete(&mut self, id: CommandId, success: bool) {
        if let Some(pos) = self.running.iter().position(|c| c.id == id) {
            let cmd = self.running.remove(pos);
            let duration = cmd.started_at.elapsed();
            self.history.push_front(CompletedCommand {
                name: cmd.name,
                success,
                duration,
                completed_at: Instant::now(),
            });
            while self.history.len() > self.max_history {
                self.history.pop_back();
            }
        }
    }

    pub fn toggle_expanded(&mut self) {
        self.expanded = !self.expanded;
    }

    pub fn is_expanded(&self) -> bool {
        self.expanded
    }

    pub fn running_count(&self) -> usize {
        self.running.len()
    }

    pub fn has_running(&self) -> bool {
        !self.running.is_empty()
    }

    /// Render inline status on the breadcrumb line (right-aligned).
    /// Returns the width consumed so breadcrumbs can avoid overlap.
    pub fn render_inline(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) -> u16 {
        if self.running.is_empty() {
            return 0;
        }

        let elapsed = self.running[0].started_at.elapsed();
        let time_str = format_duration(elapsed);

        // Build compact status
        let status = if self.running.len() == 1 {
            format!("{} {}", self.running[0].name, time_str)
        } else {
            format!(
                "{} {} (+{})",
                self.running[0].name,
                time_str,
                self.running.len() - 1
            )
        };

        let width = status.len() as u16 + 3; // +3 for spinner and spacing

        // Position on right side of area
        let x = area.right().saturating_sub(width);
        let spinner_area = Rect::new(x, area.y, 2, 1);
        let text_area = Rect::new(x + 2, area.y, width - 2, 1);

        // Render spinner
        let throbber = Throbber::default()
            .throbber_set(BRAILLE_SIX)
            .use_type(WhichUse::Spin)
            .throbber_style(Style::default().fg(theme.lavender()));
        frame.render_stateful_widget(throbber, spinner_area, &mut self.throbber_state);

        // Render text
        let text = Paragraph::new(status).style(Style::default().fg(theme.subtext0()));
        frame.render_widget(text, text_area);

        width
    }

    /// Render expanded overlay panel with full details.
    pub fn render_expanded_panel(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        if !self.expanded {
            return;
        }

        // Calculate dynamic height based on content
        let running_lines = if self.running.is_empty() {
            0
        } else {
            self.running.len() + 1 // +1 for header
        };
        let history_to_show = self.history.len().min(5);
        let history_lines = if history_to_show == 0 {
            0
        } else {
            history_to_show + 1 // +1 for header
        };
        let separator = if running_lines > 0 && history_lines > 0 {
            1
        } else {
            0
        };
        let content_lines = running_lines + history_lines + separator;

        if content_lines == 0 {
            return;
        }

        let width = 65u16.min(area.width.saturating_sub(4));
        let height = (content_lines as u16 + 2).min(15); // +2 for borders

        // Position in bottom right of main area
        let x = area.right().saturating_sub(width + 2);
        let y = area.bottom().saturating_sub(height + 1);
        let widget_area = Rect::new(x, y, width, height);

        frame.render_widget(Clear, widget_area);

        // Title with stats
        let title = if self.running.is_empty() {
            format!(" Commands ({} recent) ", self.history.len())
        } else {
            format!(
                " Commands ({} running, {} recent) ",
                self.running.len(),
                self.history.len()
            )
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.surface2()))
            .title(title)
            .title_style(
                Style::default()
                    .fg(theme.mauve())
                    .add_modifier(Modifier::BOLD),
            )
            .style(Style::default().bg(theme.mantle()));

        let inner = block.inner(widget_area);
        frame.render_widget(block, widget_area);

        // Layout: "  ▰▰▱ <name padded/truncated>  <time right-aligned>"
        // Running: prefix "  ▰▰▱ " = 7 chars, time col = 8 chars (e.g., "  1.2s  ")
        // History: prefix "  ✓ " = 4 chars, time col = 18 chars (e.g., "  1.2s · just now")
        let inner_width = inner.width as usize;
        let running_prefix_len = 7; // "  ▰▰▱ "
        let running_time_col = 10;
        let history_prefix_len = 4; // "  ✓ "
        let history_time_col = 18;

        let mut lines: Vec<Line> = Vec::new();

        // Running commands section
        if !self.running.is_empty() {
            lines.push(Line::from(vec![
                Span::styled("⚡ ", Style::default().fg(theme.yellow())),
                Span::styled(
                    "RUNNING",
                    Style::default()
                        .fg(theme.yellow())
                        .add_modifier(Modifier::BOLD),
                ),
            ]));

            let name_max_len = inner_width
                .saturating_sub(running_prefix_len)
                .saturating_sub(running_time_col);

            for cmd in &self.running {
                let elapsed = cmd.started_at.elapsed();
                let time_str = format_duration(elapsed);

                // Create a simple progress indicator based on time
                let progress_char = match elapsed.as_secs() % 4 {
                    0 => "▰▱▱",
                    1 => "▰▰▱",
                    2 => "▰▰▰",
                    _ => "▱▰▰",
                };

                let name = truncate_with_ellipsis(&cmd.name, name_max_len);
                let padding = name_max_len.saturating_sub(display_width(&name));
                let time_display = format!("{:>width$}", time_str, width = running_time_col);

                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(progress_char, Style::default().fg(theme.peach())),
                    Span::raw(" "),
                    Span::styled(name, Style::default().fg(theme.text())),
                    Span::raw(" ".repeat(padding)),
                    Span::styled(
                        time_display,
                        Style::default()
                            .fg(theme.overlay1())
                            .add_modifier(Modifier::DIM),
                    ),
                ]));
            }
        }

        // Separator
        if !self.running.is_empty() && !self.history.is_empty() {
            lines.push(Line::raw(""));
        }

        // History section
        if !self.history.is_empty() {
            lines.push(Line::from(vec![
                Span::styled("📋 ", Style::default().fg(theme.subtext0())),
                Span::styled(
                    "RECENT",
                    Style::default()
                        .fg(theme.subtext0())
                        .add_modifier(Modifier::BOLD),
                ),
            ]));

            let name_max_len = inner_width
                .saturating_sub(history_prefix_len)
                .saturating_sub(history_time_col);

            for cmd in self.history.iter().take(5) {
                let (icon, color) = if cmd.success {
                    ("✓", theme.green())
                } else {
                    ("✗", theme.red())
                };

                let duration_str = format_duration(cmd.duration);
                let age = format_age(cmd.completed_at.elapsed());
                let time_info = format!("{} · {}", duration_str, age);

                let name = truncate_with_ellipsis(&cmd.name, name_max_len);
                let padding = name_max_len.saturating_sub(display_width(&name));
                let time_display = format!("{:>width$}", time_info, width = history_time_col);

                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(icon, Style::default().fg(color)),
                    Span::raw(" "),
                    Span::styled(name, Style::default().fg(theme.subtext1())),
                    Span::raw(" ".repeat(padding)),
                    Span::styled(
                        time_display,
                        Style::default()
                            .fg(theme.overlay0())
                            .add_modifier(Modifier::DIM),
                    ),
                ]));
            }
        }

        let paragraph = Paragraph::new(lines);
        frame.render_widget(paragraph, inner);
    }
}

fn truncate_with_ellipsis(s: &str, max_len: usize) -> String {
    if max_len == 0 {
        return String::new();
    }
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max_len {
        s.to_string()
    } else if max_len <= 1 {
        "…".to_string()
    } else {
        let truncated: String = chars[..max_len - 1].iter().collect();
        format!("{}…", truncated)
    }
}

fn display_width(s: &str) -> usize {
    s.chars().count()
}

fn format_duration(d: Duration) -> String {
    let secs = d.as_secs_f64();
    if secs < 1.0 {
        format!("{:.0}ms", d.as_millis())
    } else if secs < 10.0 {
        format!("{:.1}s", secs)
    } else if secs < 60.0 {
        format!("{:.0}s", secs)
    } else {
        let mins = secs / 60.0;
        format!("{:.1}m", mins)
    }
}

fn format_age(d: Duration) -> String {
    let secs = d.as_secs();
    if secs < 5 {
        "just now".to_string()
    } else if secs < 60 {
        format!("{}s ago", secs)
    } else if secs < 3600 {
        format!("{}m ago", secs / 60)
    } else {
        format!("{}h ago", secs / 3600)
    }
}

impl Default for CommandStatusView {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for CommandStatusView {
    type Output = ();

    fn on_tick(&mut self) {
        self.throbber_state.calc_next();
    }

    fn handle_key(
        &mut self,
        _key: crossterm::event::KeyEvent,
    ) -> crate::ui::Result<crate::ui::Handled<Self::Output>> {
        Ok(crate::ui::Handled::Ignored)
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        // Only render expanded panel here; inline is rendered separately
        self.render_expanded_panel(frame, area, theme);
    }
}
