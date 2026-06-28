mod config;
mod gcs;
pub mod secret_manager;
mod service;

pub use crate::provider::gcp::config::discover_gcloud_configs;
use crate::provider::gcp::gcs::GcsLogic;
use crate::provider::gcp::secret_manager::SecretManagerLogic;
use crate::provider::gcp::service::GcpProvider;
use crate::registry::ServiceRegistry;

/// Register all GCP services with the registry.
pub fn register(registry: &mut ServiceRegistry) {
    registry
        .register(GcpProvider::<SecretManagerLogic>::new())
        .register(GcpProvider::<GcsLogic>::new());
}
