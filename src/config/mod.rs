pub mod actions;
mod defaults;
pub mod key;
pub mod keybindings;
pub mod loader;
pub mod resolver;

use keybindings::KeybindingsConfig;
use serde::{Deserialize, Serialize};

pub use actions::*;
pub use loader::{load, save_theme};
pub use resolver::KeyResolver;

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
}
