use crate::model::Provider;
use std::fmt;

/// Unique identifier for a cloud service.
///
/// Combines the cloud provider with a service-specific key to create
/// a globally unique identifier.
///
/// # Example
///
/// ```ignore
/// let id = ServiceId::gcp("secret-manager");
/// assert_eq!(id.to_string(), "gcp:secret-manager");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ServiceId {
    /// The cloud provider
    pub provider: Provider,
    /// The service identifier (e.g., "secret-manager", "s3", "storage")
    pub service: String,
}

impl ServiceId {
    /// Create a new service ID.
    pub fn new(provider: Provider, service: impl Into<String>) -> Self {
        Self {
            provider,
            service: service.into(),
        }
    }

    /// Create a GCP service ID.
    pub fn gcp(service: impl Into<String>) -> Self {
        Self::new(Provider::Gcp, service)
    }

    /// Create an AWS service ID.
    pub fn aws(service: impl Into<String>) -> Self {
        Self::new(Provider::Aws, service)
    }

    /// Create an Azure service ID.
    pub fn azure(service: impl Into<String>) -> Self {
        Self::new(Provider::Azure, service)
    }
}

impl fmt::Display for ServiceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.provider, self.service)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_id_display() {
        let id = ServiceId::gcp("secret-manager");
        assert_eq!(id.to_string(), "gcp:secret-manager");
    }

    #[test]
    fn test_service_id_equality() {
        let id1 = ServiceId::gcp("secret-manager");
        let id2 = ServiceId::new(Provider::Gcp, "secret-manager");
        assert_eq!(id1, id2);
    }
}
