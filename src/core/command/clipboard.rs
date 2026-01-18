use crate::component::ToastType;
use crate::core::Command;
use crate::core::command::CommandEnv;
use async_trait::async_trait;

/// Command to copy text to the system clipboard.
///
/// On Linux, clipboard contents are only available while the owning process holds
/// the clipboard. Uses CommandEnv to access a shared clipboard that persists for
/// the lifetime of the application.
pub struct CopyToClipboardCmd {
    text: String,
    toast_message: String,
    env: CommandEnv,
}

impl CopyToClipboardCmd {
    pub fn new(text: impl Into<String>, toast_message: impl Into<String>, env: CommandEnv) -> Self {
        Self {
            text: text.into(),
            toast_message: toast_message.into(),
            env,
        }
    }
}

#[async_trait]
impl Command for CopyToClipboardCmd {
    fn name(&self) -> &'static str {
        "Copying to clipboard"
    }

    async fn execute(self: Box<Self>) -> color_eyre::Result<()> {
        self.env.set_clipboard(&self.text)?;
        self.env
            .show_toast(format!("Copied {}", self.toast_message), ToastType::Success);
        Ok(())
    }
}
