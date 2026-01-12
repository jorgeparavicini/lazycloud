//! Messages for Secret Manager service.
//!
//! All internal communication for the Secret Manager service flows through
//! this single message type, including lifecycle, user actions, and async results.

use crate::provider::gcp::secret_manager::client::SecretManagerClient;
use crate::provider::gcp::secret_manager::model::{
    IamPolicy, ReplicationConfig, Secret, SecretPayload, SecretVersion,
};
use std::collections::HashMap;

/// Messages for the Secret Manager service.
#[derive(Debug, Clone)]
pub enum SecretManagerMsg {
    // === Lifecycle ===
    /// Initialize the service client
    Initialize,

    // === Navigation ===
    /// Navigate back to the previous view
    NavigateBack,
    /// Reload data for current view
    ReloadData,
    /// User cancelled a dialog
    DialogCancelled,

    // === Secrets ===
    /// Load secrets list
    LoadSecrets,
    /// Show the create secret dialog
    ShowCreateSecretDialog,
    /// Create a new secret (step 2: show payload input)
    CreateSecretStep2 { name: String },
    /// Create a new secret
    CreateSecret {
        name: String,
        payload: Option<String>,
    },
    /// Show delete confirmation for a secret
    ShowDeleteSecretDialog(Secret),
    /// Confirmed deletion of a secret
    DeleteSecret(Secret),

    // === Versions ===
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

    // === Payload ===
    /// User selected a version to view its payload
    LoadPayload(Secret, Option<SecretVersion>),
    /// Copy payload to clipboard
    CopyPayload(String),

    // === Metadata ===
    /// Show labels for a secret
    ShowLabels(Secret),
    /// Update labels for a secret
    UpdateLabels {
        secret: Secret,
        labels: HashMap<String, String>,
    },
    /// Show IAM policy for a secret
    ShowIamPolicy(Secret),
    /// Show replication info for a secret
    ShowReplicationInfo(Secret),

    // === Async Results ===
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
        version: Option<SecretVersion>,
        payload: SecretPayload,
    },
    /// Secret created successfully
    SecretCreated(Secret),
    /// Secret deleted successfully
    SecretDeleted(String),
    /// Version added successfully
    VersionAdded {
        secret: Secret,
    },
    /// Version disabled successfully
    VersionDisabled {
        secret: Secret,
    },
    /// Version enabled successfully
    VersionEnabled {
        secret: Secret,
    },
    /// Version destroyed successfully
    VersionDestroyed {
        secret: Secret,
    },
    /// Labels updated successfully
    LabelsUpdated(Secret),
    /// IAM policy loaded
    IamPolicyLoaded {
        secret: Secret,
        policy: IamPolicy,
    },
    /// Replication info loaded
    ReplicationInfoLoaded {
        secret: Secret,
        replication: ReplicationConfig,
    },
}
