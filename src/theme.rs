use catppuccin::PALETTE;
use ratatui::style::Color;
use ratatui::widgets::BorderType;

/// Convert a catppuccin color to a ratatui color.
const fn catppuccin_to_color(c: &catppuccin::Color) -> Color {
    Color::Rgb(c.rgb.r, c.rgb.g, c.rgb.b)
}

/// Application theme with customizable colors.
///
/// This struct holds all color values directly, making it independent of any
/// specific color palette. Use the provided factory functions like `catppuccin_mocha()`
/// to create pre-configured themes, or build custom themes by setting colors directly.
#[derive(Debug, Clone, Copy)]
pub struct Theme {
    // Base colors
    pub base: Color,
    pub mantle: Color,
    pub crust: Color,

    // Surface colors
    pub surface0: Color,
    pub surface1: Color,
    pub surface2: Color,

    // Overlay colors
    pub overlay0: Color,
    pub overlay1: Color,
    pub overlay2: Color,

    // Text colors
    pub text: Color,
    pub subtext0: Color,
    pub subtext1: Color,

    // Accent colors
    pub rosewater: Color,
    pub flamingo: Color,
    pub pink: Color,
    pub mauve: Color,
    pub red: Color,
    pub maroon: Color,
    pub peach: Color,
    pub yellow: Color,
    pub green: Color,
    pub teal: Color,
    pub sky: Color,
    pub sapphire: Color,
    pub blue: Color,
    pub lavender: Color,

    pub border_type: BorderType,
}

