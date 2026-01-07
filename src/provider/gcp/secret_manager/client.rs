use crate::provider::gcp::secret_manager::model::{Secret, SecretPayload, SecretVersion};
use chrono::{DateTime, Utc};
use google_cloud_secretmanager_v1::client::SecretManagerService as GcpSecretManagerClient;

fn format_timestamp(seconds: i64) -> String {
    DateTime::<Utc>::from_timestamp(seconds, 0)
        .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_else(|| "Unknown".to_string())
}

#[derive(Clone, Debug)]
pub struct SecretManagerClient {
    client: GcpSecretManagerClient,
    project_id: String,
}

impl SecretManagerClient {
    pub async fn new(project_id: String) -> color_eyre::Result<Self> {
        let client = GcpSecretManagerClient::builder().build().await?;
        Ok(Self { client, project_id })
    }

    pub async fn list_secrets(&self) -> color_eyre::Result<Vec<Secret>> {
        let parent = format!("projects/{}", self.project_id);

        let response = self.client.list_secrets().set_parent(parent).send().await?;

        let mut secrets = Vec::new();
        for secret in response.secrets {
            if let Some(name) = secret.name.split('/').last() {
                secrets.push(Secret {
                    name: name.to_string(),
                    created_at: secret
                        .create_time
                        .as_ref()
                        .map_or("Unknown".to_string(), |t| format_timestamp(t.seconds())),
                    labels: secret.labels.clone(),
                });
            }
        }
        Ok(secrets)
    }

    pub async fn list_versions(&self, secret_id: &str) -> color_eyre::Result<Vec<SecretVersion>> {
        let parent = format!("projects/{}/secrets/{}", self.project_id, secret_id);

        let response = self
            .client
            .list_secret_versions()
            .set_parent(parent)
            .send()
            .await?;

        let mut versions = Vec::new();
        for version in response.versions {
            if let Some(name) = version.name.split('/').last() {
                versions.push(SecretVersion {
                    version_id: name.to_string(),
                    state: format!("{:?}", version.state),
                    created_at: version
                        .create_time
                        .as_ref()
                        .map_or("Unknown".to_string(), |t| format_timestamp(t.seconds())),
                });
            }
        }
        Ok(versions)
    }

    pub async fn access_version(
        &self,
        secret_id: &str,
        version_id: &str,
    ) -> color_eyre::Result<SecretPayload> {
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
}
