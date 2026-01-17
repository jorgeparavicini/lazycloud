//! Event types and handling results.
//!
//! Events represent input from the external world (keyboard, mouse, timers).
//! They flow INTO the application from the TUI layer.

use crossterm::event::{KeyEvent, MouseEvent};

#[derive(Clone, Debug)]
pub enum Event {
    Init,
    Quit,
    Error(String),
    Closed,
    Tick,
    Render,
    FocusGained,
    FocusLost,
    Paste(String),
    Key(KeyEvent),
    Mouse(MouseEvent),
    Resize(u16, u16),
}

#[derive(Debug)]
pub enum EventResult<M> {
    Ignored,
    Consumed(Option<M>),
}

impl<M> EventResult<M> {
    pub fn consumed() -> Self {
        Self::Consumed(None)
    }

    pub fn with_message(msg: M) -> Self {
        Self::Consumed(Some(msg))
    }
}
