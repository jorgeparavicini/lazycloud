//! Async commands pattern for side effects.
//!
//! Commands represent async operations that run outside the main event loop.
//! Services return commands, and the App spawns them with automatic
//! completion detection and status tracking.

mod clipboard;

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
pub use clipboard::CopyToClipboardCmd;
use color_eyre::Result;

use crate::ui::ToastType;

/// Default wall-clock timeout applied to a command's execution.
///
/// Network/auth operations occasionally hang (e.g. a token fetch stuck
/// retrying transient errors). Capping execution turns an indefinite hang into
/// a surfaced error so the UI never gets stuck on a loading spinner forever.
pub const DEFAULT_COMMAND_TIMEOUT: Duration = Duration::from_secs(30);

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

    /// How long the command may run before the shell aborts it and surfaces a
    /// timeout error. Returning `None` opts out of the timeout entirely (use
    /// for genuinely long-running work such as polling an API-enable LRO).
    fn timeout(&self) -> Option<Duration> {
        Some(DEFAULT_COMMAND_TIMEOUT)
    }

    /// Execute the command.
    ///
    /// `ctx` exposes the (deliberately narrow) set of things a command may do
    /// to the surrounding application while running. Returning `Err` causes the
    /// App to surface the error to the user.
    async fn execute(self: Box<Self>, ctx: Arc<dyn CommandCtx>) -> Result<()>;
}
