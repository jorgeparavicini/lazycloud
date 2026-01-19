use std::fs;
use std::path::PathBuf;

use crate::config::AppConfig;

const CONFIG_DIR: &str = "lazycloud";
const CONFIG_FILE: &str = "config.toml";

pub fn config_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|p| p.join(CONFIG_DIR))
}

pub fn config_path() -> Option<PathBuf> {
    config_dir().map(|p| p.join(CONFIG_FILE))
}

pub fn load() -> color_eyre::Result<AppConfig> {
    let path = match config_path() {
        Some(p) => p,
        None => {
            log::debug!("No config directory found, using defaults");
            return Ok(AppConfig::default());
        }
    };

    if !path.exists() {
        log::debug!("Config file not found at {:?}, using defaults", path);
        return Ok(AppConfig::default());
    }

    let content = fs::read_to_string(&path)?;
    let config: AppConfig = toml::from_str(&content)?;
    log::debug!("Loaded config from {:?}", path);
    Ok(config)
}

pub fn save(config: &AppConfig) -> color_eyre::Result<()> {
    let dir = match config_dir() {
        Some(p) => p,
        None => {
            log::warn!("Could not determine config directory");
            return Ok(());
        }
    };

    if !dir.exists() {
        fs::create_dir_all(&dir)?;
    }

    let path = dir.join(CONFIG_FILE);
    let content = toml::to_string_pretty(config)?;
    fs::write(&path, content)?;
    log::debug!("Saved config to {:?}", path);
    Ok(())
}

pub fn save_theme(theme_name: &str) -> color_eyre::Result<()> {
    let mut config = load().unwrap_or_default();
    config.theme.name = theme_name.to_string();
    save(&config)
}

pub fn save_last_context(context_name: &str) -> color_eyre::Result<()> {
    let mut config = load().unwrap_or_default();
    config.last_context = Some(context_name.to_string());
    save(&config)
}
