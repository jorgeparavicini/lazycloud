//! Provider registration module.
//!
//! This module registers all available service providers with the registry.

pub mod gcp;

use std::fmt;
use crate::registry::ServiceRegistry;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Provider {
    Aws,
    Azure,
    Gcp,
}

impl Provider {
    /// Human-readable display name for the provider.
    pub const fn display_name(&self) -> &'static str {
        match self {
            Self::Aws => "AWS",
            Self::Azure => "Azure",
            Self::Gcp => "GCP",
        }
    }

    /// Short lowercase identifier for the provider.
    pub const fn id(&self) -> &'static str {
        match self {
            Self::Aws => "aws",
            Self::Azure => "azure",
            Self::Gcp => "gcp",
        }
    }
}

impl fmt::Display for Provider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.id())
    }
}

/// Register all providers with the given registry.
pub fn register_all(registry: &mut ServiceRegistry) {
    gcp::register(registry);
}
