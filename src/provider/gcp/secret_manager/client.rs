use std::collections::HashMap;

use crate::context::GcpContext;
use crate::provider::gcp::secret_manager::payload::SecretPayload;
use crate::provider::gcp::secret_manager::secrets::{
    IamBinding, IamPolicy, ReplicationConfig, Secret,
};
use crate::provider::gcp::secret_manager::versions::SecretVersion;
use chrono::{DateTime, Utc};
use color_eyre::Result;
use google_cloud_secretmanager_v1::client::SecretManagerService as GcpSecretManagerClient;
use google_cloud_secretmanager_v1::model;
use google_cloud_wkt::FieldMask;
use tokio_util::bytes::Bytes;

#[derive(Clone, Debug)]
pub struct SecretManagerClient {
    client: GcpSecretManagerClient,
    project_id: String,
}

impl SecretManagerClient {
    /// Create a new `SecretManagerClient` with account-specific credentials.
    ///
    /// Uses the gcloud CLI credentials for the specified account.
    pub async fn new(context: &GcpContext) -> Result<Self> {
        let credentials = context.create_credentials()?;

        let client = GcpSecretManagerClient::builder()
            .with_credentials(credentials)
            .build()
            .await?;

        Ok(Self {
            client,
            project_id: context.project_id.clone(),
        })
    }

    pub async fn list_secrets(&self) -> Result<Vec<Secret>> {
        let parent = format!("projects/{}", self.project_id);

        let response = self.client.list_secrets().set_parent(parent).send().await?;

        let mut secrets = Vec::new();
        for secret in response.secrets {
            if let Some(name) = secret.name.split('/').next_back() {
                let replication = parse_replication(secret.replication.as_ref());
                let expire_time = secret
                    .expire_time()
                    .as_ref()
                    .map(|t| format_timestamp(t.seconds()));

                secrets.push(Secret {
                    name: name.to_string(),
                    replication,
                    created_at: secret
                        .create_time
                        .as_ref()
                        .map_or_else(|| "Unknown".to_string(), |t| format_timestamp(t.seconds())),
                    expire_time,
                    labels: secret.labels.clone(),
                });
            }
        }
        Ok(secrets)
    }

    pub async fn list_versions(&self, secret_id: &str) -> Result<Vec<SecretVersion>> {
        let parent = format!("projects/{}/secrets/{}", self.project_id, secret_id);

        let response = self
            .client
            .list_secret_versions()
            .set_parent(parent)
            .send()
            .await?;

        let mut versions = Vec::new();
        for version in response.versions {
            if let Some(name) = version.name.split('/').next_back() {
                versions.push(SecretVersion {
                    version_id: name.to_string(),
                    state: format!("{:?}", version.state),
                    created_at: version
                        .create_time
                        .as_ref()
                        .map_or_else(|| "Unknown".to_string(), |t| format_timestamp(t.seconds())),
                });
            }
        }
        Ok(versions)
    }

    pub async fn access_version(&self, secret_id: &str, version_id: &str) -> Result<SecretPayload> {
        let name = format!(
            "projects/{}/secrets/{}/versions/{}",
            self.project_id, secret_id, version_id
        );

        let response = self
            .client
            .access_secret_version()
            .set_name(name)
            .send()
            .await?;

        if let Some(payload) = response.payload {
            let data = String::from_utf8_lossy(&payload.data).to_string();
            Ok(SecretPayload {
                data,
                is_binary: false,
            })
        } else {
            Err(color_eyre::eyre::eyre!(
                "No payload found for the secret version"
            ))
        }
    }

    pub async fn access_latest_version(&self, secret_id: &str) -> Result<SecretPayload> {
        let name = format!(
            "projects/{}/secrets/{}/versions/latest",
            self.project_id, secret_id
        );

        let response = self
            .client
            .access_secret_version()
            .set_name(name)
            .send()
            .await?;

        if let Some(payload) = response.payload {
            let data = String::from_utf8_lossy(&payload.data).to_string();
            Ok(SecretPayload {
                data,
                is_binary: false,
            })
        } else {
            Err(color_eyre::eyre::eyre!(
                "No payload found for the latest secret version"
            ))
        }
    }