impl Theme {
    /// Create a theme from a Catppuccin flavor.
    const fn from_catppuccin(flavor: &catppuccin::Flavor) -> Self {
        let c = &flavor.colors;
        Self {
            base: catppuccin_to_color(&c.base),
            mantle: catppuccin_to_color(&c.mantle),
            crust: catppuccin_to_color(&c.crust),
            surface0: catppuccin_to_color(&c.surface0),
            surface1: catppuccin_to_color(&c.surface1),
            surface2: catppuccin_to_color(&c.surface2),
            overlay0: catppuccin_to_color(&c.overlay0),
            overlay1: catppuccin_to_color(&c.overlay1),
            overlay2: catppuccin_to_color(&c.overlay2),
            text: catppuccin_to_color(&c.text),
            subtext0: catppuccin_to_color(&c.subtext0),
            subtext1: catppuccin_to_color(&c.subtext1),
            rosewater: catppuccin_to_color(&c.rosewater),
            flamingo: catppuccin_to_color(&c.flamingo),
            pink: catppuccin_to_color(&c.pink),
            mauve: catppuccin_to_color(&c.mauve),
            red: catppuccin_to_color(&c.red),
            maroon: catppuccin_to_color(&c.maroon),
            peach: catppuccin_to_color(&c.peach),
            yellow: catppuccin_to_color(&c.yellow),
            green: catppuccin_to_color(&c.green),
            teal: catppuccin_to_color(&c.teal),
            sky: catppuccin_to_color(&c.sky),
            sapphire: catppuccin_to_color(&c.sapphire),
            blue: catppuccin_to_color(&c.blue),
            lavender: catppuccin_to_color(&c.lavender),
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

    /// Catppuccin Frappé theme (dark).
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
        // Orange/amber accent colors inspired by k9s
        let orange = Color::Rgb(255, 153, 0); // Primary orange #FF9900
        let orange_light = Color::Rgb(255, 179, 71); // Lighter orange #FFB347
        let orange_dark = Color::Rgb(204, 102, 0); // Darker orange #CC6600
        let amber = Color::Rgb(255, 191, 0); // Amber/gold #FFBF00
        let coral = Color::Rgb(255, 127, 80); // Coral #FF7F50
        let rust = Color::Rgb(183, 65, 14); // Rust #B7410E

        Self {
            // Transparent backgrounds - use terminal default
            base: Color::Reset,
            mantle: Color::Reset,
            crust: Color::Reset,

            // Surface colors - dark with subtle warmth, keeps text readable
            surface0: Color::Rgb(40, 40, 40),
            surface1: Color::Rgb(55, 50, 45), // Selection background - subtle warm tint
            surface2: Color::Rgb(70, 65, 60),

            // Overlay colors
            overlay0: Color::Rgb(120, 120, 120),
            overlay1: Color::Rgb(140, 140, 140),
            overlay2: Color::Rgb(160, 160, 160),

            // Text colors - bright for visibility
            text: Color::Rgb(230, 230, 230),
            subtext0: Color::Rgb(180, 180, 180),
            subtext1: Color::Rgb(200, 200, 200),

            // Accent colors - orange/ember palette
            rosewater: orange_light,
            flamingo: coral,
            pink: Color::Rgb(255, 154, 162), // Soft pink
            mauve: orange,                   // Primary accent
            red: Color::Rgb(255, 85, 85),    // Bright red
            maroon: rust,
            peach: orange_light,
            yellow: amber,
            green: Color::Rgb(152, 195, 121),    // Muted green
            teal: Color::Rgb(86, 182, 194),      // Teal
            sky: Color::Rgb(137, 220, 235),      // Sky blue
            sapphire: Color::Rgb(116, 199, 236), // Sapphire
            blue: Color::Rgb(137, 180, 250),     // Soft blue
            lavender: orange_dark,               // Secondary accent

            border_type: BorderType::Rounded,
        }
    }

    // Base colors
    #[must_use]
    pub const fn base(&self) -> Color {
        self.base
    }

    #[must_use]
    pub const fn mantle(&self) -> Color {
        self.mantle
    }

    #[must_use]
    pub const fn crust(&self) -> Color {
        self.crust
    }

    // Surface colors
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

    // Overlay colors
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

    // Text colors
    #[must_use]
    pub const fn text(&self) -> Color {
        self.text
    }

    #[must_use]
    pub const fn subtext0(&self) -> Color {
        self.subtext0
    }

    #[must_use]
    pub const fn subtext1(&self) -> Color {
        self.subtext1
    }

    // Accent colors
    #[must_use]
    pub const fn rosewater(&self) -> Color {
        self.rosewater
    }

    #[must_use]
    pub const fn flamingo(&self) -> Color {
        self.flamingo
    }

    #[must_use]
    pub const fn pink(&self) -> Color {
        self.pink
    }

    #[must_use]
    pub const fn mauve(&self) -> Color {
        self.mauve
    }

    #[must_use]
    pub const fn red(&self) -> Color {
        self.red
    }

    #[must_use]
    pub const fn maroon(&self) -> Color {
        self.maroon
    }

    #[must_use]
    pub const fn peach(&self) -> Color {
        self.peach
    }

    #[must_use]
    pub const fn yellow(&self) -> Color {
        self.yellow
    }

    #[must_use]
    pub const fn green(&self) -> Color {
        self.green
    }

    #[must_use]
    pub const fn teal(&self) -> Color {
        self.teal
    }

    #[must_use]
    pub const fn sky(&self) -> Color {
        self.sky
    }

    #[must_use]
    pub const fn sapphire(&self) -> Color {
        self.sapphire
    }

    #[must_use]
    pub const fn blue(&self) -> Color {
        self.blue
    }

    #[must_use]
    pub const fn lavender(&self) -> Color {
        self.lavender
    }

    // Semantic colors
    #[must_use]
    pub const fn primary(&self) -> Color {
        self.blue
    }

    #[must_use]
    pub const fn secondary(&self) -> Color {
        self.mauve
    }

    #[must_use]
    pub const fn success(&self) -> Color {
        self.green
    }

    #[must_use]
    pub const fn warning(&self) -> Color {
        self.yellow
    }

    #[must_use]
    pub const fn error(&self) -> Color {
        self.red
    }

    #[must_use]
    pub const fn info(&self) -> Color {
        self.sky
    }

    // UI element colors
    #[must_use]
    pub const fn border(&self) -> Color {
        self.surface1
    }

    #[must_use]
    pub const fn border_focused(&self) -> Color {
        self.lavender
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
        self.yellow
    }

    #[must_use]
    pub const fn highlight(&self) -> Color {
        self.mauve
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
        ThemeInfo::new("Catppuccin Frappé", Theme::catppuccin_frappe()),
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
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Borders, Clear, ListItem};

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
