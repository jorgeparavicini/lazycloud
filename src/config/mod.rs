pub mod actions;
mod defaults;
pub mod key;
pub mod keybindings;
pub mod loader;
pub mod resolver;

pub use actions::*;
use keybindings::KeybindingsConfig;
pub use loader::{load, save_last_context, save_theme};
pub use resolver::KeyResolver;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeConfig {
    pub name: String,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            name: "Catppuccin Mocha".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    #[serde(default)]
    pub theme: ThemeConfig,
    #[serde(default)]
    pub keybindings: KeybindingsConfig,
    #[serde(default)]
    pub last_context: Option<String>,
}