    /// Create a new secret without an initial version.
    pub async fn create_secret(&self, secret_id: &str) -> Result<Secret> {
        let parent = format!("projects/{}", self.project_id);

        let secret = model::Secret::default().set_replication(
            model::Replication::default().set_automatic(model::replication::Automatic::default()),
        );

        let response = self
            .client
            .create_secret()
            .set_parent(parent)
            .set_secret_id(secret_id)
            .set_secret(secret)
            .send()
            .await?;

        Ok(Secret {
            name: secret_id.to_string(),
            replication: parse_replication(response.replication.as_ref()),
            created_at: response
                .create_time
                .as_ref()
                .map_or_else(|| "Unknown".to_string(), |t| format_timestamp(t.seconds())),
            expire_time: response
                .expire_time()
                .map(|t| format_timestamp(t.seconds())),
            labels: response.labels,
        })
    }

    /// Create a new secret with an initial payload.
    pub async fn create_secret_with_payload(
        &self,
        secret_id: &str,
        payload: &[u8],
    ) -> Result<Secret> {
        // First create the secret
        let secret = self.create_secret(secret_id).await?;

        // Then add the initial version
        self.add_secret_version(secret_id, payload).await?;

        Ok(secret)
    }

    /// Delete a secret and all its versions.
    pub async fn delete_secret(&self, secret_id: &str) -> Result<()> {
        let name = format!("projects/{}/secrets/{}", self.project_id, secret_id);

        self.client.delete_secret().set_name(name).send().await?;

        Ok(())
    }

    /// Add a new version to an existing secret.
    pub async fn add_secret_version(
        &self,
        secret_id: &str,
        payload: &[u8],
    ) -> Result<SecretVersion> {
        let parent = format!("projects/{}/secrets/{}", self.project_id, secret_id);

        let payload_model = model::SecretPayload::default().set_data(Bytes::from(payload.to_vec()));

        let response = self
            .client
            .add_secret_version()
            .set_parent(parent)
            .set_payload(payload_model)
            .send()
            .await?;

        let version_id = response
            .name
            .split('/')
            .next_back()
            .unwrap_or("unknown")
            .to_string();

        Ok(SecretVersion {
            version_id,
            state: format!("{:?}", response.state),
            created_at: response
                .create_time
                .as_ref()
                .map_or_else(|| "Unknown".to_string(), |t| format_timestamp(t.seconds())),
        })
    }

    /// Disable a secret version.
    pub async fn disable_version(
        &self,
        secret_id: &str,
        version_id: &str,
    ) -> Result<SecretVersion> {
        let name = format!(
            "projects/{}/secrets/{}/versions/{}",
            self.project_id, secret_id, version_id
        );

        let response = self
            .client
            .disable_secret_version()
            .set_name(name)
            .send()
            .await?;

        Ok(SecretVersion {
            version_id: version_id.to_string(),
            state: format!("{:?}", response.state),
            created_at: response
                .create_time
                .as_ref()
                .map_or_else(|| "Unknown".to_string(), |t| format_timestamp(t.seconds())),
        })
    }

    /// Enable a previously disabled secret version.
    pub async fn enable_version(&self, secret_id: &str, version_id: &str) -> Result<SecretVersion> {
        let name = format!(
            "projects/{}/secrets/{}/versions/{}",
            self.project_id, secret_id, version_id
        );

        let response = self
            .client
            .enable_secret_version()
            .set_name(name)
            .send()
            .await?;

        Ok(SecretVersion {
            version_id: version_id.to_string(),
            state: format!("{:?}", response.state),
            created_at: response
                .create_time
                .as_ref()
                .map_or_else(|| "Unknown".to_string(), |t| format_timestamp(t.seconds())),
        })
    }

