//! UI trait hierarchy for the TUI.
//!
//! This module defines the core UI abstractions:
//! - [`Component`] - Reusable, interactive UI building blocks
//! - [`Screen`] - Full-page views that orchestrate components
//! - [`Modal`] - Ephemeral overlays that block the screen below
//! - [`Handled`] - Result of handling an input event

mod component;
mod modal;
mod screen;

pub use component::Component;
pub use modal::Modal;
pub use screen::Screen;

/// Result type alias for UI operations.
pub type Result<T> = std::result::Result<T, color_eyre::Report>;

/// Result of handling an input event.
///
/// This enum represents the three possible outcomes of handling a key event:
/// - `Ignored` - The handler didn't recognize or handle this input
/// - `Consumed` - The input was handled but produced no message
/// - `Event(E)` - The input was handled and produced a message
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Handled<E> {
    /// Input was not handled, parent should process it.
    Ignored,
    /// Input was consumed but produced no event.
    Consumed,
    /// Input was consumed and produced an event.
    Event(E),
}

impl<E> Handled<E> {
    /// Returns true if the input was consumed (not ignored).
    pub fn is_consumed(&self) -> bool {
        !matches!(self, Handled::Ignored)
    }

    /// Returns the event if present.
    pub fn event(self) -> Option<E> {
        match self {
            Handled::Event(e) => Some(e),
            _ => None,
        }
    }

    /// Maps the event type using the provided function.
    pub fn map<F, U>(self, f: F) -> Handled<U>
    where
        F: FnOnce(E) -> U,
    {
        match self {
            Handled::Ignored => Handled::Ignored,
            Handled::Consumed => Handled::Consumed,
            Handled::Event(e) => Handled::Event(f(e)),
        }
    }
}

impl<E> From<E> for Handled<E> {
    fn from(event: E) -> Self {
        Handled::Event(event)
    }
}

// Convenient conversion from Handled<E> to Result<Handled<E>>
impl<E> From<Handled<E>> for Result<Handled<E>> {
    fn from(handled: Handled<E>) -> Self {
        Ok(handled)
    }
}

/// Extension trait for processing `Result<Handled<E>>` in event handlers.
pub trait HandledResultExt<E> {
    /// Process the result, returning whether it was consumed and any event.
    ///
    /// Errors are treated as consumed (returns `(true, None)`).
    ///
    /// # Returns
    /// - `(true, Some(event))` - Input handled and produced an event
    /// - `(true, None)` - Input consumed without event (or error occurred)
    /// - `(false, None)` - Input ignored, caller should continue processing
    fn process(self) -> (bool, Option<E>);
}

impl<E> HandledResultExt<E> for Result<Handled<E>> {
    fn process(self) -> (bool, Option<E>) {
        match self {
            Ok(Handled::Event(e)) => (true, Some(e)),
            Ok(Handled::Consumed) => (true, None),
            Ok(Handled::Ignored) => (false, None),
            Err(_) => (true, None), // Treat errors as consumed
        }
    }
}
