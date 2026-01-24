use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use crate::config::KeyResolver;
use crate::context::CloudContext;
use crate::provider::Provider;
use crate::service::Service;

/// Unique identifier for a cloud service.
///
/// Combines the cloud provider with a service-specific key to create
/// a globally unique identifier.
///
/// # Example
///
/// ```rust
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
    fn create_service(
        &self,
        ctx: &CloudContext,
        resolver: Arc<KeyResolver>,
    ) -> Box<dyn Service>;

    /// Check if this service is available for the given context.
    fn is_available(&self, ctx: &CloudContext) -> bool {
        self.provider() == ctx.provider()
    }
}


/// Registry of available cloud services.
///
/// The registry holds all registered service providers and provides
/// methods to query and filter them.
///
/// # Example
///
/// ```rust
/// let mut registry = ServiceRegistry::new();
///
/// // Register services
/// registry.register(SecretManagerProvider);
/// registry.register(S3Provider);
///
/// // Get services for a context
/// let services = registry.available_services(&context);
/// for service in services {
///     println!("{}: {}", service.display_name(), service.description());
/// }
/// ```
pub struct ServiceRegistry {
    providers: HashMap<ServiceId, Arc<dyn ServiceProvider>>,
}

impl ServiceRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
        }
    }

    /// Register a service provider.
    ///
    /// If a provider with the same service ID already exists, it will be replaced.
    pub fn register<P: ServiceProvider + 'static>(&mut self, provider: P) {
        let id = provider.service_id();
        self.providers.insert(id, Arc::new(provider));
    }

    /// Get a service provider by ID.
    pub fn get(&self, id: &ServiceId) -> Option<Arc<dyn ServiceProvider>> {
        self.providers.get(id).cloned()
    }

    /// Get all services for a specific cloud provider.
    pub fn services_for_provider(&self, provider: Provider) -> Vec<Arc<dyn ServiceProvider>> {
        self.providers
            .values()
            .filter(|p| p.provider() == provider)
            .cloned()
            .collect()
    }

    /// Get all services available for a given context.
    ///
    /// This filters services based on their `is_available` method,
    /// which by default checks if the provider matches.
    pub fn available_services(&self, ctx: &CloudContext) -> Vec<Arc<dyn ServiceProvider>> {
        self.providers
            .values()
            .filter(|p| p.is_available(ctx))
            .cloned()
            .collect()
    }

    /// Get all registered service IDs.
    pub fn all_service_ids(&self) -> Vec<ServiceId> {
        self.providers.keys().cloned().collect()
    }

    /// Get all registered service providers.
    pub fn all_providers(&self) -> Vec<Arc<dyn ServiceProvider>> {
        self.providers.values().cloned().collect()
    }

    /// Get the number of registered services.
    pub fn len(&self) -> usize {
        self.providers.len()
    }

    /// Check if the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.providers.is_empty()
    }
}

impl Default for ServiceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crossterm::event::KeyEvent;
    use ratatui::layout::Rect;
    use ratatui::Frame;

    use super::*;
    use crate::config::KeyResolver;
    use crate::context::{AuthMethod, GcpContext};
    use crate::service::{Service, ServiceMsg};
    use crate::ui::EventResult;
    use crate::Theme;

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

    struct MockProvider;

    impl ServiceProvider for MockProvider {
        fn provider(&self) -> Provider {
            Provider::Gcp
        }

        fn service_key(&self) -> &'static str {
            "mock-service"
        }

        fn display_name(&self) -> &'static str {
            "Mock Service"
        }

        fn create_service(
            &self,
            _ctx: &CloudContext,
            _resolver: Arc<KeyResolver>,
        ) -> Box<dyn Service> {
            Box::new(MockService)
        }
    }

    struct MockService;

    impl Service for MockService {
        fn handle_key(&mut self, _key: KeyEvent) -> EventResult<()> {
            EventResult::Ignored
        }

        fn update(&mut self) -> color_eyre::Result<ServiceMsg> {
            Ok(ServiceMsg::Idle)
        }

        fn render(&mut self, _frame: &mut Frame, _area: Rect, _theme: &Theme) {}

        fn breadcrumbs(&self) -> Vec<String> {
            vec!["Mock".to_string()]
        }
    }

    #[test]
    fn test_register_and_get() {
        let mut registry = ServiceRegistry::new();
        registry.register(MockProvider);

        let id = ServiceId::gcp("mock-service");
        assert!(registry.get(&id).is_some());
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn test_available_services() {
        let mut registry = ServiceRegistry::new();
        registry.register(MockProvider);

        let gcp_ctx = CloudContext::Gcp(GcpContext {
            display_name: "test-config".to_string(),
            project_id: "test".to_string(),
            account: "user@example.com".to_string(),
            region: Some("europe-west4".to_string()),
            zone: Some("europe-west4-a".to_string()),
            auth: AuthMethod::ApplicationDefault,
        });

        let services = registry.available_services(&gcp_ctx);
        assert_eq!(services.len(), 1);
    }
}
