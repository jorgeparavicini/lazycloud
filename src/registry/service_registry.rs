use crate::model::{CloudContext, Provider};
use crate::registry::{ServiceId, ServiceProvider};
use std::collections::HashMap;
use std::sync::Arc;

/// Registry of available cloud services.
///
/// The registry holds all registered service providers and provides
/// methods to query and filter them.
///
/// # Example
///
/// ```ignore
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
    use super::*;
    use crate::Theme;
    use crate::config::KeyResolver;
    use crate::core::command::CommandEnv;
    use crate::core::event::Event;
    use crate::core::service::{Service, UpdateResult};
    use crate::model::GcpContext;
    use ratatui::Frame;
    use ratatui::layout::Rect;
    use std::sync::Arc;

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
            _cmd_env: CommandEnv,
        ) -> Box<dyn Service> {
            Box::new(MockService)
        }
    }

    struct MockService;

    impl Service for MockService {
        fn handle_input(&mut self, _event: &Event) -> bool {
            false
        }

        fn update(&mut self) -> UpdateResult {
            UpdateResult::Idle
        }

        fn view(&mut self, _frame: &mut Frame, _area: Rect, _theme: &Theme) {}

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
            config_name: "test-config".to_string(),
            project_id: "test".to_string(),
            account: "user@example.com".to_string(),
            region: Some("europe-west4".to_string()),
            credentials_path: None,
        });

        let services = registry.available_services(&gcp_ctx);
        assert_eq!(services.len(), 1);
    }
}
