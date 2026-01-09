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
            CloudContext::Gcp(ctx) => &ctx.config_name,
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
                write!(
                    f,
                    "GCP - {} ({})",
                    ctx.config_name, ctx.project_id
                )
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
    /// Configuration name (e.g., "default", "prod")
    pub config_name: String,
    /// GCP project ID
    pub project_id: String,
    /// Account email (e.g., "user@example.com")
    pub account: String,
    /// Optional region from the configuration
    pub region: Option<String>,
    /// Path to credentials JSON file (if discovered)
    pub credentials_path: Option<String>,
}

/// Load available cloud contexts.
///
/// Discovers GCP contexts from gcloud CLI configurations.
/// TODO: Add support for AWS profiles and Azure subscriptions.
pub fn load_contexts() -> Vec<CloudContext> {
    use crate::provider::gcp::context::discover_gcloud_contexts;

    let mut contexts = Vec::new();

    // Discover GCP contexts from gcloud CLI
    if let Ok(gcp_contexts) = discover_gcloud_contexts() {
        for ctx in gcp_contexts {
            contexts.push(CloudContext::Gcp(GcpContext {
                config_name: ctx.config_name,
                project_id: ctx.project_id,
                account: ctx.account,
                region: ctx.region,
                credentials_path: ctx.credentials_path.map(|p| p.to_string_lossy().to_string()),
            }));
        }
    }

    contexts
}

/// Get all available cloud contexts.
///
/// This is an alias for [`load_contexts`] for API consistency.
pub fn get_available_contexts() -> Vec<CloudContext> {
    load_contexts()
}
