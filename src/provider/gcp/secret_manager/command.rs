//! Commands for Secret Manager operations.
//!
//! These commands perform async operations and send results back
//! through the service's message channel.

use crate::core::Command;
use crate::provider::gcp::secret_manager::client::SecretManagerClient;
use crate::provider::gcp::secret_manager::message::SecretManagerMsg;
use crate::provider::gcp::secret_manager::model::{Secret, SecretVersion};
use async_trait::async_trait;
use tokio::sync::mpsc::UnboundedSender;

/// Initialize the Secret Manager client.
pub struct InitClientCmd {
    project_id: String,
    tx: UnboundedSender<SecretManagerMsg>,
}

impl InitClientCmd {
    pub fn new(project_id: String, tx: UnboundedSender<SecretManagerMsg>) -> Self {
        Self { project_id, tx }
    }
}

#[async_trait]
impl Command for InitClientCmd {
    fn name(&self) -> &'static str {
        "Initializing Secret Manager"
    }

    async fn execute(self: Box<Self>) -> color_eyre::Result<()> {
        match SecretManagerClient::new(self.project_id.clone()).await {
            Ok(client) => {
                let _ = self.tx.send(SecretManagerMsg::ClientInitialized(client));
            }
            Err(e) => {
                let _ = self.tx.send(SecretManagerMsg::OperationFailed(e.to_string()));
            }
        }
        Ok(())
    }
}

/// Fetch the list of secrets.
pub struct FetchSecretsCmd {
    client: SecretManagerClient,
    tx: UnboundedSender<SecretManagerMsg>,
}

impl FetchSecretsCmd {
    pub fn new(client: SecretManagerClient, tx: UnboundedSender<SecretManagerMsg>) -> Self {
        Self { client, tx }
    }
}

#[async_trait]
impl Command for FetchSecretsCmd {
    fn name(&self) -> &'static str {
        "Loading secrets"
    }

    async fn execute(self: Box<Self>) -> color_eyre::Result<()> {
        match self.client.list_secrets().await {
            Ok(secrets) => {
                let _ = self.tx.send(SecretManagerMsg::SecretsLoaded(secrets));
            }
            Err(e) => {
                let _ = self.tx.send(SecretManagerMsg::OperationFailed(e.to_string()));
            }
        }
        Ok(())
    }
}

/// Fetch versions for a secret.
pub struct FetchVersionsCmd {
    client: SecretManagerClient,
    secret: Secret,
    tx: UnboundedSender<SecretManagerMsg>,
}

impl FetchVersionsCmd {
    pub fn new(
        client: SecretManagerClient,
        secret: Secret,
        tx: UnboundedSender<SecretManagerMsg>,
    ) -> Self {
        Self { client, secret, tx }
    }
}

#[async_trait]
impl Command for FetchVersionsCmd {
    fn name(&self) -> &'static str {
        "Loading versions"
    }

    async fn execute(self: Box<Self>) -> color_eyre::Result<()> {
        match self.client.list_versions(&self.secret.name).await {
            Ok(versions) => {
                let _ = self.tx.send(SecretManagerMsg::VersionsLoaded {
                    secret: self.secret,
                    versions,
                });
            }
            Err(e) => {
                let _ = self.tx.send(SecretManagerMsg::OperationFailed(e.to_string()));
            }
        }
        Ok(())
    }
}

/// Fetch payload for a specific version.
pub struct FetchPayloadCmd {
    client: SecretManagerClient,
    secret: Secret,
    version: SecretVersion,
    tx: UnboundedSender<SecretManagerMsg>,
}

impl FetchPayloadCmd {
    pub fn new(
        client: SecretManagerClient,
        secret: Secret,
        version: SecretVersion,
        tx: UnboundedSender<SecretManagerMsg>,
    ) -> Self {
        Self {
            client,
            secret,
            version,
            tx,
        }
    }
}

#[async_trait]
impl Command for FetchPayloadCmd {
    fn name(&self) -> &'static str {
        "Loading secret payload"
    }

    async fn execute(self: Box<Self>) -> color_eyre::Result<()> {
        match self
            .client
            .access_version(&self.secret.name, &self.version.version_id)
            .await
        {
            Ok(payload) => {
                let _ = self.tx.send(SecretManagerMsg::PayloadLoaded {
                    secret: self.secret,
                    version: self.version,
                    payload,
                });
            }
            Err(e) => {
                let _ = self.tx.send(SecretManagerMsg::OperationFailed(e.to_string()));
            }
        }
        Ok(())
    }
}
