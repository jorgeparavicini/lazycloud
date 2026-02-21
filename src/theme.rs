use catppuccin::PALETTE;
use ratatui::style::Color;
use ratatui::widgets::BorderType;

/// Convert a catppuccin color to a ratatui color.
const fn catppuccin_to_color(c: &catppuccin::Color) -> Color {
    Color::Rgb(c.rgb.r, c.rgb.g, c.rgb.b)
}

/// Application theme with semantic color names.
///
/// Use the provided factory functions
/// like `catppuccin_mocha()` to create pre-configured themes, or build custom
/// themes by setting colors directly.
#[derive(Debug, Clone, Copy)]
pub struct Theme {
    // Backgrounds
    pub bg: Color,
    pub bg_dim: Color,
    pub bg_deep: Color,

    // Surfaces (interactive backgrounds, dim to bright)
    pub surface0: Color,
    pub surface1: Color,
    pub surface2: Color,

    // Overlays (muted UI chrome)
    pub overlay0: Color,
    pub overlay1: Color,
    pub overlay2: Color,

    // Text
    pub text: Color,
    pub text_dim: Color,
    pub text_muted: Color,

    // Semantic accent colors
    pub primary: Color,
    pub secondary: Color,
    pub accent: Color,
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub info: Color,
    pub highlight: Color,

    pub border_type: BorderType,
}

impl Theme {
    /// Create a theme from a Catppuccin flavor.
    const fn from_catppuccin(flavor: &catppuccin::Flavor) -> Self {
        let c = &flavor.colors;
        Self {
            bg: catppuccin_to_color(&c.base),
            bg_dim: catppuccin_to_color(&c.mantle),
            bg_deep: catppuccin_to_color(&c.crust),
            surface0: catppuccin_to_color(&c.surface0),
            surface1: catppuccin_to_color(&c.surface1),
            surface2: catppuccin_to_color(&c.surface2),
            overlay0: catppuccin_to_color(&c.overlay0),
            overlay1: catppuccin_to_color(&c.overlay1),
            overlay2: catppuccin_to_color(&c.overlay2),
            text: catppuccin_to_color(&c.text),
            text_dim: catppuccin_to_color(&c.subtext1),
            text_muted: catppuccin_to_color(&c.subtext0),
            primary: catppuccin_to_color(&c.blue),
            secondary: catppuccin_to_color(&c.mauve),
            accent: catppuccin_to_color(&c.peach),
            success: catppuccin_to_color(&c.green),
            warning: catppuccin_to_color(&c.yellow),
            error: catppuccin_to_color(&c.red),
            info: catppuccin_to_color(&c.sky),
            highlight: catppuccin_to_color(&c.lavender),
            border_type: BorderType::Rounded,
        }
    }

    /// Catppuccin Mocha theme (dark).
    #[must_use]
    pub const fn catppuccin_mocha() -> Self {
        Self::from_catppuccin(&PALETTE.mocha)
    }

    /// Catppuccin Latte theme (light).
    #[must_use]
    pub const fn catppuccin_latte() -> Self {
        Self::from_catppuccin(&PALETTE.latte)
    }

    /// Catppuccin Frappe theme (dark).
    #[must_use]
    pub const fn catppuccin_frappe() -> Self {
        Self::from_catppuccin(&PALETTE.frappe)
    }

    /// Catppuccin Macchiato theme (dark).
    #[must_use]
    pub const fn catppuccin_macchiato() -> Self {
        Self::from_catppuccin(&PALETTE.macchiato)
    }

