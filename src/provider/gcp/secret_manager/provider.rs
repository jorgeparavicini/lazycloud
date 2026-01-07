use crate::core::Service;
use crate::model::{CloudContext, Provider};
use crate::provider::gcp::secret_manager::SecretManager;
use crate::registry::ServiceProvider;

/// Provider for GCP Secret Manager.
pub struct SecretManagerProvider;

impl ServiceProvider for SecretManagerProvider {
    fn provider(&self) -> Provider {
        Provider::Gcp
    }

    fn service_key(&self) -> &'static str {
        "secret-manager"
    }

    fn display_name(&self) -> &'static str {
        "Secret Manager"
    }

    fn description(&self) -> &'static str {
        "Store and manage secrets, API keys, and certificates"
    }

    fn icon(&self) -> Option<&'static str> {
        Some("🔐")
    }

    fn create_service(&self, ctx: &CloudContext) -> Box<dyn Service> {
        let CloudContext::Gcp(gcp_ctx) = ctx else {
            panic!("SecretManagerProvider requires GcpContext");
        };
        Box::new(SecretManager::new(gcp_ctx))
    }
}
