use std::fs;

use serde::Deserialize;
use tracing::{debug, error, info};

/// A discovered GCP context from gcloud CLI configuration.
#[derive(Deserialize)]
pub struct GcloudConfig {
    #[serde(default)]
    pub name: String,
    pub core: GcloudCoreConfig,
    pub compute: GcloudComputeConfig,
}

#[derive(Deserialize)]
pub struct GcloudCoreConfig {
    pub account: String,
    pub project: String,
}

#[derive(Deserialize)]
pub struct GcloudComputeConfig {
    pub zone: Option<String>,
    pub region: Option<String>,
}

/// Discover GCP contexts from gcloud CLI configurations.
///
/// Reads configuration files from the following location:
///  - Linux/Mac: `~/.config/gcloud/configurations`
///  - Windows: `%APPDATA%\gcloud\configurations`
pub fn discover_gcloud_configs() -> Vec<GcloudConfig> {
    let mut contexts = Vec::new();

    #[cfg(target_os = "macos")]
    let config_dir = match dirs::home_dir() {
        Some(dir) => dir.join(".config").join("gcloud").join("configurations"),
        None => {
            error!("Could not determine home directory for gcloud config");
            return contexts;
        }
    };

    #[cfg(not(target_os = "macos"))]
    let config_dir = match dirs::config_dir() {
        Some(dir) => dir.join("gcloud").join("configurations"),
        None => {
            error!("Could not determine config directory for gcloud config");
            return contexts;
        }
    };

    debug!(path = %config_dir.display(), "Searching for gcloud configurations");

    if !config_dir.exists() {
        debug!(path = %config_dir.display(), "gcloud configurations directory does not exist");
        return contexts;
    }

    let Ok(entries) = fs::read_dir(&config_dir) else {
        error!(path = %config_dir.display(), "Failed to read gcloud configurations directory");
        return contexts;
    };

    for entry in entries.flatten() {
        let path = entry.path();

        let Some(file_name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };

        if !file_name.starts_with("config_") {
            continue;
        }

        let config_name = file_name.trim_start_matches("config_").to_string();

        match fs::read_to_string(&path) {
            Ok(content) => match serini::from_str::<GcloudConfig>(&content) {
                Ok(mut config) => {
                    config.name = config_name;
                    debug!(name = %config.name, "Discovered gcloud config");
                    contexts.push(config);
                }
                Err(err) => {
                    error!(path = %path.display(), %err, "Failed to parse gcloud config file");
                }
            },
            Err(err) => {
                error!(path = %path.display(), %err, "Failed to read gcloud config file");
            }
        }
    }

    info!(count = contexts.len(), "GCP configuration discovery complete");
    contexts
}