    /// Ember theme - transparent background with orange accents (k9s-inspired).
    #[must_use]
    pub const fn ember() -> Self {
        let orange = Color::Rgb(255, 153, 0);
        let orange_light = Color::Rgb(255, 179, 71);
        let orange_dark = Color::Rgb(204, 102, 0);
        let amber = Color::Rgb(255, 191, 0);

        Self {
            bg: Color::Reset,
            bg_dim: Color::Reset,
            bg_deep: Color::Reset,

            surface0: Color::Rgb(40, 40, 40),
            surface1: Color::Rgb(55, 50, 45),
            surface2: Color::Rgb(70, 65, 60),

            overlay0: Color::Rgb(120, 120, 120),
            overlay1: Color::Rgb(140, 140, 140),
            overlay2: Color::Rgb(160, 160, 160),

            text: Color::Rgb(230, 230, 230),
            text_dim: Color::Rgb(200, 200, 200),
            text_muted: Color::Rgb(180, 180, 180),

            primary: Color::Rgb(137, 180, 250),
            secondary: orange,
            accent: orange_light,
            success: Color::Rgb(152, 195, 121),
            warning: amber,
            error: Color::Rgb(255, 85, 85),
            info: Color::Rgb(137, 220, 235),
            highlight: orange_dark,

            border_type: BorderType::Rounded,
        }
    }

    // Backgrounds
    #[must_use]
    pub const fn bg(&self) -> Color {
        self.bg
    }

    #[must_use]
    pub const fn bg_dim(&self) -> Color {
        self.bg_dim
    }

    #[must_use]
    pub const fn bg_deep(&self) -> Color {
        self.bg_deep
    }

    // Surfaces
    #[must_use]
    pub const fn surface0(&self) -> Color {
        self.surface0
    }

    #[must_use]
    pub const fn surface1(&self) -> Color {
        self.surface1
    }

    #[must_use]
    pub const fn surface2(&self) -> Color {
        self.surface2
    }

    // Overlays
    #[must_use]
    pub const fn overlay0(&self) -> Color {
        self.overlay0
    }

    #[must_use]
    pub const fn overlay1(&self) -> Color {
        self.overlay1
    }

    #[must_use]
    pub const fn overlay2(&self) -> Color {
        self.overlay2
    }

    // Text
    #[must_use]
    pub const fn text(&self) -> Color {
        self.text
    }

    #[must_use]
    pub const fn text_dim(&self) -> Color {
        self.text_dim
    }

    #[must_use]
    pub const fn text_muted(&self) -> Color {
        self.text_muted
    }

    // Semantic accent colors
    #[must_use]
    pub const fn primary(&self) -> Color {
        self.primary
    }

    #[must_use]
    pub const fn secondary(&self) -> Color {
        self.secondary
    }

    #[must_use]
    pub const fn accent(&self) -> Color {
        self.accent
    }

    #[must_use]
    pub const fn success(&self) -> Color {
        self.success
    }

    #[must_use]
    pub const fn warning(&self) -> Color {
        self.warning
    }

    #[must_use]
    pub const fn error(&self) -> Color {
        self.error
    }

    #[must_use]
    pub const fn info(&self) -> Color {
        self.info
    }

    #[must_use]
    pub const fn highlight(&self) -> Color {
        self.highlight
    }

    // Computed semantic aliases
    #[must_use]
    pub const fn border(&self) -> Color {
        self.surface1
    }

    #[must_use]
    pub const fn border_focused(&self) -> Color {
        self.highlight
    }

    #[must_use]
    pub const fn selection_bg(&self) -> Color {
        self.surface1
    }

    #[must_use]
    pub const fn selection_fg(&self) -> Color {
        self.text
    }

    #[must_use]
    pub const fn header(&self) -> Color {
        self.warning
    }
}

// === Block / Style helpers ===

use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Borders};

