//! Cloud Storage service wiring.
//!
//! All lifecycle/connection/API-enable machinery lives in the shared
//! [`crate::provider::gcp::service`] harness. This module only declares what is
//! unique to GCS: its domain messages, its per-service state, and how to connect
//! and dispatch.

use async_trait::async_trait;
use color_eyre::Result;
use tokio::sync::mpsc::UnboundedSender;

use crate::context::GcpContext;
use crate::provider::gcp::gcs::buckets::{self, BucketsMsg};
use crate::provider::gcp::gcs::client::{ClientError, GcsClient};
use crate::provider::gcp::gcs::objects::{self, ObjectsMsg, PreviewContent};
use crate::provider::gcp::service::{ConnectError, GcpService, GcpServiceLogic, HostMsg};
use crate::service::ServiceMsg;

/// Concrete service type used throughout the GCS feature slices.
pub type Gcs = GcpService<GcsLogic>;

/// The message type that flows through the GCS service.
pub type GcsMsg = HostMsg<GcsLogic>;

/// GCS-specific messages, dispatched to feature slices.
#[derive(Debug, Clone)]
pub enum GcsDomainMsg {
    Bucket(BucketsMsg),
    Object(ObjectsMsg),
}

/// Per-service domain state owned by the harness.
pub struct GcsLogic {
    /// Channel feeding object previews to the active browser screen.
    pub(super) preview_tx: Option<UnboundedSender<PreviewContent>>,
}

#[async_trait]
impl GcpServiceLogic for GcsLogic {
    type Domain = GcsDomainMsg;
    type Client = GcsClient;

    const SERVICE_KEY: &'static str = "gcs";
    const DISPLAY_NAME: &'static str = "Cloud Storage";
    const DESCRIPTION: &'static str = "Manage Google Cloud Storage buckets and objects.";
    const API_SERVICE: &'static str = "storage.googleapis.com";

    fn new() -> Self {
        Self { preview_tx: None }
    }

    async fn connect(context: &GcpContext) -> Result<Self::Client, ConnectError> {
        match GcsClient::new(context).await {
            Ok(client) => Ok(client),
            Err(ClientError::ApiDisabled) => Err(ConnectError::ApiDisabled),
            Err(e) => Err(ConnectError::Other(e.into())),
        }
    }

    fn on_connected(host: &mut GcpService<Self>) {
        host.queue(BucketsMsg::Load.into());
    }

    fn update(host: &mut GcpService<Self>, msg: Self::Domain) -> Result<ServiceMsg> {
        match msg {
            GcsDomainMsg::Bucket(msg) => buckets::update(host, msg),
            GcsDomainMsg::Object(msg) => objects::update(host, msg),
        }
    }
}
