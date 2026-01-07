//! Core framework for lazycloud.
//!
//! This module contains the foundational types and traits that power the TUI:
//! - [`Event`] - Input events from the terminal
//! - [`AppMessage`] - Internal communication between components
//! - [`Command`] - Async side effect operations
//! - [`Service`] - Cloud service screens (Elm-style architecture)
//! - [`Tui`] - Terminal wrapper

pub mod command;
pub mod event;
pub mod message;
pub mod service;
pub mod tui;

// Re-export commonly used types
pub use command::Command;
pub use message::AppMessage;
pub use service::{Service, UpdateResult};
