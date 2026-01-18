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

use crate::component::CommandId;
use crate::model::CloudContext;
use crate::registry::ServiceId;
use crate::theme::ThemeInfo;

#[derive(Debug, Clone)]
pub enum AppMessage {
    Tick,
    Render,
    Resize(u16, u16),
    Suspend,
    Resume,
    Quit,
    ClearScreen,

    DisplayError(String),
    DisplayHelp,
    DisplayThemeSelector,
    ClosePopup,

    CommandCompleted { id: CommandId, success: bool },
    ToggleCommandStatus,

    SelectContext(CloudContext),
    SelectService(ServiceId),
    SelectTheme(ThemeInfo),
    GoBack,
}
