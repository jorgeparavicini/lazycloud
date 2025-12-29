use serde::{Deserialize, Serialize};

pub mod secret_manager;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Service {
    Gcp(GcpService),
}

impl Service {
    pub fn name(&self) -> &str {
        match self {
            Service::Gcp(gcp_service) => match gcp_service {
                GcpService::SecretManager => "GCP Secret Manager",
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GcpService {
    SecretManager
}
