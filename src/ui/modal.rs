//! Modal trait for ephemeral overlay dialogs.

use crate::ui::{Handled, Result};
use crate::Theme;
use crossterm::event::KeyEvent;
use ratatui::layout::Rect;
use ratatui::Frame;

/// Ephemeral overlay that blocks the screen below.
///
/// Modals are transient views for specific tasks like confirmations,
/// input dialogs, or multi-step wizards. They capture all input until
/// dismissed.
///
/// # Examples
///
/// - `DeleteSecretDialog` - confirmation dialog for deletion
/// - `CreateSecretWizard` - multi-step wizard for creation
/// - `CreateVersionDialog` - input dialog for new version
pub trait Modal {
    /// The message type this modal emits (same as parent screen's message type)
    type Msg;

    /// Handle a key event.
    ///
    /// Returns `Ok(Handled::...)` where:
    /// - `Ignored` - key was not handled (unusual for modals)
    /// - `Consumed` - key was handled but produced no message
    /// - `Event(msg)` - key was handled and produced a message
    ///
    /// Returns `Err(...)` if an error occurred during handling.
    fn handle_key(&mut self, key: KeyEvent) -> Result<Handled<Self::Msg>>;

    /// Render the modal (typically as a centered overlay).
    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme);

    /// Title shown in the modal header (optional).
    fn title(&self) -> Option<&str> {
        None
    }
}
