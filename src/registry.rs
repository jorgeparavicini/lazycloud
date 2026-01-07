//! Service registry for dynamic service discovery.
//!
//! This module provides a plugin-like architecture for cloud services:
//! - [`ServiceId`] - Unique identifier for a service (provider + key)
//! - [`ServiceProvider`] - Trait for service implementations to register
//! - [`ServiceRegistry`] - Central registry for discovering services

pub mod service_id;
pub mod service_provider;
pub mod service_registry;

pub use service_id::ServiceId;
pub use service_provider::ServiceProvider;
pub use service_registry::ServiceRegistry;
