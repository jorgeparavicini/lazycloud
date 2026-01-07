use crate::model::provider::Provider;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Cloud context containing connection and authentication details.
///
/// Each variant holds provider-specific configuration needed to
/// authenticate and interact with that cloud provider's APIs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CloudContext {
    /// AWS context with profile and region
    Aws(AwsContext),
    /// Azure context with subscription and tenant
    Azure(AzureContext),
    /// GCP context with project and zone
    Gcp(GcpContext),
}

impl CloudContext {
    /// Get the provider for this context.
    pub fn provider(&self) -> Provider {
        match self {
            CloudContext::Aws(_) => Provider::Aws,
            CloudContext::Azure(_) => Provider::Azure,
            CloudContext::Gcp(_) => Provider::Gcp,
        }
    }

    /// Get a short display name for this context.
    pub fn name(&self) -> &str {
        match self {
            CloudContext::Aws(ctx) => &ctx.profile,
            CloudContext::Azure(ctx) => &ctx.subscription_id,
            CloudContext::Gcp(ctx) => &ctx.project_id,
        }
    }
}

impl fmt::Display for CloudContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CloudContext::Aws(ctx) => {
                write!(f, "AWS - Profile: {}, Region: {}", ctx.profile, ctx.region)
            }
            CloudContext::Azure(ctx) => write!(
                f,
                "Azure - Subscription: {}, Tenant: {}",
                ctx.subscription_id, ctx.tenant_id
            ),
            CloudContext::Gcp(ctx) => {
                write!(f, "GCP - Project: {}, Zone: {}", ctx.project_id, ctx.zone)
            }
        }
    }
}

/// AWS connection context.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AwsContext {
    /// AWS region (e.g., "us-east-1")
    pub region: String,
    /// AWS profile name from credentials file
    pub profile: String,
}

/// Azure connection context.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AzureContext {
    /// Azure subscription ID
    pub subscription_id: String,
    /// Azure tenant ID
    pub tenant_id: String,
}

/// GCP connection context.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GcpContext {
    /// GCP project ID
    pub project_id: String,
    /// Path to service account JSON file
    pub service_account_path: String,
    /// GCP zone (e.g., "us-central1-a")
    pub zone: String,
}

/// Load available cloud contexts.
///
/// TODO: Load from configuration file (~/.lazycloud/config.yaml)
pub fn load_contexts() -> Vec<CloudContext> {
    vec![CloudContext::Gcp(GcpContext {
        project_id: "vpc-host-prod-ug322-nt609".to_string(),
        service_account_path: "/path/to/service_account.json".to_string(),
        zone: "us-central1-a".to_string(),
    })]
}

/// Get all available cloud contexts.
///
/// This is an alias for [`load_contexts`] for API consistency.
pub fn get_available_contexts() -> Vec<CloudContext> {
    load_contexts()
}
