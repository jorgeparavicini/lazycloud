use std::collections::HashMap;

use google_cloud_gax::error::rpc::Code;
use google_cloud_secretmanager_v1::client::SecretManagerService as GcpSecretManagerClient;
use google_cloud_secretmanager_v1::model;
use google_cloud_wkt::FieldMask;
use tokio_util::bytes::Bytes;
use tracing::{debug, info};

use crate::context::{CredentialError, GcpContext};
use crate::provider::gcp::secret_manager::payload::SecretPayload;
use crate::provider::gcp::secret_manager::secrets::{
    IamBinding,
    IamPolicy,
    Secret,
};
use crate::provider::gcp::secret_manager::versions::SecretVersion;

#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("Secret Manager API is not enabled for this project")]
    ApiDisabled,

    #[error("{message}")]
    Rpc { code: Code, message: String },

    #[error("{0}")]
    Credentials(#[from] CredentialError),

    #[error("{0}")]
    Other(String),
}

impl From<google_cloud_gax::error::Error> for ClientError {
    fn from(e: google_cloud_gax::error::Error) -> Self {
        if let Some(status) = e.status() {
            if status.code == Code::PermissionDenied
                && (status.message.contains("has not been used")
                    || status.message.contains("is disabled"))
            {
                return Self::ApiDisabled;
            }
            Self::Rpc {
                code: status.code,
                message: status.message.clone(),
            }
        } else {
            Self::Other(e.to_string())
        }
    }
}

#[derive(Clone, Debug)]
pub struct SecretManagerClient {
    client: GcpSecretManagerClient,
    project_id: String,
}

impl SecretManagerClient {
    /// Create a new `SecretManagerClient` with account-specific credentials.
    ///
    /// Uses the gcloud CLI credentials for the specified account.
    pub async fn new(context: &GcpContext) -> Result<Self, ClientError> {
        info!(
            project = %context.project_id,
            account = %context.account,
            "Creating Secret Manager credentials"
        );
        let credentials = context.create_credentials().await?;

        debug!("Credentials validated; building Secret Manager client");
        let client = GcpSecretManagerClient::builder()
            .with_credentials(credentials)
            .build()
            .await
            .map_err(|e| ClientError::Other(e.to_string()))?;

        info!("Secret Manager client ready");
        Ok(Self {
            client,
            project_id: context.project_id.clone(),
        })
    }

    pub async fn list_secrets(&self) -> Result<Vec<Secret>, ClientError> {
        let parent = format!("projects/{}", self.project_id);
        info!(%parent, "Listing secrets");

        let response = self.client.list_secrets().set_parent(parent).send().await?;
        debug!(count = response.secrets.len(), "Received secrets response");

        let mut secrets = Vec::new();
        for secret in response.secrets {
            if let Some(secret_id) = secret.name.rsplit('/').next() {
                let secret_id = secret_id.to_owned();
                secrets.push(Secret::from_proto(&secret_id, secret));
            }
        }
        Ok(secrets)
    }

    pub async fn list_versions(&self, secret_id: &str) -> Result<Vec<SecretVersion>, ClientError> {
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
                versions.push(SecretVersion::from_proto(name, &version));
            }
        }
        Ok(versions)
    }

    pub async fn access_version(&self, secret_id: &str, version_id: &str) -> Result<SecretPayload, ClientError> {
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
            Err(ClientError::Other(
                "No payload found for the secret version".into(),
            ))
        }
    }

    pub async fn access_latest_version(&self, secret_id: &str) -> Result<SecretPayload, ClientError> {
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
            Err(ClientError::Other(
                "No payload found for the latest secret version".into(),
            ))
        }
    }

    /// Create a new secret without an initial version.
    pub async fn create_secret(&self, secret_id: &str) -> Result<Secret, ClientError> {
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

        Ok(Secret::from_proto(secret_id, response))
    }

    /// Create a new secret with an initial payload.
    pub async fn create_secret_with_payload(
        &self,
        secret_id: &str,
        payload: &[u8],
    ) -> Result<Secret, ClientError> {
        // First create the secret
        let secret = self.create_secret(secret_id).await?;

        // Then add the initial version
        self.add_secret_version(secret_id, payload).await?;

        Ok(secret)
    }

    /// Delete a secret and all its versions.
    pub async fn delete_secret(&self, secret_id: &str) -> Result<(), ClientError> {
        let name = format!("projects/{}/secrets/{}", self.project_id, secret_id);

        self.client.delete_secret().set_name(name).send().await?;

        Ok(())
    }

    /// Add a new version to an existing secret.
    pub async fn add_secret_version(
        &self,
        secret_id: &str,
        payload: &[u8],
    ) -> Result<SecretVersion, ClientError> {
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
            .unwrap_or("unknown");

        Ok(SecretVersion::from_proto(version_id, &response))
    }

    /// Disable a secret version.
    pub async fn disable_version(
        &self,
        secret_id: &str,
        version_id: &str,
    ) -> Result<SecretVersion, ClientError> {
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

        Ok(SecretVersion::from_proto(version_id, &response))
    }

    /// Enable a previously disabled secret version.
    pub async fn enable_version(&self, secret_id: &str, version_id: &str) -> Result<SecretVersion, ClientError> {
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

        Ok(SecretVersion::from_proto(version_id, &response))
    }

    /// Destroy a secret version permanently.
    pub async fn destroy_version(
        &self,
        secret_id: &str,
        version_id: &str,
    ) -> Result<SecretVersion, ClientError> {
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

        Ok(SecretVersion::from_proto(version_id, &response))
    }

    /// Update secret labels.
    pub async fn update_labels(
        &self,
        secret_id: &str,
        labels: HashMap<String, String>,
    ) -> Result<Secret, ClientError> {
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

        Ok(Secret::from_proto(secret_id, response))
    }

    /// Get the IAM policy for a secret.
    pub async fn get_iam_policy(&self, secret_id: &str) -> Result<IamPolicy, ClientError> {
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
    pub async fn get_secret(&self, secret_id: &str) -> Result<Secret, ClientError> {
        let name = format!("projects/{}/secrets/{}", self.project_id, secret_id);
        let response = self.client.get_secret().set_name(name).send().await?;

        Ok(Secret::from_proto(secret_id, response))
    }
}
