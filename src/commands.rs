//! Async commands pattern for side effects.
//!
//! Commands represent async operations that run outside the main event loop.
//! Services return commands, and the App spawns them with automatic
//! completion detection and status tracking.

mod clipboard;

use std::sync::{Arc, Mutex};

use arboard::Clipboard;
use async_trait::async_trait;
pub use clipboard::CopyToClipboardCmd;
use color_eyre::Result;
use color_eyre::eyre::eyre;
use tokio::sync::mpsc::UnboundedSender;

use crate::app::AppMessage;

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
    async fn execute(self: Box<Self>, env: CommandEnv) -> Result<()>;
}

/// Shared environment for commands.
///
/// Provides access to shared resources like clipboard and app messaging.
/// Clone is inexpensive (Arc-based) so it can be passed to multiple commands.
#[derive(Clone)]
pub struct CommandEnv {
    clipboard: Arc<Mutex<Option<Clipboard>>>,
    app_tx: UnboundedSender<AppMessage>,
}

// TODO: Remove this
impl CommandEnv {
    #[must_use]
    pub fn new(app_tx: UnboundedSender<AppMessage>) -> Self {
        Self {
            clipboard: Arc::new(Mutex::new(None)),
            app_tx,
        }
    }

    pub fn send<T: Into<AppMessage>>(&self, msg: T) {
        let _ = self.app_tx.send(msg.into());
    }

    /// Copy text to the system clipboard.
    ///
    /// # Errors
    /// Returns an error if clipboard access fails.
    pub fn set_clipboard(&self, text: &str) -> Result<()> {
        let mut guard = self
            .clipboard
            .lock()
            .map_err(|e| eyre!("Failed to lock clipboard: {}", e))?;

        // Create clipboard on first use (lazy initialization)
        if guard.is_none() {
            *guard = Some(Clipboard::new()?);
        }

        if let Some(clipboard) = guard.as_mut() {
            clipboard.set_text(text)?;
        }

        drop(guard);
        Ok(())
    }
}
