//! Async commands pattern for side effects.
//!
//! Commands represent async operations that run outside the main event loop.
//! Services return commands, and the App spawns them with automatic
//! completion detection and status tracking.

mod clipboard;

use std::sync::Arc;

use async_trait::async_trait;
pub use clipboard::CopyToClipboardCmd;
use color_eyre::Result;

use crate::ui::ToastType;

/// Capabilities a [`Command`] is allowed to use to talk back to the
/// application shell while it runs.
///
/// This is the narrow port that decouples commands from the concrete `App`.
/// Commands never import `crate::app`; the App provides an implementation and
/// hands it to each command. Adding a new capability here is a deliberate,
/// reviewable widening of what background work can do.
pub trait CommandCtx: Send + Sync {
    /// Show a transient toast notification to the user.
    fn toast(&self, message: String, toast_type: ToastType);
}

/// Async commands that perform side effects.
///
/// Commands are spawned by the App and tracked for status display.
/// They typically send results back to the service via a channel.
#[async_trait]
pub trait Command: Send + 'static {
    /// Human-readable name for status display.
    /// Include context like secret names, version IDs, etc.
    fn name(&self) -> String;

    /// Execute the command.
    ///
    /// `ctx` exposes the (deliberately narrow) set of things a command may do
    /// to the surrounding application while running. Returning `Err` causes the
    /// App to surface the error to the user.
    async fn execute(self: Box<Self>, ctx: Arc<dyn CommandCtx>) -> Result<()>;
}
