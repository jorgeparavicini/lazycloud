use crate::Theme;
use crate::component::Keybinding;
use crate::config::{GlobalAction, KeyResolver, NavAction};
use crate::model::CloudContext;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
};
use std::sync::Arc;

/// ASCII art logo for the status bar.
const LOGO: &[&str] = &[
    r#"  .--.            z Z "#,
    r#" (^_^ )  .--.      Z  "#,
    r#"  `--'  ( u.u) .--. z "#,
    r#"         `--' (^o^ )  "#,
    r#"    .--.       `--'   "#,
    r#"   ( -.-) lazycloud   "#,
    r#"    `--'              "#,
];

pub struct StatusBarView {
    active_context: Option<CloudContext>,
    resolver: Arc<KeyResolver>,
}

impl StatusBarView {
    pub fn new(resolver: Arc<KeyResolver>) -> Self {
        Self {
            active_context: None,
            resolver,
        }
    }

    pub fn set_active_context(&mut self, context: CloudContext) {
        self.active_context = Some(context);
    }

    pub fn clear_context(&mut self) {
        self.active_context = None;
    }

    pub fn render_with_keybindings(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        theme: &Theme,
        local_keybindings: &[Keybinding],
    ) {
        // Draw outer block
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.surface1()));

        let inner_area = block.inner(area);
        frame.render_widget(block, area);

        // Split into three columns: status (left), keybindings (middle), logo (right)
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(20), // Left: status info
                Constraint::Min(30),    // Middle: keybindings (flexible)
                Constraint::Length(25), // Right: logo
            ])
            .split(inner_area);

        // === Left: Status Info ===
        self.render_status_info(frame, chunks[0], theme);

        // === Middle: Keybindings in columns ===
        self.render_keybindings(frame, chunks[1], theme, local_keybindings);

        // === Right: Logo ===
        self.render_logo(frame, chunks[2], theme);
    }

    fn render_status_info(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let context_line = match &self.active_context {
            Some(CloudContext::Gcp(gcp)) => {
                vec![
                    Line::from(Span::styled(
                        "Context",
                        Style::default()
                            .fg(theme.subtext0())
                            .add_modifier(Modifier::BOLD),
                    )),
                    Line::from(Span::styled("GCP", Style::default().fg(theme.blue()))),
                    Line::from(Span::styled(
                        truncate_str(&gcp.project_id, area.width as usize - 1),
                        Style::default().fg(theme.text()),
                    )),
                ]
            }
            Some(CloudContext::Aws(aws)) => {
                vec![
                    Line::from(Span::styled(
                        "Context",
                        Style::default()
                            .fg(theme.subtext0())
                            .add_modifier(Modifier::BOLD),
                    )),
                    Line::from(Span::styled("AWS", Style::default().fg(theme.peach()))),
                    Line::from(Span::styled(
                        truncate_str(&aws.profile, area.width as usize - 1),
                        Style::default().fg(theme.text()),
                    )),
                ]
            }
            Some(CloudContext::Azure(azure)) => {
                vec![
                    Line::from(Span::styled(
                        "Context",
                        Style::default()
                            .fg(theme.subtext0())
                            .add_modifier(Modifier::BOLD),
                    )),
                    Line::from(Span::styled("Azure", Style::default().fg(theme.sky()))),
                    Line::from(Span::styled(
                        truncate_str(&azure.subscription_id, area.width as usize - 1),
                        Style::default().fg(theme.text()),
                    )),
                ]
            }
            None => {
                vec![
                    Line::from(Span::styled(
                        "Context",
                        Style::default()
                            .fg(theme.subtext0())
                            .add_modifier(Modifier::BOLD),
                    )),
                    Line::from(Span::styled("None", Style::default().fg(theme.overlay0()))),
                ]
            }
        };

        let paragraph = Paragraph::new(context_line);
        frame.render_widget(paragraph, area);
    }

    fn render_keybindings(
        &self,
        frame: &mut Frame,
        area: Rect,
        theme: &Theme,
        local_keybindings: &[Keybinding],
    ) {
        // Generate global keybindings from resolver
        let global_keybindings = self.global_keybindings();

        // Collect all hint keybindings (local first, then global)
        let hints: Vec<&Keybinding> = local_keybindings
            .iter()
            .filter(|kb| kb.hint)
            .chain(global_keybindings.iter().filter(|kb| kb.hint))
            .collect();

        if hints.is_empty() {
            return;
        }

        // Calculate how many columns we can fit
        // Each keybinding takes roughly: key(5) + space(1) + desc(10) + padding(2) = ~18 chars
        let col_width = 16_u16;
        let num_cols = (area.width / col_width).max(1) as usize;
        let num_rows = area.height as usize;

        // Distribute keybindings across columns (fill column by column)
        let mut columns: Vec<Vec<Line>> = vec![Vec::new(); num_cols];

        for (i, kb) in hints.iter().enumerate() {
            let col_idx = i / num_rows;
            if col_idx >= num_cols {
                break; // No more space
            }

            let line = Line::from(vec![
                Span::styled(format!("{:>5}", kb.key), Style::default().fg(theme.peach())),
                Span::raw(" "),
                Span::styled(
                    kb.description.clone(),
                    Style::default().fg(theme.subtext0()),
                ),
            ]);
            columns[col_idx].push(line);
        }

        // Create column areas
        let col_constraints: Vec<Constraint> = vec![Constraint::Length(col_width); num_cols];
        let col_areas = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(col_constraints)
            .split(area);

        // Render each column
        for (col_idx, col_lines) in columns.into_iter().enumerate() {
            if col_idx < col_areas.len() {
                let paragraph = Paragraph::new(col_lines);
                frame.render_widget(paragraph, col_areas[col_idx]);
            }
        }
    }

    fn render_logo(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let logo_lines: Vec<Line> = LOGO
            .iter()
            .map(|line| {
                Line::from(Span::styled(
                    *line,
                    Style::default()
                        .fg(theme.mauve())
                        .add_modifier(Modifier::BOLD),
                ))
            })
            .collect();

        let paragraph = Paragraph::new(logo_lines);
        frame.render_widget(paragraph, area);
    }

    /// Get the global keybindings for use in the help overlay.
    pub fn global_keybindings(&self) -> Vec<Keybinding> {
        vec![
            Keybinding::hint(self.resolver.display_global(GlobalAction::Help), "Help"),
            Keybinding::hint(self.resolver.display_global(GlobalAction::Back), "Back"),
            Keybinding::new(self.resolver.display_global(GlobalAction::Theme), "Theme"),
            Keybinding::new(self.resolver.display_global(GlobalAction::Quit), "Quit"),
            Keybinding::new(
                self.resolver.display_global(GlobalAction::CommandsToggle),
                "Commands",
            ),
            Keybinding::new(self.resolver.display_nav(NavAction::Select), "Select"),
            Keybinding::new(
                format!(
                    "{}/{}",
                    self.resolver.display_nav(NavAction::Up),
                    self.resolver.display_nav(NavAction::Down)
                ),
                "Navigate",
            ),
        ]
    }
}

/// Truncate a string to fit within a given width, adding "..." if truncated.
fn truncate_str(s: &str, max_width: usize) -> String {
    if s.len() <= max_width {
        s.to_string()
    } else if max_width > 3 {
        format!("{}...", &s[..max_width - 3])
    } else {
        s[..max_width].to_string()
    }
}
