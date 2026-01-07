mod client;
mod command;
mod message;
mod model;
mod service;
mod view;
mod provider;

use crossterm::event::KeyEvent;
use ratatui::layout::Rect;
use ratatui::Frame;

pub use message::SecretManagerMsg;
pub use service::SecretManager;
pub use provider::SecretManagerProvider;

/// Trait for Secret Manager views.
///
/// Each view handles keyboard input and renders its content.
pub trait SecretManagerView {
    /// Handle a key event, returning a message if an action was triggered.
    fn handle_key(&mut self, key: KeyEvent) -> Option<SecretManagerMsg>;

    /// Render the view to the frame.
    fn render(&mut self, frame: &mut Frame, area: Rect);
}
