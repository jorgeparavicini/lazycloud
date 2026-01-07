use arboard::Clipboard;
use async_trait::async_trait;
use crate::core::Command;

/// Command to copy text to the system clipboard.
pub struct CopyToClipboardCmd {
    text: String,
}

impl CopyToClipboardCmd {
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }
}

#[async_trait]
impl Command for CopyToClipboardCmd {
    fn name(&self) -> &'static str {
        "Copying to clipboard"
    }

    async fn execute(self: Box<Self>) -> color_eyre::Result<()> {
        let mut clipboard = Clipboard::new()?;
        clipboard.set_text(&self.text)?;
        Ok(())
    }
}
