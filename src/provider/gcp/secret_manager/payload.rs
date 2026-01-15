use super::secrets::Secret;
use super::versions::SecretVersion;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretPayload {
    pub data: String,
    pub is_binary: bool,
}

#[derive(Debug, Clone)]
pub enum PayloadMsg {
    /// User selected a version to view its payload
    LoadPayload(Secret, Option<SecretVersion>),
    /// Copy payload to clipboard
    CopyPayload(String),   
}
