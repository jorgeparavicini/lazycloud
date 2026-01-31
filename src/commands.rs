//! Async commands pattern for side effects.
//!
//! Commands represent async operations that run outside the main event loop.
//! Services return commands, and the App spawns them with automatic
//! completion detection and status tracking.

mod clipboard;

use crate::app::AppMessage;
use async_trait::async_trait;
pub use clipboard::CopyToClipboardCmd;
use color_eyre::Result;
use tokio::sync::mpsc::UnboundedSender;

/// Async commands that perform side effects.
///
/// Commands are spawned by the App and tracked for status display.
/// They typically send results back to the service via a channel.
#[async_trait]
pub trait Command: Send + 'static {
    /// Human-readable name for status display.
    /// Include context like secret names, version IDs, etc.
    fn name(&self) -> String;

    /// Execute the commands.
    async fn execute(self: Box<Self>, action_tx: UnboundedSender<AppMessage>) -> Result<()>;
}
