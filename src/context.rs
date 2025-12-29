use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Context {
    Aws(AwsContext),
    Azure(AzureContext),
    Gcp(GcpContext),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AwsContext {
    pub region: String,
    pub profile: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AzureContext {
    pub subscription_id: String,
    pub tenant_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GcpContext {
    pub project_id: String,
    pub service_account_path: String,
    pub zone: String,
}
