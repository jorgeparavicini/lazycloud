use std::fs;
use std::path::PathBuf;

use color_eyre::Result;
use tracing::{debug, warn};
use crate::config::AppConfig;

const CONFIG_DIR: &str = "lazycloud";
const CONFIG_FILE: &str = "config.toml";

pub fn config_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|p| p.join(CONFIG_DIR))
}

pub fn config_path() -> Option<PathBuf> {
    config_dir().map(|p| p.join(CONFIG_FILE))
}

pub fn load() -> Result<AppConfig> {
    let Some(path) = config_path() else {
        debug!("No config directory found, using defaults");
        return Ok(AppConfig::default());
    };

    if !path.exists() {
        debug!(
            "Config file not found at {}, using defaults",
            path.display()
        );
        return Ok(AppConfig::default());
    }

    let content = fs::read_to_string(&path)?;
    let config: AppConfig = toml::from_str(&content)?;
    debug!("Loaded config from {}", path.display());
    Ok(config)
}

pub fn save(config: &AppConfig) -> Result<()> {
    let Some(dir) = config_dir() else {
        warn!("Could not determine config directory");
        return Ok(());
    };

    if !dir.exists() {
        fs::create_dir_all(&dir)?;
    }

    let path = dir.join(CONFIG_FILE);
    let content = toml::to_string_pretty(config)?;
    fs::write(&path, content)?;
    debug!("Saved config to {}", path.display());
    Ok(())
}

pub fn save_theme(theme_name: &str) -> Result<()> {
    let mut config = load().unwrap_or_default();
    config.theme.name = theme_name.to_string();
    save(&config)
}

pub fn save_last_context(context_name: &str) -> Result<()> {
    let mut config = load().unwrap_or_default();
    config.last_context = Some(context_name.to_string());
    save(&config)
}
