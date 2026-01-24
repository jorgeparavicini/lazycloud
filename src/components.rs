mod command_status;
mod confirm_dialog;
mod error_dialog;
mod help;
mod list;
mod spinner;
mod status_bar;
mod table;
mod text_input;
mod theme_selector;
mod toast;

use crossterm::event::KeyEvent;
use ratatui::Frame;
use ratatui::layout::Rect;
use color_eyre::Result;
use crate::components::help::Keybinding;
use crate::Theme;
pub use crate::components::list::{ListRow, ListEvent, ListComponent};
pub use crate::components::toast::ToastType;

/// Result of handling an input event.
///
/// This enum represents the three possible outcomes of handling an input event:
/// - `Ignored` - The handler didn't recognize or handle this input
/// - `Consumed` - The input was handled but produced no message, the input will not be propagated further
/// - `Event(E)` - The input was handled and produced a message, the input will not be propagated further
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventResult<E> {
    /// Input was not handled, parent should process it.
    Ignored,
    /// Input was consumed but produced no event.
    Consumed,
    /// Input was consumed and produced an event.
    Event(E),
}

/// Interactive UI building block.
///
/// Components are reusable widgets that handle input events and emit
/// generic outputs. They know nothing about business logic.
///
/// # Examples
///
/// - `TableComponent` - Selectable table with search/filter
/// - `TextInputComponent` - Single-line text input
/// - `ListComponent` - Selectable list with navigation
pub trait Component {
    /// The output type produced by this components.
    ///
    /// # Examples
    /// - `TableComponent` produces `TableEvent` to notify parent of row selection
    type Output;

    /// Handle a key event.
    ///
    /// Returns `Ok(EventResult::...)` where:
    /// - `Ignored` - key was not handled, parent should process it
    /// - `Consumed` - key was handled but produced no output
    /// - `Event(output)` - key was handled and produced an output
    ///
    /// Returns `Err(...)` if an error occurred during handling.
    fn handle_key(&mut self, key: KeyEvent) -> Result<EventResult<Self::Output>> {
        _ = key;
        Ok(EventResult::Ignored)
    }

    /// Called on each tick for animations and time-based updates.
    fn handle_tick(&mut self) {}

    /// Render the components to the frame.
    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme);
}

/// Ephemeral overlay that blocks the screen below.
///
/// Modals are transient views for specific tasks like confirmations,
/// input dialogs, or multistep wizards. They capture all input until
/// dismissed.
///
/// # Examples
///
/// - `DeleteSecretDialog` - confirmation dialog for deletion
/// - `CreateSecretWizard` - multistep wizard for creation
/// - `CreateVersionDialog` - input dialog for creating a new version
pub trait Modal: Component {
    /// Title shown in the modal header (optional).
    fn title(&self) -> Option<&str> {
        None
    }
}

/// Full-page view that orchestrates components.
///
/// Screens connect UI interactions to business logic by translating
/// components events into domain messages. They know about the domain.
///
/// # Examples
///
/// - `SecretListScreen` - displays secrets table, emits `SecretManagerMsg`
/// - `VersionListScreen` - displays versions table, emits `SecretManagerMsg`
/// - `PayloadScreen` - displays secret payload with syntax highlighting
pub trait Screen: Component {
    /// Breadcrumb segments for navigation context.
    ///
    /// Returns a list of strings representing the navigation path.
    /// For example: `["Secrets", "my-secret", "Versions"]`
    fn breadcrumbs(&self) -> Vec<String> {
        vec![]
    }

    /// Returns the keybindings for this screen.
    fn keybindings(&self) -> Vec<Keybinding> {
        vec![]
    }
}
