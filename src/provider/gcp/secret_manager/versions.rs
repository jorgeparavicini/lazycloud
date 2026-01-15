use std::fmt::Display;
use crate::provider::gcp::secret_manager::secrets::Secret;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretVersion {
    pub version_id: String,
    pub state: String,
    pub created_at: String,
}

impl Display for SecretVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.version_id)
    }
}

#[derive(Debug, Clone)]
pub enum VersionsMsg {
    /// User selected a secret to view its versions
    LoadVersions(Secret),
    /// Show dialog to add a new version
    ShowCreateVersionDialog(Secret),
    /// Add a new version to a secret
    CreateVersion {
        secret: Secret,
        payload: String,
    },
    /// Disable a secret version
    DisableVersion {
        secret: Secret,
        version: SecretVersion,
    },
    /// Enable a secret version
    EnableVersion {
        secret: Secret,
        version: SecretVersion,
    },
    /// Show destroy confirmation for a version
    ShowDestroyVersionDialog {
        secret: Secret,
        version: SecretVersion,
    },
    /// Confirmed destruction of a version
    DestroyVersion {
        secret: Secret,
        version: SecretVersion,
    },
}
