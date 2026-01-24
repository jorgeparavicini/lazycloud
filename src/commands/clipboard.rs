use crate::app::AppMessage;
use crate::commands::{Command, CommandEnv};
use crate::ui::ToastType;
use async_trait::async_trait;

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

    async fn execute(self: Box<Self>, env: CommandEnv) -> color_eyre::Result<()> {
        env.set_clipboard(&self.text)?;
        env.send(AppMessage::ShowToast {
            message: format!("Copied {}", self.toast_message),
            toast_type: ToastType::Success,
        });
        Ok(())
    }
}
