//! View components for the TUI.
//!
//! All views implement the [`View`] trait which provides a consistent interface
//! for handling keyboard input and rendering.

mod command_status;
mod context_selector;
mod help;
mod list;
mod service_selector;
mod spinner;
mod status_bar;
mod table;
mod theme_selector;

pub use command_status::{CommandId, CommandStatusView};
pub use context_selector::ContextSelectorView;
pub use help::{HelpEvent, HelpView, Keybinding};
pub use list::{ListEvent, ListRow, ListView};
pub use service_selector::ServiceSelectorView;
pub use spinner::SpinnerView;
pub use status_bar::StatusBarView;
pub use table::{ColumnDef, TableEvent, TableRow, TableView};
pub use theme_selector::{ThemeEvent, ThemeSelectorView};

use crate::Theme;
use crossterm::event::KeyEvent;
use ratatui::layout::Rect;
use ratatui::Frame;

/// Result of handling a key event.
pub enum KeyResult<E> {
    /// Key was not handled, parent should process it.
    Ignored,
    /// Key was consumed but produced no event.
    Consumed,
    /// Key was consumed and produced an event.
    Event(E),
}

impl<E> KeyResult<E> {
    /// Returns true if the key was consumed (not ignored).
    pub fn is_consumed(&self) -> bool {
        !matches!(self, KeyResult::Ignored)
    }

    /// Extract the event if present.
    pub fn into_event(self) -> Option<E> {
        match self {
            KeyResult::Event(e) => Some(e),
            _ => None,
        }
    }
}

impl<E> From<E> for KeyResult<E> {
    fn from(event: E) -> Self {
        KeyResult::Event(event)
    }
}

/// Trait for all view components.
///
/// Views manage their own internal UI state and return events when
/// user interactions occur that the parent should handle.
pub trait View {
    /// The event type returned by this view.
    type Event;

    /// Handle a key event.
    /// - `Ignored` - key was not handled, parent should process it
    /// - `Consumed` - key was handled but produced no event
    /// - `Event(e)` - key was handled and produced an event
    fn handle_key(&mut self, _key: KeyEvent) -> KeyResult<Self::Event> {
        KeyResult::Ignored
    }

    /// Called on each tick for animations and time-based updates.
    fn on_tick(&mut self) {}

    /// Render the view to the frame.
    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme);
}
