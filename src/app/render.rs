use color_eyre::Result;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Paragraph};

use super::{ActivePopup, App, AppState};
use crate::tui::Tui;
use crate::ui::{Component, Screen};

impl App {
    pub(super) fn render(&mut self, tui: &mut Tui) -> Result<()> {
        tui.draw(|frame| {
            // Fill background with theme base color
            frame.render_widget(
                Block::default().style(Style::default().bg(self.theme.base())),
                frame.area(),
            );

            // Get keybindings for status bar
            let local_keybindings = match &self.state {
                AppState::ActiveService(service) => service.keybindings(),
                _ => vec![],
            };

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(9), // Status bar (logo + keybindings + context)
                    Constraint::Min(0),    // Main content
                    Constraint::Length(1), // Breadcrumbs
                ])
                .split(frame.area());

            // Render status bar with keybinding hints
            self.status_bar.render_with_keybindings(
                frame,
                chunks[0],
                &self.theme,
                &local_keybindings,
            );

            // Render current state
            match &mut self.state {
                AppState::SelectingContext(selector) => {
                    selector.render(frame, chunks[1], &self.theme);
                }
                AppState::SelectingService(selector) => {
                    selector.render(frame, chunks[1], &self.theme);
                }
                AppState::ActiveService(service) => {
                    service.render(frame, chunks[1], &self.theme);
                }
            }

            // Render breadcrumbs (left) and inline commands status (right)
            let breadcrumbs = self.build_breadcrumbs();
            let bc_text = breadcrumbs.join(" > ");

            // First render inline commands status to get its width
            let cmd_width = self
                .command_tracker
                .render_inline(frame, chunks[2], &self.theme);

            // Render breadcrumbs in remaining space
            let bc_area = Rect::new(
                chunks[2].x,
                chunks[2].y,
                chunks[2].width.saturating_sub(cmd_width + 2),
                chunks[2].height,
            );
            let bc_widget = Paragraph::new(bc_text).style(
                Style::default()
                    .fg(self.theme.overlay1())
                    .add_modifier(Modifier::ITALIC),
            );
            frame.render_widget(bc_widget, bc_area);

            // Render expanded commands panel (overlay on main content)
            self.command_tracker.render(frame, chunks[1], &self.theme);

            // Render toasts (bottom right of main content)
            self.toast_manager.render(frame, chunks[1], &self.theme);

            // Render popup overlay on top
            if let Some(ref mut popup) = self.popup {
                match popup {
                    ActivePopup::Help(help) => {
                        help.render(frame, frame.area(), &self.theme);
                    }
                    ActivePopup::ThemeSelector(selector) => {
                        selector.render(frame, frame.area(), &self.theme);
                    }
                    ActivePopup::Error(dialog) => {
                        dialog.render(frame, frame.area(), &self.theme);
                    }
                    ActivePopup::ContextMerge(merge) => {
                        merge.render(frame, frame.area(), &self.theme);
                    }
                }
            }
        })?;
        Ok(())
    }

    pub(super) fn build_breadcrumbs(&self) -> Vec<String> {
        match &self.state {
            AppState::SelectingContext(_) => {
                vec!["Select Context".to_string()]
            }
            AppState::SelectingService(_) => {
                let mut bc = vec![];
                if let Some(ctx) = &self.active_context {
                    bc.push(ctx.provider().display_name().to_string());
                }
                bc.push("Select Service".to_string());
                bc
            }
            AppState::ActiveService(service) => {
                let mut bc = vec![];
                if let Some(ctx) = &self.active_context {
                    bc.push(ctx.provider().display_name().to_string());
                }
                bc.extend(service.breadcrumbs());
                bc
            }
        }
    }
}
