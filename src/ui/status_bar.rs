use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::Theme;
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
}

impl StatusBar {
    pub const fn new() -> Self {
        Self {
            active_context: None,
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
        let block = theme.block();

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
                let region = gcp.region.as_deref().or(gcp.zone.as_deref()).unwrap_or("—");

                vec![
                    Line::from(Span::styled(
                        truncate_str(&gcp.display_name, w),
                        Style::default()
                            .fg(theme.highlight())
                            .add_modifier(Modifier::BOLD),
                    )),
                    Line::from(""),
                    status_line(
                        "provider",
                        "GCP",
                        w,
                        label_style,
                        Style::default().fg(theme.primary()),
                    ),
                    status_line("project", &gcp.project_id, w, label_style, value_style),
                    status_line("account", &gcp.account, w, label_style, value_style),
                    status_line("region", region, w, label_style, value_style),
                ]
            }
            None => {
                vec![Line::from(Span::styled(
                    "No context",
                    Style::default()
                        .fg(theme.overlay0())
                        .add_modifier(Modifier::BOLD),
                ))]
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
        // Generate global keybindings
        let global_keybindings = self.global_keybindings();

        // Collect all hint keybindings (local first, then global)
        let hints: Vec<&Keybinding> = local_keybindings
            .iter()
            .filter(|kb| kb.is_primary())
            .chain(global_keybindings.iter().filter(|kb| kb.is_primary()))
            .collect();

        if hints.is_empty() {
            return;
        }

        // Compute alignment widths from actual content so the separator
        // forms a straight vertical line regardless of key length.
        let max_key_w = hints.iter().map(|kb| kb.key.len()).max().unwrap_or(1);
        let max_desc_w = hints
            .iter()
            .map(|kb| kb.description.len())
            .max()
            .unwrap_or(1);
        // key(right-aligned) + " │ " (3) + desc + gap(2)
        let col_width = u16::try_from(max_key_w + 3 + max_desc_w + 2).unwrap_or(u16::MAX);
        let num_cols = (area.width / col_width).max(1) as usize;
        let num_rows = area.height as usize;

        // How many hints actually fit. If there are more than the grid can hold,
        // reserve the last cell for a "+N more" indicator pointing at the help
        // overlay, rather than silently dropping the overflow.
        let capacity = num_cols.saturating_mul(num_rows);
        let overflow = hints.len() > capacity;
        let shown = if overflow {
            capacity.saturating_sub(1)
        } else {
            hints.len()
        };

        let cell = |key: &str, desc: &str, key_style: Style, desc_style: Style| {
            Line::from(vec![
                Span::styled(format!("{key:>max_key_w$}"), key_style),
                Span::styled(" │ ", Style::default().fg(theme.surface2())),
                Span::styled(desc.to_string(), desc_style),
            ])
        };

        // Distribute keybindings across columns (fill column by column).
        let mut columns: Vec<Vec<Line>> = vec![Vec::new(); num_cols];

        for (i, kb) in hints.iter().take(shown).enumerate() {
            let col_idx = i / num_rows;
            columns[col_idx].push(cell(
                &kb.key,
                &kb.description,
                Style::default().fg(theme.accent()),
                Style::default().fg(theme.text_muted()),
            ));
        }

        if overflow {
            let remaining = hints.len() - shown;
            let col_idx = (shown / num_rows).min(num_cols - 1);
            columns[col_idx].push(cell(
                "?",
                &format!("+{remaining} more"),
                Style::default().fg(theme.accent()),
                Style::default()
                    .fg(theme.text_muted())
                    .add_modifier(Modifier::ITALIC),
            ));
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
                        .fg(theme.secondary())
                        .add_modifier(Modifier::BOLD),
                ))
            })
            .collect();

        let paragraph = Paragraph::new(logo_lines);
        frame.render_widget(paragraph, area);
    }

    /// Get the global keybindings for use in the help overlay.
    #[allow(clippy::unused_self)]
    pub fn global_keybindings(&self) -> Vec<Keybinding> {
        vec![
            Keybinding::primary("?", "Help"),
            Keybinding::primary("Esc", "Back"),
            Keybinding::secondary("t", "Theme"),
            Keybinding::secondary("q", "Quit"),
            Keybinding::secondary("c", "Commands"),
            Keybinding::secondary("L", "Logs"),
            Keybinding::secondary("Enter", "Select"),
            Keybinding::secondary("k/j", "Navigate"),
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
