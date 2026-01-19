use std::sync::{Arc, Mutex};

use arboard::Clipboard;
use tokio::sync::mpsc::UnboundedSender;

use crate::component::ToastType;
use crate::core::message::AppMessage;

/// Shared environment for commands.
///
/// Provides access to shared resources like clipboard and app messaging.
/// Clone is cheap (Arc-based) so it can be passed to multiple commands.
#[derive(Clone)]
pub struct CommandEnv {
    clipboard: Arc<Mutex<Option<Clipboard>>>,
    app_tx: UnboundedSender<AppMessage>,
}

impl CommandEnv {
    pub fn new(app_tx: UnboundedSender<AppMessage>) -> Self {
        Self {
            clipboard: Arc::new(Mutex::new(None)),
            app_tx,
        }
    }

    /// Copy text to the system clipboard.
    ///
    /// On Linux, the clipboard is held by the application, so the text remains
    /// available until the next copy or app exit.
    pub fn set_clipboard(&self, text: &str) -> color_eyre::Result<()> {
        let mut guard = self
            .clipboard
            .lock()
            .map_err(|e| color_eyre::eyre::eyre!("Failed to lock clipboard: {}", e))?;

        // Create clipboard on first use (lazy initialization)
        if guard.is_none() {
            *guard = Some(Clipboard::new()?);
        }

        if let Some(clipboard) = guard.as_mut() {
            clipboard.set_text(text)?;
        }

        Ok(())
    }

    /// Show a toast notification.
    pub fn show_toast(&self, message: impl Into<String>, toast_type: ToastType) {
        let _ = self.app_tx.send(AppMessage::ShowToast {
            message: message.into(),
            toast_type,
        });
    }
}
