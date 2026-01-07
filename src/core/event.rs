//! Event types and handling results.
//!
//! Events represent input from the external world (keyboard, mouse, timers).
//! They flow INTO the application from the TUI layer.

use crossterm::event::{KeyEvent, MouseEvent};

/// Events from the terminal/environment.
///
/// These are produced by the TUI event loop and consumed by screens.
#[derive(Clone, Debug)]
pub enum Event {
    /// Terminal initialized
    Init,
    /// Quit requested
    Quit,
    /// Error occurred in the event loop
    Error(String),
    /// Terminal closed
    Closed,
    /// Periodic tick (for animations, polling)
    Tick,
    /// Render frame requested
    Render,
    /// Terminal gained focus
    FocusGained,
    /// Terminal lost focus
    FocusLost,
    /// Text pasted from clipboard
    Paste(String),
    /// Key pressed
    Key(KeyEvent),
    /// Mouse event
    Mouse(MouseEvent),
    /// Terminal resized
    Resize(u16, u16),
}

/// Result of handling an event.
///
/// Screens return this to indicate whether they consumed an event
/// and optionally produce a message.
#[derive(Debug)]
pub enum EventResult<M> {
    /// Event was not handled, propagate to parent
    Ignored,
    /// Event was handled, optionally producing a message
    Consumed(Option<M>),
}

impl<M> EventResult<M> {
    /// Event was consumed without producing a message
    pub fn consumed() -> Self {
        Self::Consumed(None)
    }

    /// Event was consumed and produced a message
    pub fn with_message(msg: M) -> Self {
        Self::Consumed(Some(msg))
    }
}
