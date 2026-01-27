mod config;
pub mod secret_manager;

pub use crate::provider::gcp::config::discover_gcloud_configs;
use crate::provider::gcp::secret_manager::SecretManagerProvider;
use crate::registry::ServiceRegistry;

/// Register all GCP services with the registry.
pub fn register(registry: &mut ServiceRegistry) {
    registry.register(SecretManagerProvider);
}
