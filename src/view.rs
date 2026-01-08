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

/// Trait for all view components.
///
/// Views manage their own internal UI state and return events when
/// user interactions occur that the parent should handle.
pub trait View {
    /// The event type returned by this view.
    type Event;

    /// Handle a key event. Returns `Some(event)` if the key triggered an action.
    fn handle_key(&mut self, key: KeyEvent) -> Option<Self::Event>;

    /// Called on each tick for animations and time-based updates.
    fn on_tick(&mut self) {}

    /// Render the view to the frame.
    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme);
}
