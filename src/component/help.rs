use crate::ui::{Component, Handled, Result};
use crate::Theme;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame,
};

pub struct Keybinding {
    pub key: String,
    pub description: String,
    /// Whether this keybinding should be shown in the hints line at the bottom.
    pub hint: bool,
}

impl Keybinding {
    pub fn new(key: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            description: description.into(),
            hint: false,
        }
    }

    /// Create a keybinding that is also shown as a hint at the bottom of the screen.
    pub fn hint(key: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            description: description.into(),
            hint: true,
        }
    }
}

/// A section of keybindings for the help overlay.
pub struct KeybindingSection {
    pub title: String,
    pub keybindings: Vec<Keybinding>,
}

impl KeybindingSection {
    pub fn new(title: impl Into<String>, keybindings: Vec<Keybinding>) -> Self {
        Self {
            title: title.into(),
            keybindings,
        }
    }
}

pub enum HelpEvent {
    Close,
}

pub struct HelpView {
    sections: Vec<KeybindingSection>,
}

impl HelpView {
    pub fn new(keybindings: Vec<Keybinding>) -> Self {
        Self {
            sections: vec![KeybindingSection::new("Keybindings", keybindings)],
        }
    }

    pub fn with_sections(sections: Vec<KeybindingSection>) -> Self {
        Self { sections }
    }
}

impl Component for HelpView {
    type Output = HelpEvent;

    fn handle_key(&mut self, key: KeyEvent) -> Result<Handled<Self::Output>> {
        Ok(match key.code {
            KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q') => HelpEvent::Close.into(),
            _ => Handled::Ignored,
        })
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        // Calculate centered popup area
        let popup_area = area.centered(Constraint::Percentage(60), Constraint::Percentage(70));

        // Clear the area behind the popup
        frame.render_widget(Clear, popup_area);

        // Build keybinding lines with sections
        let key_style = Style::default()
            .fg(theme.peach())
            .add_modifier(Modifier::BOLD);
        let desc_style = Style::default().fg(theme.text());
        let section_style = Style::default()
            .fg(theme.subtext0())
            .add_modifier(Modifier::BOLD);

        let mut lines: Vec<Line> = Vec::new();

        for (i, section) in self.sections.iter().enumerate() {
            // Add blank line between sections (but not before first)
            if i > 0 {
                lines.push(Line::from(""));
            }

            // Section header
            let header = format!("── {} ──", section.title);
            lines.push(Line::from(Span::styled(header, section_style)));

            // Keybindings in this section
            for kb in &section.keybindings {
                lines.push(Line::from(vec![
                    Span::styled(format!("{:>12}", kb.key), key_style),
                    Span::raw("  "),
                    Span::styled(kb.description.clone(), desc_style),
                ]));
            }
        }

        let block = Block::default()
            .title(" Help (press ? or Esc to close) ")
            .title_style(Style::default().fg(theme.mauve()).add_modifier(Modifier::BOLD))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.lavender()))
            .style(Style::default().bg(theme.base()));

        let paragraph = Paragraph::new(lines).block(block);

        frame.render_widget(paragraph, popup_area);
    }
}
