use std::collections::HashMap;
use std::fmt::Display;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Secret {
    pub name: String,
    pub replication: ReplicationConfig,
    pub created_at: String,
    pub expire_time: Option<String>,
    pub labels: HashMap<String, String>,
}

impl Display for Secret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretPayload {
    pub data: String,
    pub is_binary: bool,
}

/// An IAM binding representing a role and its members.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IamBinding {
    pub role: String,
    pub members: Vec<String>,
}

/// IAM policy for a secret.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IamPolicy {
    pub bindings: Vec<IamBinding>,
}

/// Replication configuration for a secret.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplicationConfig {
    /// Automatic replication managed by GCP.
    Automatic,
    /// User-managed replication with specific locations.
    UserManaged { locations: Vec<String> },
}

impl ReplicationConfig {
    /// Short display string for table column.
    pub fn short_display(&self) -> String {
        match self {
            ReplicationConfig::Automatic => "Automatic".to_string(),
            ReplicationConfig::UserManaged { locations } if locations.len() == 1 => {
                locations[0].clone()
            }
            ReplicationConfig::UserManaged { locations } => {
                format!("{} regions", locations.len())
            }
        }
    }
}