    /// Destroy a secret version permanently.
    pub async fn destroy_version(
        &self,
        secret_id: &str,
        version_id: &str,
    ) -> Result<SecretVersion> {
        let name = format!(
            "projects/{}/secrets/{}/versions/{}",
            self.project_id, secret_id, version_id
        );

        let response = self
            .client
            .destroy_secret_version()
            .set_name(name)
            .send()
            .await?;

        Ok(SecretVersion {
            version_id: version_id.to_string(),
            state: format!("{:?}", response.state),
            created_at: response
                .create_time
                .as_ref()
                .map_or_else(|| "Unknown".to_string(), |t| format_timestamp(t.seconds())),
        })
    }

    /// Update secret labels.
    pub async fn update_labels(
        &self,
        secret_id: &str,
        labels: HashMap<String, String>,
    ) -> Result<Secret> {
        let name = format!("projects/{}/secrets/{}", self.project_id, secret_id);

        let mut secret = model::Secret::default();
        secret.name.clone_from(&name);
        secret.labels.clone_from(&labels);

        let update_mask = FieldMask::default().set_paths(vec!["labels".to_string()]);

        let response = self
            .client
            .update_secret()
            .set_secret(secret)
            .set_update_mask(update_mask)
            .send()
            .await?;

        Ok(Secret {
            name: secret_id.to_string(),
            replication: parse_replication(response.replication.as_ref()),
            created_at: response
                .create_time
                .as_ref()
                .map_or_else(|| "Unknown".to_string(), |t| format_timestamp(t.seconds())),
            expire_time: response
                .expire_time()
                .map(|t| format_timestamp(t.seconds())),
            labels: response.labels,
        })
    }

    /// Get the IAM policy for a secret.
    pub async fn get_iam_policy(&self, secret_id: &str) -> Result<IamPolicy> {
        let resource = format!("projects/{}/secrets/{}", self.project_id, secret_id);

        let response = self
            .client
            .get_iam_policy()
            .set_resource(resource)
            .send()
            .await?;

        let bindings = response
            .bindings
            .into_iter()
            .map(|b| IamBinding {
                role: b.role,
                members: b.members,
            })
            .collect();

        Ok(IamPolicy { bindings })
    }

    /// Get secret metadata including replication configuration.
    pub async fn get_secret(&self, secret_id: &str) -> Result<Secret> {
        let name = format!("projects/{}/secrets/{}", self.project_id, secret_id);
        let response = self.client.get_secret().set_name(name).send().await?;

        Ok(Secret {
            name: secret_id.to_string(),
            replication: parse_replication(response.replication.as_ref()),
            created_at: response
                .create_time
                .as_ref()
                .map_or_else(|| "Unknown".to_string(), |t| format_timestamp(t.seconds())),
            expire_time: response
                .expire_time()
                .map(|t| format_timestamp(t.seconds())),
            labels: response.labels,
        })
    }
}

// === Utilities ===

fn format_timestamp(seconds: i64) -> String {
    DateTime::<Utc>::from_timestamp(seconds, 0).map_or_else(|| "Unknown".to_string(), |dt| dt.format("%Y-%m-%d %H:%M").to_string())
}

fn parse_replication(replication: Option<&model::Replication>) -> ReplicationConfig {
    let Some(replication) = replication else {
        return ReplicationConfig::Automatic;
    };
    let Some(ref rep) = replication.replication else {
        return ReplicationConfig::Automatic;
    };

    match rep {
        model::replication::Replication::UserManaged(user_managed) => {
            let locations = user_managed
                .replicas
                .iter()
                .map(|r| r.location.clone())
                .collect();
            ReplicationConfig::UserManaged { locations }
        }
        _ => ReplicationConfig::Automatic,
    }
}
