//! Service trait for cloud service screens.
//!
//! Services follow the Elm architecture with a single-funnel update pattern:
//! - `init()` queues initial message(s)
//! - `handle_input()` queues messages from user input
//! - `handle_tick()` handles animation ticks
//! - `update()` processes all queued messages - THE SINGLE FUNNEL
//!
//! Only `update()` can return commands, close the service, or report errors.
//! This ensures all side effects flow through one place.

use ratatui::Frame;
use ratatui::layout::Rect;

use crate::Theme;
use crate::component::Keybinding;
use crate::core::command::Command;
use crate::core::event::Event;

/// Result from `update()`
pub enum UpdateResult {
    /// No action needed
    Idle,
    /// Spawn these commands
    Commands(Vec<Box<dyn Command>>),
    /// Close this service (go back to service selection)
    Close,
    /// Report an error
    Error(String),
}

impl<T: Command> From<T> for UpdateResult {
    fn from(value: T) -> Self {
        UpdateResult::Commands(vec![Box::new(value)])
    }
}

/// A cloud service screen.
///
/// Services manage their own internal state and message queue. The App calls
/// methods in this order:
///
/// 1. `init()` - once when service becomes active
/// 2. `update()` - immediately after init to process startup message
/// 3. For each event:
///    - `handle_tick()` if tick event
///    - `handle_input()` if input event, then `update()` if consumed
/// 4. When command completes: `update()`
/// 5. `destroy()` - when service is closing
pub trait Service {
    /// Initialize the service by queuing startup message(s).
    ///
    /// Called once when the service becomes active. Queue your initial
    /// message (e.g., `Initialize`) here. The App will call `update()`
    /// immediately after to process it.
    fn init(&mut self) {}

    /// Clean up when the service is closing.
    fn destroy(&mut self) {}

    /// Handle a tick event for animations (spinners, etc.).
    ///
    /// Called on each tick. Use this for visual updates only.
    /// Do NOT queue messages here - just update animation state.
    fn handle_tick(&mut self) {}

    /// Handle an input event (keyboard, mouse).
    ///
    /// Queue internal messages based on user input. Return `true` if the
    /// event was consumed (the App will then call `update()`).
    fn handle_input(&mut self, event: &Event) -> bool;

    /// Process all queued messages and return the result.
    ///
    /// **THIS IS THE SINGLE FUNNEL.** This is the ONLY method that can:
    /// - Return commands to spawn
    /// - Request to close the service
    /// - Report errors
    ///
    /// Called by the App:
    /// - After `init()`
    /// - After `handle_input()` returns `true`
    /// - After any command completes
    fn update(&mut self) -> UpdateResult;

    /// Render the current state to the terminal.
    fn view(&mut self, frame: &mut Frame, area: Rect, theme: &Theme);

    /// Breadcrumb segments for the navigation bar.
    fn breadcrumbs(&self) -> Vec<String>;

    /// Returns the keybindings for the current view in this service.
    fn keybindings(&self) -> Vec<Keybinding> {
        vec![]
    }
}
