use catppuccin::PALETTE;
use ratatui::style::Color;

/// Convert a catppuccin color to a ratatui color.
fn catppuccin_to_color(c: &catppuccin::Color) -> Color {
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
}

impl Theme {
    /// Create a theme from a Catppuccin flavor.
    fn from_catppuccin(flavor: &catppuccin::Flavor) -> Self {
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
        }
    }

    /// Catppuccin Mocha theme (dark).
    pub fn catppuccin_mocha() -> Self {
        Self::from_catppuccin(&PALETTE.mocha)
    }

    /// Catppuccin Latte theme (light).
    pub fn catppuccin_latte() -> Self {
        Self::from_catppuccin(&PALETTE.latte)
    }

    /// Catppuccin Frappé theme (dark).
    pub fn catppuccin_frappe() -> Self {
        Self::from_catppuccin(&PALETTE.frappe)
    }

    /// Catppuccin Macchiato theme (dark).
    pub fn catppuccin_macchiato() -> Self {
        Self::from_catppuccin(&PALETTE.macchiato)
    }

    // Base colors
    pub fn base(&self) -> Color {
        self.base
    }

    pub fn mantle(&self) -> Color {
        self.mantle
    }

    pub fn crust(&self) -> Color {
        self.crust
    }

    // Surface colors
    pub fn surface0(&self) -> Color {
        self.surface0
    }

    pub fn surface1(&self) -> Color {
        self.surface1
    }

    pub fn surface2(&self) -> Color {
        self.surface2
    }

    // Overlay colors
    pub fn overlay0(&self) -> Color {
        self.overlay0
    }

    pub fn overlay1(&self) -> Color {
        self.overlay1
    }

    pub fn overlay2(&self) -> Color {
        self.overlay2
    }

    // Text colors
    pub fn text(&self) -> Color {
        self.text
    }

    pub fn subtext0(&self) -> Color {
        self.subtext0
    }

    pub fn subtext1(&self) -> Color {
        self.subtext1
    }

    // Accent colors
    pub fn rosewater(&self) -> Color {
        self.rosewater
    }

    pub fn flamingo(&self) -> Color {
        self.flamingo
    }

    pub fn pink(&self) -> Color {
        self.pink
    }

    pub fn mauve(&self) -> Color {
        self.mauve
    }

    pub fn red(&self) -> Color {
        self.red
    }

    pub fn maroon(&self) -> Color {
        self.maroon
    }

    pub fn peach(&self) -> Color {
        self.peach
    }

    pub fn yellow(&self) -> Color {
        self.yellow
    }

    pub fn green(&self) -> Color {
        self.green
    }

    pub fn teal(&self) -> Color {
        self.teal
    }

    pub fn sky(&self) -> Color {
        self.sky
    }

    pub fn sapphire(&self) -> Color {
        self.sapphire
    }

    pub fn blue(&self) -> Color {
        self.blue
    }

    pub fn lavender(&self) -> Color {
        self.lavender
    }

    // Semantic colors
    pub fn primary(&self) -> Color {
        self.blue
    }

    pub fn secondary(&self) -> Color {
        self.mauve
    }

    pub fn success(&self) -> Color {
        self.green
    }

    pub fn warning(&self) -> Color {
        self.yellow
    }

    pub fn error(&self) -> Color {
        self.red
    }

    pub fn info(&self) -> Color {
        self.sky
    }

    // UI element colors
    pub fn border(&self) -> Color {
        self.surface1
    }

    pub fn border_focused(&self) -> Color {
        self.lavender
    }

    pub fn selection_bg(&self) -> Color {
        self.surface1
    }

    pub fn selection_fg(&self) -> Color {
        self.text
    }

    pub fn header(&self) -> Color {
        self.yellow
    }

    pub fn highlight(&self) -> Color {
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
        ThemeInfo::new("Catppuccin Mocha", Theme::catppuccin_mocha()),
        ThemeInfo::new("Catppuccin Macchiato", Theme::catppuccin_macchiato()),
        ThemeInfo::new("Catppuccin Frappé", Theme::catppuccin_frappe()),
        ThemeInfo::new("Catppuccin Latte", Theme::catppuccin_latte()),
    ]
}
