use std::sync::Arc;

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, BorderType, Borders, Clear, ListItem};

use crate::Theme;
use crate::components::{ListComponent, ListEvent, ListRow};
use crate::config::KeyResolver;
use crate::theme::{ThemeInfo, available_themes};
use crate::components::{Component, Handled, Result};

impl ListRow for ThemeInfo {
    fn render_row(&self, theme: &Theme) -> ListItem<'static> {
        ListItem::new(self.name.to_string()).style(Style::default().fg(theme.text()))
    }
}

pub enum ThemeEvent {
    Cancelled,
    Selected(ThemeInfo),
}

pub struct ThemeSelectorView {
    list: ListComponent<ThemeInfo>,
}

impl ThemeSelectorView {
    pub fn new(resolver: Arc<KeyResolver>) -> Self {
        let themes = available_themes();
        Self {
            list: ListComponent::new(themes, resolver),
        }
    }
}

impl Component for ThemeSelectorView {
    type Output = ThemeEvent;

    fn handle_key(&mut self, key: KeyEvent) -> Result<Handled<Self::Output>> {
        // Handle escape/toggle to close
        if matches!(key.code, KeyCode::Esc | KeyCode::Char('t')) {
            return Ok(ThemeEvent::Cancelled.into());
        }

        // Delegate to list
        let result = self.list.handle_key(key)?;
        Ok(match result {
            Handled::Event(ListEvent::Activated(info)) => ThemeEvent::Selected(info).into(),
            Handled::Consumed | Handled::Event(_) => Handled::Consumed,
            Handled::Ignored => Handled::Ignored,
        })
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        // Calculate centered popup area
        let popup_area = area.centered(Constraint::Percentage(40), Constraint::Percentage(50));

        // Clear the area behind the popup
        frame.render_widget(Clear, popup_area);

        // Render block background
        let block = Block::default()
            .title(" Select Theme (Enter to confirm, Esc to cancel) ")
            .title_style(
                Style::default()
                    .fg(theme.mauve())
                    .add_modifier(Modifier::BOLD),
            )
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.lavender()))
            .style(Style::default().bg(theme.base()));

        let inner = block.inner(popup_area);
        frame.render_widget(block, popup_area);

        // Render the list inside
        self.list.render(frame, inner, theme);
    }
}
