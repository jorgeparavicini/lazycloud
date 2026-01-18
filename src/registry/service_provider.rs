use crate::config::KeyResolver;
use crate::core::Service;
use crate::model::{CloudContext, Provider};
use crate::registry::ServiceId;
use std::sync::Arc;

/// Trait for cloud service providers.
///
/// Implement this trait to register a new cloud service with the registry.
/// The registry will use this to display available services and create
/// service instances when the user selects a service.
pub trait ServiceProvider: Send + Sync {
    /// The cloud provider this service belongs to.
    fn provider(&self) -> Provider;

    /// Unique service key within the provider (e.g., "secret-manager", "s3").
    fn service_key(&self) -> &'static str;

    /// Human-readable display name (e.g., "Secret Manager", "S3").
    fn display_name(&self) -> &'static str;

    /// Short description of what the service does.
    fn description(&self) -> &'static str {
        ""
    }

    /// Icon or emoji for the service (optional).
    fn icon(&self) -> Option<&'static str> {
        None
    }

    /// Construct the full service ID.
    fn service_id(&self) -> ServiceId {
        ServiceId::new(self.provider(), self.service_key())
    }

    /// Create a new service instance.
    fn create_service(&self, ctx: &CloudContext, resolver: Arc<KeyResolver>) -> Box<dyn Service>;

    /// Check if this service is available for the given context.
    fn is_available(&self, ctx: &CloudContext) -> bool {
        self.provider() == ctx.provider()
    }
}
