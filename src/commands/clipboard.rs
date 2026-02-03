use arboard::Clipboard;
#[cfg(target_os = "linux")]
use arboard::SetExtLinux;
use async_trait::async_trait;
use color_eyre::Result;
use tokio::sync::mpsc::UnboundedSender;

use crate::app::AppMessage;
use crate::commands::Command;
use crate::ui::ToastType;

/// Copies a string to the system clipboard and shows a success toast notification.
pub struct CopyToClipboardCmd {
    text: String,
    toast_message: String,
}

impl CopyToClipboardCmd {
    pub fn new(text: impl Into<String>, toast_message: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            toast_message: toast_message.into(),
        }
    }
}

#[async_trait]
impl Command for CopyToClipboardCmd {
    fn name(&self) -> String {
        format!("Copying {}", self.toast_message)
    }

    async fn execute(self: Box<Self>, action_tx: UnboundedSender<AppMessage>) -> Result<()> {
        let mut clipboard = Clipboard::new()?;
        #[cfg(target_os = "linux")]
        clipboard.set().wait().text(self.text)?;

        #[cfg(not(target_os = "linux"))]
        clipboard.set_text(self.text)?;

        action_tx.send(AppMessage::ShowToast {
            message: format!("Copied {}", self.toast_message),
            toast_type: ToastType::Success,
        })?;
        Ok(())
    }
}
