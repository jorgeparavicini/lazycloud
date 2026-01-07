use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};
use std::collections::VecDeque;
use std::time::Instant;
use throbber_widgets_tui::{Throbber, ThrobberState, BRAILLE_SIX, WhichUse};

/// Unique identifier for a tracked command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CommandId(u64);

/// A running command being tracked.
#[derive(Debug)]
struct RunningCommand {
    id: CommandId,
    name: &'static str,
    started_at: Instant,
}

/// A completed command in history.
#[derive(Debug)]
struct CompletedCommand {
    name: &'static str,
    completed_at: Instant,
    success: bool,
}

/// Tracks running and recently completed commands.
pub struct CommandTracker {
    running: Vec<RunningCommand>,
    history: VecDeque<CompletedCommand>,
    next_id: u64,
    max_history: usize,
    expanded: bool,
    throbber_state: ThrobberState,
}

impl CommandTracker {
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

    /// Start tracking a new command, returns its ID.
    pub fn start(&mut self, name: &'static str) -> CommandId {
        let id = CommandId(self.next_id);
        self.next_id += 1;
        self.running.push(RunningCommand {
            id,
            name,
            started_at: Instant::now(),
        });
        id
    }

    /// Mark a command as completed.
    pub fn complete(&mut self, id: CommandId, success: bool) {
        if let Some(pos) = self.running.iter().position(|c| c.id == id) {
            let cmd = self.running.remove(pos);
            self.history.push_front(CompletedCommand {
                name: cmd.name,
                completed_at: Instant::now(),
                success,
            });
            // Trim history
            while self.history.len() > self.max_history {
                self.history.pop_back();
            }
        }
    }

    /// Toggle expanded view.
    pub fn toggle_expanded(&mut self) {
        self.expanded = !self.expanded;
    }

    /// Check if expanded.
    pub fn is_expanded(&self) -> bool {
        self.expanded
    }

    /// Get number of running commands.
    pub fn running_count(&self) -> usize {
        self.running.len()
    }

    /// Check if any commands are running.
    pub fn has_running(&self) -> bool {
        !self.running.is_empty()
    }

    /// Called on tick to animate spinner.
    pub fn on_tick(&mut self) {
        self.throbber_state.calc_next();
    }

    /// Render the command status widget.
    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        if self.running.is_empty() && self.history.is_empty() {
            return;
        }

        if self.expanded {
            self.render_expanded(frame, area);
        } else {
            self.render_collapsed(frame, area);
        }
    }

    fn render_collapsed(&mut self, frame: &mut Frame, area: Rect) {
        if self.running.is_empty() {
            return;
        }

        // Build status text
        let status = if self.running.len() == 1 {
            format!(" {} ", self.running[0].name)
        } else {
            format!(" {} (+{} more) ", self.running[0].name, self.running.len() - 1)
        };

        let width = status.len() as u16 + 3; // +3 for spinner and padding
        let height = 1;

        // Position in bottom right
        let x = area.right().saturating_sub(width + 1);
        let y = area.bottom().saturating_sub(height + 1);
        let widget_area = Rect::new(x, y, width, height);

        // Build line with spinner placeholder
        let line = Line::from(vec![
            Span::raw(" "),
            Span::styled(
                status,
                Style::default().fg(Color::Yellow),
            ),
        ]);

        // Clear background and render
        frame.render_widget(Clear, widget_area);

        // Render spinner at start
        let spinner_area = Rect::new(x, y, 2, 1);
        let throbber = Throbber::default()
            .throbber_set(BRAILLE_SIX)
            .use_type(WhichUse::Spin);
        frame.render_stateful_widget(throbber, spinner_area, &mut self.throbber_state);

        // Render text after spinner
        let text_area = Rect::new(x + 2, y, width - 2, 1);
        frame.render_widget(Paragraph::new(line), text_area);
    }

    fn render_expanded(&mut self, frame: &mut Frame, area: Rect) {
        let running_count = self.running.len();
        let history_count = self.history.len().min(5); // Show max 5 history items
        let total_lines = running_count + history_count + 2; // +2 for headers/spacing

        let width = 40u16;
        let height = (total_lines as u16).min(12) + 2; // +2 for borders

        // Position in bottom right
        let x = area.right().saturating_sub(width + 2);
        let y = area.bottom().saturating_sub(height + 1);
        let widget_area = Rect::new(x, y, width, height);

        frame.render_widget(Clear, widget_area);

        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Commands ")
            .style(Style::default().fg(Color::DarkGray));

        let inner = block.inner(widget_area);
        frame.render_widget(block, widget_area);

        let mut lines: Vec<Line> = Vec::new();

        // Running commands
        if !self.running.is_empty() {
            lines.push(Line::from(Span::styled(
                "Running:",
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            )));
            for cmd in &self.running {
                let elapsed = cmd.started_at.elapsed().as_secs();
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled("● ", Style::default().fg(Color::Yellow)),
                    Span::raw(cmd.name),
                    Span::styled(
                        format!(" ({}s)", elapsed),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]));
            }
        }

        // History
        if !self.history.is_empty() {
            if !lines.is_empty() {
                lines.push(Line::raw(""));
            }
            lines.push(Line::from(Span::styled(
                "Recent:",
                Style::default().fg(Color::DarkGray).add_modifier(Modifier::BOLD),
            )));
            for cmd in self.history.iter().take(5) {
                let icon = if cmd.success { "✓" } else { "✗" };
                let color = if cmd.success { Color::Green } else { Color::Red };
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(format!("{} ", icon), Style::default().fg(color)),
                    Span::styled(cmd.name, Style::default().fg(Color::DarkGray)),
                ]));
            }
        }

        let paragraph = Paragraph::new(lines);
        frame.render_widget(paragraph, inner);
    }
}

impl Default for CommandTracker {
    fn default() -> Self {
        Self::new()
    }
}
