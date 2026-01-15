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
        let versions = self.client.list_versions(&self.secret.name).await?;
        self.tx.send(SecretManagerMsg::VersionsLoaded {
            secret: self.secret,
            versions,
        })?;
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
        let payload = self
            .client
            .access_version(&self.secret.name, &self.version.version_id)
            .await?;
        self.tx.send(SecretManagerMsg::PayloadLoaded {
            secret: self.secret,
            version: Some(self.version),
            payload,
        })?;
        Ok(())
    }
}

/// Fetch payload for the latest version.
pub struct FetchLatestPayloadCmd {
    client: SecretManagerClient,
    secret: Secret,
    tx: UnboundedSender<SecretManagerMsg>,
}

impl FetchLatestPayloadCmd {
    pub fn new(
        client: SecretManagerClient,
        secret: Secret,
        tx: UnboundedSender<SecretManagerMsg>,
    ) -> Self {
        Self { client, secret, tx }
    }
}

#[async_trait]
impl Command for FetchLatestPayloadCmd {
    fn name(&self) -> &'static str {
        "Loading latest secret payload"
    }

    async fn execute(self: Box<Self>) -> color_eyre::Result<()> {
        let payload = self.client.access_latest_version(&self.secret.name).await?;
        self.tx.send(SecretManagerMsg::PayloadLoaded {
            secret: self.secret,
            version: None,
            payload,
        })?;
        Ok(())
    }
}


/// Add a new version to a secret.
pub struct AddVersionCmd {
    client: SecretManagerClient,
    secret: Secret,
    payload: String,
    tx: UnboundedSender<SecretManagerMsg>,
}

impl AddVersionCmd {
    pub fn new(
        client: SecretManagerClient,
        secret: Secret,
        payload: String,
        tx: UnboundedSender<SecretManagerMsg>,
    ) -> Self {
        Self {
            client,
            secret,
            payload,
            tx,
        }
    }
}

#[async_trait]
impl Command for AddVersionCmd {
    fn name(&self) -> &'static str {
        "Adding secret version"
    }

    async fn execute(self: Box<Self>) -> color_eyre::Result<()> {
        self.client
            .add_secret_version(&self.secret.name, self.payload.as_bytes())
            .await?;
        self.tx
            .send(SecretManagerMsg::VersionAdded { secret: self.secret })?;
        Ok(())
    }
}

/// Disable a secret version.
pub struct DisableVersionCmd {
    client: SecretManagerClient,
    secret: Secret,
    version: SecretVersion,
    tx: UnboundedSender<SecretManagerMsg>,
}

impl DisableVersionCmd {
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
impl Command for DisableVersionCmd {
    fn name(&self) -> &'static str {
        "Disabling version"
    }

    async fn execute(self: Box<Self>) -> color_eyre::Result<()> {
        self.client
            .disable_version(&self.secret.name, &self.version.version_id)
            .await?;
        self.tx
            .send(SecretManagerMsg::VersionDisabled { secret: self.secret })?;
        Ok(())
    }
}

/// Enable a secret version.
pub struct EnableVersionCmd {
    client: SecretManagerClient,
    secret: Secret,
    version: SecretVersion,
    tx: UnboundedSender<SecretManagerMsg>,
}

impl EnableVersionCmd {
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
impl Command for EnableVersionCmd {
    fn name(&self) -> &'static str {
        "Enabling version"
    }

    async fn execute(self: Box<Self>) -> color_eyre::Result<()> {
        self.client
            .enable_version(&self.secret.name, &self.version.version_id)
            .await?;
        self.tx
            .send(SecretManagerMsg::VersionEnabled { secret: self.secret })?;
        Ok(())
    }
}

/// Destroy a secret version.
pub struct DestroyVersionCmd {
    client: SecretManagerClient,
    secret: Secret,
    version: SecretVersion,
    tx: UnboundedSender<SecretManagerMsg>,
}

impl DestroyVersionCmd {
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
impl Command for DestroyVersionCmd {
    fn name(&self) -> &'static str {
        "Destroying version"
    }

    async fn execute(self: Box<Self>) -> color_eyre::Result<()> {
        self.client
            .destroy_version(&self.secret.name, &self.version.version_id)
            .await?;
        self.tx
            .send(SecretManagerMsg::VersionDestroyed { secret: self.secret })?;
        Ok(())
    }
}

/// Update secret labels.
pub struct UpdateLabelsCmd {
    client: SecretManagerClient,
    secret: Secret,
    labels: std::collections::HashMap<String, String>,
    tx: UnboundedSender<SecretManagerMsg>,
}

impl UpdateLabelsCmd {
    pub fn new(
        client: SecretManagerClient,
        secret: Secret,
        labels: std::collections::HashMap<String, String>,
        tx: UnboundedSender<SecretManagerMsg>,
    ) -> Self {
        Self {
            client,
            secret,
            labels,
            tx,
        }
    }
}

#[async_trait]
impl Command for UpdateLabelsCmd {
    fn name(&self) -> &'static str {
        "Updating labels"
    }

    async fn execute(self: Box<Self>) -> color_eyre::Result<()> {
        let secret = self
            .client
            .update_labels(&self.secret.name, self.labels)
            .await?;
        self.tx.send(SecretManagerMsg::LabelsUpdated(secret))?;
        Ok(())
    }
}

/// Fetch IAM policy for a secret.
pub struct FetchIamPolicyCmd {
    client: SecretManagerClient,
    secret: Secret,
    tx: UnboundedSender<SecretManagerMsg>,
}

impl FetchIamPolicyCmd {
    pub fn new(
        client: SecretManagerClient,
        secret: Secret,
        tx: UnboundedSender<SecretManagerMsg>,
    ) -> Self {
        Self { client, secret, tx }
    }
}

#[async_trait]
impl Command for FetchIamPolicyCmd {
    fn name(&self) -> &'static str {
        "Loading IAM policy"
    }

    async fn execute(self: Box<Self>) -> color_eyre::Result<()> {
        let policy = self.client.get_iam_policy(&self.secret.name).await?;
        self.tx.send(SecretManagerMsg::IamPolicyLoaded {
            secret: self.secret,
            policy,
        })?;
        Ok(())
    }
}

/// Fetch secret metadata including replication info.
pub struct FetchSecretMetadataCmd {
    client: SecretManagerClient,
    secret: Secret,
    tx: UnboundedSender<SecretManagerMsg>,
}

impl FetchSecretMetadataCmd {
    pub fn new(
        client: SecretManagerClient,
        secret: Secret,
        tx: UnboundedSender<SecretManagerMsg>,
    ) -> Self {
        Self { client, secret, tx }
    }
}

#[async_trait]
impl Command for FetchSecretMetadataCmd {
    fn name(&self) -> &'static str {
        "Loading secret metadata"
    }

    async fn execute(self: Box<Self>) -> color_eyre::Result<()> {
        let secret = self.client.get_secret(&self.secret.name).await?;
        let replication = secret.replication.clone();
        self.tx.send(SecretManagerMsg::ReplicationInfoLoaded {
            secret,
            replication,
        })?;
        Ok(())
    }
}
