//! Screen trait for full-page views.

use crate::ui::{Handled, Result};
use crate::Theme;
use crossterm::event::KeyEvent;
use ratatui::layout::Rect;
use ratatui::Frame;

/// Full-page view that orchestrates components.
///
/// Screens connect UI interactions to business logic by translating
/// component events into domain messages. They know about the domain.
///
/// # Examples
///
/// - `SecretListScreen` - displays secrets table, emits `SecretManagerMsg`
/// - `VersionListScreen` - displays versions table, emits `SecretManagerMsg`
/// - `PayloadScreen` - displays secret payload with syntax highlighting
pub trait Screen {
    /// The message type this screen emits (e.g., `SecretManagerMsg`)
    type Msg;

    /// Handle a key event, possibly emitting a business message.
    ///
    /// Returns `Ok(Handled::...)` where:
    /// - `Ignored` - key was not handled, parent should process it
    /// - `Consumed` - key was handled but produced no message
    /// - `Event(msg)` - key was handled and produced a message
    ///
    /// Returns `Err(...)` if an error occurred during handling.
    fn handle_key(&mut self, key: KeyEvent) -> Result<Handled<Self::Msg>> {
        _ = key;
        Ok(Handled::Ignored)
    }

    /// Called on each tick for animations and time-based updates.
    fn on_tick(&mut self) {}

    /// Render the screen to the frame.
    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme);

    /// Breadcrumb segments for navigation context.
    ///
    /// Returns a list of strings representing the navigation path.
    /// For example: `["Secrets", "my-secret", "Versions"]`
    fn breadcrumbs(&self) -> Vec<String> {
        vec![]
    }
}
