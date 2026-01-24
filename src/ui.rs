pub mod components;
pub mod widgets;

mod command_panel;
mod error_dialog;
mod help;
mod status_bar;
mod toast;

use crossterm::event::KeyEvent;
use ratatui::layout::Rect;
use ratatui::Frame;

pub use color_eyre::Result;

use crate::Theme;

// Re-export components
pub use components::{ConfirmDialog, ConfirmEvent, ConfirmStyle, List, ListEvent, ListRow, Table, TableEvent, TableRow, ColumnDef, TextInput, TextInputEvent};

// Re-export widgets
pub use widgets::Spinner;

// Re-export app-level UI
pub use command_panel::{CommandId, CommandPanel};
pub use error_dialog::{ErrorDialog, ErrorDialogEvent};
pub use help::{HelpEvent, HelpOverlay, Keybinding, KeybindingSection};
pub use status_bar::StatusBar;
pub use toast::{Toast, ToastManager, ToastType};

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

impl<E> EventResult<E> {
    /// Returns true if the input was consumed (either with or without an event).
    pub fn is_consumed(&self) -> bool {
        !matches!(self, EventResult::Ignored)
    }
}

impl<E> From<E> for EventResult<E> {
    fn from(event: E) -> Self {
        EventResult::Event(event)
    }
}

/// Extension trait for processing `Result<EventResult<T>>` from component handlers.
pub trait EventResultExt<T> {
    /// Process the result into a tuple of (was_consumed, optional_message).
    fn process(self) -> (bool, Option<T>);
}

impl<T> EventResultExt<T> for Result<EventResult<T>> {
    fn process(self) -> (bool, Option<T>) {
        match self {
            Ok(EventResult::Event(msg)) => (true, Some(msg)),
            Ok(EventResult::Consumed) => (true, None),
            Ok(EventResult::Ignored) => (false, None),
            Err(_) => (false, None),
        }
    }
}

/// Interactive UI building block.
///
/// Components are reusable widgets that handle input events and emit
/// generic outputs. They know nothing about business logic.
///
/// # Examples
///
/// - `Table` - Selectable table with search/filter
/// - `TextInput` - Single-line text input
/// - `List` - Selectable list with navigation
pub trait Component {
    /// The output type produced by this component.
    ///
    /// # Examples
    /// - `Table` produces `TableEvent` to notify parent of row selection
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

    /// Render the component to the frame.
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
pub trait Modal {
    /// The message type produced by this modal.
    type Output;

    /// Handle a key event.
    fn handle_key(&mut self, key: KeyEvent) -> Result<EventResult<Self::Output>>;

    /// Render the modal to the frame.
    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme);

    /// Title shown in the modal header (optional).
    fn title(&self) -> Option<&str> {
        None
    }
}

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
    /// The message type produced by this screen.
    type Output;

    /// Handle a key event.
    fn handle_key(&mut self, key: KeyEvent) -> Result<EventResult<Self::Output>>;

    /// Render the screen to the frame.
    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme);

    /// Called on each tick for animations and time-based updates.
    fn handle_tick(&mut self) {}

    /// Breadcrumb segments for navigation context.
    fn breadcrumbs(&self) -> Vec<String> {
        vec![]
    }

    /// Returns the keybindings for this screen.
    fn keybindings(&self) -> Vec<Keybinding> {
        vec![]
    }
}
