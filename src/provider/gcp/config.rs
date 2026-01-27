use std::fs;

use serde::Deserialize;

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

    let config_dir = match dirs::home_dir() {
        Some(dir) => dir.join(".config").join("gcloud").join("configurations"),
        None => return contexts,
    };

    if !config_dir.exists() {
        return contexts;
    }

    let Ok(entries) = fs::read_dir(config_dir) else {
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

        let Ok(content) = fs::read_to_string(&path) else {
            continue;
        };

        let Ok(mut config) = serini::from_str::<GcloudConfig>(&content) else {
            continue;
        };

        config.name = config_name;

        contexts.push(config);
    }

    contexts
}
