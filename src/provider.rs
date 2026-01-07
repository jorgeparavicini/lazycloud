//! Provider registration module.
//!
//! This module registers all available service providers with the registry.

pub mod gcp;

use crate::registry::ServiceRegistry;

/// Register all providers with the given registry.
pub fn register_all(registry: &mut ServiceRegistry) {
    gcp::register(registry);
}
