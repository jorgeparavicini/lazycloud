//! Messages for Secret Manager service.
//!
//! All internal communication for the Secret Manager service flows through
//! this single message type, including lifecycle, user actions, and async results.

use crate::provider::gcp::secret_manager::client::SecretManagerClient;
use crate::provider::gcp::secret_manager::model::{Secret, SecretPayload, SecretVersion};

/// Messages for the Secret Manager service.
#[derive(Debug, Clone)]
pub enum SecretManagerMsg {
    // === Lifecycle ===
    /// Initialize the service (queued by init())
    Initialize,

    // === User Actions (from views) ===
    /// Navigate back to the previous view
    NavigateBack,
    /// Reload data for current view
    ReloadData,
    /// Load secrets list
    LoadSecrets,
    /// User selected a secret to view its versions
    SelectSecret(Secret),
    /// User selected a version to view its payload
    SelectVersion(Secret, SecretVersion),
    /// Copy payload to clipboard
    CopyPayload(String),

    // === Async Results (from commands) ===
    /// Client initialization completed
    ClientInitialized(SecretManagerClient),
    /// Secret list loaded from API
    SecretsLoaded(Vec<Secret>),
    /// Version list loaded for a secret
    VersionsLoaded {
        secret: Secret,
        versions: Vec<SecretVersion>,
    },
    /// Payload loaded for a specific version
    PayloadLoaded {
        secret: Secret,
        version: SecretVersion,
        payload: SecretPayload,
    },
    /// An operation failed
    OperationFailed(String),
}
