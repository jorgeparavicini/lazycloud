use serde::{Deserialize, Serialize};

pub mod gcp;

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Service {
    Gcp(GcpService),
}

impl fmt::Display for Service {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
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
