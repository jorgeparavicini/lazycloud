use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Context {
    Aws(AwsContext),
    Azure(AzureContext),
    Gcp(GcpContext),
}

impl Context {
    pub fn name(&self) -> &str {
        match self {
            Context::Aws(aws) => &aws.profile,
            Context::Azure(azure) => &azure.subscription_id,
            Context::Gcp(gcp) => &gcp.project_id,
        }
    }
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

pub fn get_available_contexts() -> Vec<Context> {
    vec![
        Context::Gcp(GcpContext {
            project_id: "my-gcp-project".to_string(),
            service_account_path: "/path/to/service_account.json".to_string(),
            zone: "us-central1-a".to_string(),
        }),
    ]
}
