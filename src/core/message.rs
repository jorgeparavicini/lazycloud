//! Application-level messages.
//!
//! Messages represent internal communication within the application.
//! They flow between command and drive state changes.
//!
//! # Terminology
//! - **Event**: Input from the world (keyboard, mouse, timer) - see [`crate::core::event::Event`]
//! - **Message**: Internal communication between command
//! - **Command**: Async side effect operations - see [`crate::core::command::Command`]
//!
//! # Design
//! This enum only contains app-level messages for state transitions.
//! Service-specific messages are handled locally within each service
//! using their own message channels (e.g., `SecretManagerMsg`).

use crate::model::CloudContext;
use crate::registry::ServiceId;
use crate::theme::Theme;
use crate::view::CommandId;

/// Application-level messages for state transitions and global state.
#[derive(Debug, Clone)]
pub enum AppMessage {
    // === Lifecycle ===
    /// Periodic tick for animations and polling
    Tick,
    /// Render the UI
    Render,
    /// Terminal resized
    Resize(u16, u16),
    /// Suspend the application (Ctrl+Z)
    Suspend,
    /// Resume from suspension
    Resume,
    /// Quit the application
    Quit,
    /// Clear and redraw the command
    ClearScreen,

    // === Feedback ===
    /// Display an error to the user
    DisplayError(String),
    /// Display help overlay
    DisplayHelp,
    /// Display theme selector overlay
    DisplayThemeSelector,
    /// Close any open popup
    ClosePopup,

    // === Service ===
    /// A command completed, service should process pending messages
    CommandCompleted { id: CommandId, success: bool },
    /// Toggle expanded command status view
    ToggleCommandStatus,

    // === Phase Transitions ===
    /// User selected a cloud context, transition to service selection
    SelectContext(CloudContext),
    /// User selected a service, transition to active service
    SelectService(ServiceId),
    /// User selected a theme
    SelectTheme(Theme),
    /// Return to the previous state (service → service selector, service selector → context selector)
    GoBack,
}
