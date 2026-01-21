pub mod config;
pub mod secret_manager;

use crate::provider::gcp::secret_manager::SecretManagerProvider;
use crate::registry::ServiceRegistry;

/// Register all GCP services with the registry.
pub fn register(registry: &mut ServiceRegistry) {
    registry.register(SecretManagerProvider);
}