impl Theme {
    /// Standard block with ALL borders, `border_type`, and default border color.
    #[must_use]
    pub fn block(&self) -> Block<'static> {
        Block::default()
            .borders(Borders::ALL)
            .border_type(self.border_type)
            .border_style(Style::default().fg(self.border()))
    }

    /// Popup block with ALL borders, `border_type`, focused border color, bg fill,
    /// and a bold secondary-colored title.
    #[must_use]
    pub fn popup_block(&self, title: &str) -> Block<'static> {
        Block::default()
            .title(title.to_string())
            .title_style(self.title_style())
            .borders(Borders::ALL)
            .border_type(self.border_type)
            .border_style(Style::default().fg(self.border_focused()))
            .style(Style::default().bg(self.bg()))
    }

    /// Bold secondary color for block titles.
    #[must_use]
    pub fn title_style(&self) -> Style {
        Style::default()
            .fg(self.secondary())
            .add_modifier(Modifier::BOLD)
    }

    /// Selection highlight: `selection_bg` + highlight fg + bold.
    #[must_use]
    pub fn highlight_style(&self) -> Style {
        Style::default()
            .bg(self.selection_bg())
            .fg(self.highlight())
            .add_modifier(Modifier::BOLD)
    }

    /// Bold accent color for keybinding key labels.
    #[must_use]
    pub fn key_style(&self) -> Style {
        Style::default()
            .fg(self.accent())
            .add_modifier(Modifier::BOLD)
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::catppuccin_mocha()
    }
}

/// Information about a theme for display in selectors.
#[derive(Debug, Clone)]
pub struct ThemeInfo {
    /// Display name for the theme
    pub name: &'static str,
    /// The theme instance
    pub theme: Theme,
}

impl ThemeInfo {
    const fn new(name: &'static str, theme: Theme) -> Self {
        Self { name, theme }
    }
}

impl std::fmt::Display for ThemeInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

/// Returns a list of all available built-in themes.
pub fn available_themes() -> Vec<ThemeInfo> {
    vec![
        ThemeInfo::new("Ember", Theme::ember()),
        ThemeInfo::new("Catppuccin Mocha", Theme::catppuccin_mocha()),
        ThemeInfo::new("Catppuccin Macchiato", Theme::catppuccin_macchiato()),
        ThemeInfo::new("Catppuccin Frappe", Theme::catppuccin_frappe()),
        ThemeInfo::new("Catppuccin Latte", Theme::catppuccin_latte()),
    ]
}

/// Look up a theme by name. Returns the default theme if not found.
pub fn theme_from_name(name: &str) -> Theme {
    available_themes()
        .into_iter()
        .find(|t| t.name == name)
        .map(|t| t.theme)
        .unwrap_or_default()
}

/// Get the name of a theme that matches the given theme, if any.
#[allow(dead_code)]
pub fn theme_name(theme: &Theme) -> Option<&'static str> {
    available_themes()
        .into_iter()
        .find(|t| {
            std::mem::discriminant(&t.theme.border_type)
                == std::mem::discriminant(&theme.border_type)
        })
        .map(|t| t.name)
}

// === Theme Selector View ===

use std::sync::Arc;

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use ratatui::widgets::{Clear, ListItem};

use crate::config::KeyResolver;
use crate::ui::{Component, EventResult, List, ListEvent, ListRow, Result};

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
    list: List<ThemeInfo>,
}

impl ThemeSelectorView {
    pub fn new(resolver: Arc<KeyResolver>) -> Self {
        let themes = available_themes();
        Self {
            list: List::new(themes, resolver),
        }
    }
}

impl Component for ThemeSelectorView {
    type Output = ThemeEvent;

    fn handle_key(&mut self, key: KeyEvent) -> Result<EventResult<Self::Output>> {
        // Handle escape/toggle to close
        if matches!(key.code, KeyCode::Esc | KeyCode::Char('t')) {
            return Ok(ThemeEvent::Cancelled.into());
        }

        // Delegate to list
        let result = self.list.handle_key(key)?;
        Ok(match result {
            EventResult::Event(ListEvent::Activated(info)) => ThemeEvent::Selected(info).into(),
            EventResult::Consumed | EventResult::Event(_) => EventResult::Consumed,
            EventResult::Ignored => EventResult::Ignored,
        })
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let popup_area = area.centered(Constraint::Percentage(40), Constraint::Percentage(50));
        frame.render_widget(Clear, popup_area);

        let block = theme.popup_block(" Select Theme (Enter to confirm, Esc to cancel) ");
        let inner = block.inner(popup_area);
        frame.render_widget(block, popup_area);

        self.list.render(frame, inner, theme);
    }
}
