use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

/// A keybinding entry for the help overlay.
pub struct Keybinding {
    pub key: &'static str,
    pub description: &'static str,
}

impl Keybinding {
    pub const fn new(key: &'static str, description: &'static str) -> Self {
        Self { key, description }
    }
}

/// Help overlay that displays keybindings in a centered popup.
pub struct HelpOverlay {
    visible: bool,
}

impl HelpOverlay {
    pub fn new() -> Self {
        Self { visible: false }
    }

    pub fn show(&mut self) {
        self.visible = true;
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }

    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, keybindings: &[Keybinding]) {
        if !self.visible {
            return;
        }

        // Calculate centered popup area
        let popup_area = centered_rect(60, 70, area);

        // Clear the area behind the popup
        frame.render_widget(Clear, popup_area);

        // Build keybinding lines
        let key_style = Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD);
        let desc_style = Style::default().fg(Color::White);

        let lines: Vec<Line> = keybindings
            .iter()
            .map(|kb| {
                Line::from(vec![
                    Span::styled(format!("{:>12}", kb.key), key_style),
                    Span::raw("  "),
                    Span::styled(kb.description, desc_style),
                ])
            })
            .collect();

        let block = Block::default()
            .title(" Help (press ? or Esc to close) ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));

        let paragraph = Paragraph::new(lines).block(block);

        frame.render_widget(paragraph, popup_area);
    }
}

impl Default for HelpOverlay {
    fn default() -> Self {
        Self::new()
    }
}

/// Create a centered rectangle with percentage-based width and height.
fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
