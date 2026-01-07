//! Async command pattern for side effects.
//!
//! Commands represent async operations that run outside the main event loop.
//! Services return commands, and the App spawns them with automatic
//! completion detection and status tracking.

mod clipboard;

pub use clipboard::CopyToClipboardCmd;

use async_trait::async_trait;

/// Async command that performs side effects.
///
/// Commands are spawned by the App and tracked for status display.
/// They typically send results back to the service via a channel.
#[async_trait]
pub trait Command: Send + 'static {
    /// Human-readable name for status display (e.g., "Loading secrets").
    fn name(&self) -> &'static str;

    /// Execute the command.
    async fn execute(self: Box<Self>) -> color_eyre::Result<()>;
}
