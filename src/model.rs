//! Domain models for lazycloud.
//!
//! This module contains core domain types that are not UI-specific:
//! - [`Provider`] - Cloud provider enumeration (AWS, GCP, Azure)
//! - [`CloudContext`] - Connection/authentication context for a provider

pub mod context;
pub mod provider;

pub use context::{AwsContext, AzureContext, CloudContext, GcpContext};
pub use provider::Provider;
