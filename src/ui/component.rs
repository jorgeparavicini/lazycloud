//! Component trait for reusable UI building blocks.

use crossterm::event::KeyEvent;
use ratatui::Frame;
use ratatui::layout::Rect;

use crate::Theme;
use crate::ui::{Handled, Result};

/// Interactive UI building block.
///
/// Components are reusable widgets that handle key events and emit
/// generic outputs. They know nothing about business logic.
///
/// # Examples
///
/// - `TableComponent` - Selectable table with search/filter
/// - `TextInputComponent` - Single-line text input
/// - `ListComponent` - Selectable list with navigation
pub trait Component {
    /// The output type this component produces (e.g., `TableEvent<T>`, `String`)
    type Output;

    /// Handle a key event.
    ///
    /// Returns `Ok(Handled::...)` where:
    /// - `Ignored` - key was not handled, parent should process it
    /// - `Consumed` - key was handled but produced no output
    /// - `Event(output)` - key was handled and produced an output
    ///
    /// Returns `Err(...)` if an error occurred during handling.
    fn handle_key(&mut self, key: KeyEvent) -> Result<Handled<Self::Output>> {
        _ = key;
        Ok(Handled::Ignored)
    }

    /// Called on each tick for animations and time-based updates.
    fn on_tick(&mut self) {}

    /// Render the component to the frame.
    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme);
}
