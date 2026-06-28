//! Secret Manager service wiring.
//!
//! All lifecycle/connection/API-enable machinery lives in the shared
//! [`crate::provider::gcp::service`] harness. This module only declares what is
//! unique to Secret Manager: its domain messages, its per-service state, and how
//! to connect, dispatch, and handle external editor results.

use async_trait::async_trait;
use color_eyre::Result;

use crate::context::GcpContext;
use crate::provider::gcp::secret_manager::client::{ClientError, SecretManagerClient};
use crate::provider::gcp::secret_manager::payload::{self, PayloadMsg};
use crate::provider::gcp::secret_manager::secrets::{self, Secret, SecretsMsg};
use crate::provider::gcp::secret_manager::versions::{self, VersionsMsg};
use crate::provider::gcp::service::{ConnectError, GcpService, GcpServiceLogic, HostMsg};
use crate::service::ServiceMsg;

/// Concrete service type used throughout the Secret Manager feature slices.
pub type SecretManager = GcpService<SecretManagerLogic>;

/// The message type that flows through the Secret Manager service.
pub type SecretManagerMsg = HostMsg<SecretManagerLogic>;

/// Secret Manager-specific messages, dispatched to feature slices.
#[derive(Debug, Clone)]
pub enum SmDomainMsg {
    Secret(SecretsMsg),
    Version(VersionsMsg),
    Payload(PayloadMsg),
}

/// Per-service domain state owned by the harness.
pub struct SecretManagerLogic {
    /// Secret currently being edited in an external editor, if any.
    pub(super) editing_secret: Option<Secret>,
}

#[async_trait]
impl GcpServiceLogic for SecretManagerLogic {
    type Domain = SmDomainMsg;
    type Client = SecretManagerClient;

    const SERVICE_KEY: &'static str = "secret-manager";
    const DISPLAY_NAME: &'static str = "Secret Manager";
    const DESCRIPTION: &'static str = "Store and manage secrets, API keys, and certificates";
    const API_SERVICE: &'static str = "secretmanager.googleapis.com";

    fn new() -> Self {
        Self {
            editing_secret: None,
        }
    }

    async fn connect(context: &GcpContext) -> Result<Self::Client, ConnectError> {
        match SecretManagerClient::new(context).await {
            Ok(client) => Ok(client),
            Err(ClientError::ApiDisabled) => Err(ConnectError::ApiDisabled),
            Err(e) => Err(ConnectError::Other(e.into())),
        }
    }

    fn on_connected(host: &mut GcpService<Self>) {
        host.queue(SecretsMsg::Load.into());
    }

    fn update(host: &mut GcpService<Self>, msg: Self::Domain) -> Result<ServiceMsg> {
        match msg {
            SmDomainMsg::Secret(msg) => secrets::update(host, msg),
            SmDomainMsg::Version(msg) => versions::update(host, msg),
            SmDomainMsg::Payload(msg) => payload::update(host, msg),
        }
    }

    fn handle_editor_result(host: &mut GcpService<Self>, new_content: Option<String>) {
        if let Some(secret) = host.logic.editing_secret.take()
            && let Some(new_data) = new_content
        {
            host.queue(PayloadMsg::SaveEdit { secret, new_data }.into());
        }
    }
}
