use std::sync::Arc;

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};

use crate::Theme;
use crate::config::{GlobalAction, KeyResolver, NavAction};
use crate::context::CloudContext;
use crate::ui::Keybinding;

/// ASCII art logo for the status bar.
const LOGO: &[&str] = &[
    r"  .--.            z Z ",
    r" (^_^ )  .--.      Z  ",
    r"  `--'  ( u.u) .--. z ",
    r"         `--' (^o^ )  ",
    r"    .--.       `--'   ",
    r"   ( -.-) lazycloud   ",
    r"    `--'              ",
];

pub struct StatusBar {
    active_context: Option<CloudContext>,
    resolver: Arc<KeyResolver>,
}

impl StatusBar {
    pub const fn new(resolver: Arc<KeyResolver>) -> Self {
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
        &self,
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
                Constraint::Length(42), // Left: status info
                Constraint::Min(20),    // Middle: keybindings (flexible)
                Constraint::Length(25), // Right: logo
            ])
            .split(inner_area);

        // === Left: Status Info ===
        self.render_status_info(frame, chunks[0], theme);

        // === Middle: Keybindings in columns ===
        self.render_keybindings(frame, chunks[1], theme, local_keybindings);

        // === Right: Logo ===
        Self::render_logo(frame, chunks[2], theme);
    }

    fn render_status_info(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let w = area.width as usize;
        let label_style = Style::default().fg(theme.overlay1());
        let value_style = Style::default().fg(theme.text());

        let lines = match &self.active_context {
            Some(CloudContext::Gcp(gcp)) => {
                let region = gcp
                    .region
                    .as_deref()
                    .or(gcp.zone.as_deref())
                    .unwrap_or("—");

                vec![
                    Line::from(Span::styled(
                        truncate_str(&gcp.display_name, w),
                        Style::default()
                            .fg(theme.lavender())
                            .add_modifier(Modifier::BOLD),
                    )),
                    Line::from(""),
                    status_line("provider", "GCP", w, label_style, Style::default().fg(theme.blue())),
                    status_line("project", &gcp.project_id, w, label_style, value_style),
                    status_line("account", &gcp.account, w, label_style, value_style),
                    status_line("region", region, w, label_style, value_style),
                ]
            }
            None => {
                vec![
                    Line::from(Span::styled(
                        "No context",
                        Style::default()
                            .fg(theme.overlay0())
                            .add_modifier(Modifier::BOLD),
                    )),
                ]
            }
        };

        let paragraph = Paragraph::new(lines);
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

        // Compute alignment widths from actual content so the separator
        // forms a straight vertical line regardless of key length.
        let max_key_w = hints.iter().map(|kb| kb.key.len()).max().unwrap_or(1);
        let max_desc_w = hints.iter().map(|kb| kb.description.len()).max().unwrap_or(1);
        // key(right-aligned) + " │ " (3) + desc + gap(2)
        let col_width = u16::try_from(max_key_w + 3 + max_desc_w + 2).unwrap_or(u16::MAX);
        let num_cols = (area.width / col_width).max(1) as usize;
        let num_rows = area.height as usize;

        // Distribute keybindings across columns (fill column by column)
        let mut columns: Vec<Vec<Line>> = vec![Vec::new(); num_cols];

        for (i, kb) in hints.iter().enumerate() {
            let col_idx = i / num_rows;
            if col_idx >= num_cols {
                break;
            }

            let line = Line::from(vec![
                Span::styled(
                    format!("{:>width$}", kb.key, width = max_key_w),
                    Style::default().fg(theme.peach()),
                ),
                Span::styled(" │ ", Style::default().fg(theme.surface2())),
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

    fn render_logo(frame: &mut Frame, area: Rect, theme: &Theme) {
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

/// Render a labelled status line: `  label  value` (right-aligned label, then value).
fn status_line<'a>(
    label: &'a str,
    value: &str,
    max_width: usize,
    label_style: Style,
    value_style: Style,
) -> Line<'a> {
    const LABEL_W: usize = 10;
    let available = max_width.saturating_sub(LABEL_W + 1);
    Line::from(vec![
        Span::styled(format!("{label:>LABEL_W$}"), label_style),
        Span::raw(" "),
        Span::styled(truncate_str(value, available), value_style),
    ])
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
