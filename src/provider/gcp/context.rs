//! GCP context discovery from gcloud CLI configurations.
//!
//! Discovers GCP contexts by reading gcloud CLI configuration files.
//! Each gcloud configuration becomes a separate context with its own
//! project and credentials.

use std::path::PathBuf;

/// A discovered GCP context from gcloud CLI configuration.
#[derive(Debug, Clone)]
pub struct DiscoveredGcpContext {
    /// The gcloud configuration name (e.g., "default", "prod", "staging")
    pub config_name: String,
    /// GCP project ID
    pub project_id: String,
    /// Account email (e.g., "user@example.com")
    pub account: String,
    /// Optional region from the configuration
    pub region: Option<String>,
    /// Path to the credentials JSON file
    pub credentials_path: Option<PathBuf>,
}

/// Discover all gcloud configurations from the local system.
///
/// Reads configuration files from `~/.config/gcloud/configurations/config_*`
/// and returns a list of discovered contexts.
pub fn discover_gcloud_contexts() -> color_eyre::Result<Vec<DiscoveredGcpContext>> {
    let gcloud_dir = dirs::home_dir()
        .ok_or_else(|| color_eyre::eyre::eyre!("Could not find home directory"))?
        .join(".config/gcloud");

    let configs_dir = gcloud_dir.join("configurations");
    if !configs_dir.exists() {
        return Ok(Vec::new());
    }

    let mut contexts = Vec::new();

    for entry in std::fs::read_dir(&configs_dir)? {
        let entry = entry?;
        let path = entry.path();

        if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
            if filename.starts_with("config_") {
                let config_name = filename.strip_prefix("config_").unwrap().to_string();
                if let Ok(mut context) = parse_gcloud_config(&path, config_name) {
                    // Try to find credentials for this account
                    context.credentials_path = get_credentials_path(&context.account);
                    contexts.push(context);
                }
            }
        }
    }

    // Sort by config name for consistent ordering
    contexts.sort_by(|a, b| a.config_name.cmp(&b.config_name));

    Ok(contexts)
}

/// Parse a single gcloud configuration file (INI format).
fn parse_gcloud_config(
    path: &PathBuf,
    config_name: String,
) -> color_eyre::Result<DiscoveredGcpContext> {
    let content = std::fs::read_to_string(path)?;

    let mut project_id = None;
    let mut account = None;
    let mut region = None;

    for line in content.lines() {
        let line = line.trim();
        if line.starts_with("project") {
            project_id = line.split('=').nth(1).map(|s| s.trim().to_string());
        } else if line.starts_with("account") {
            account = line.split('=').nth(1).map(|s| s.trim().to_string());
        } else if line.starts_with("region") {
            region = line.split('=').nth(1).map(|s| s.trim().to_string());
        }
    }

    Ok(DiscoveredGcpContext {
        config_name,
        project_id: project_id
            .ok_or_else(|| color_eyre::eyre::eyre!("No project in config"))?,
        account: account.ok_or_else(|| color_eyre::eyre::eyre!("No account in config"))?,
        region,
        credentials_path: None,
    })
}

/// Get the path to credentials for a specific account.
///
/// Looks for credentials in `~/.config/gcloud/legacy_credentials/{account}/adc.json`.
pub fn get_credentials_path(account: &str) -> Option<PathBuf> {
    let gcloud_dir = dirs::home_dir()?.join(".config/gcloud");

    // Try legacy_credentials first (most common for user accounts)
    let legacy_path = gcloud_dir
        .join("legacy_credentials")
        .join(account)
        .join("adc.json");

    if legacy_path.exists() {
        return Some(legacy_path);
    }

    None
}

/// Load credentials JSON for a specific account.
pub fn load_credentials_json(account: &str) -> color_eyre::Result<serde_json::Value> {
    let path = get_credentials_path(account)
        .ok_or_else(|| color_eyre::eyre::eyre!("No credentials found for account: {}", account))?;

    let content = std::fs::read_to_string(&path)?;
    let json: serde_json::Value = serde_json::from_str(&content)?;
    Ok(json)
}
