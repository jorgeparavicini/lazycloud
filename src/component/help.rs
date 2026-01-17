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
    pub key: &'static str,
    pub description: &'static str,
}

impl Keybinding {
    pub const fn new(key: &'static str, description: &'static str) -> Self {
        Self { key, description }
    }
}

pub enum HelpEvent {
    Close,
}

pub struct HelpView {
    keybindings: &'static [Keybinding],
}

impl HelpView {
    pub fn new(keybindings: &'static [Keybinding]) -> Self {
        Self { keybindings }
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

        // Build keybinding lines
        let key_style = Style::default()
            .fg(theme.peach())
            .add_modifier(Modifier::BOLD);
        let desc_style = Style::default().fg(theme.text());

        let lines: Vec<Line> = self
            .keybindings
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
            .title_style(Style::default().fg(theme.mauve()).add_modifier(Modifier::BOLD))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.lavender()))
            .style(Style::default().bg(theme.base()));

        let paragraph = Paragraph::new(lines).block(block);

        frame.render_widget(paragraph, popup_area);
    }
}
