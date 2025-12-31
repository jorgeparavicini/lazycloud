pub mod context_selector;
pub mod service_selector;
pub mod services;
pub mod status;

use crate::action::Action;
use crate::components::EventResult::Ignored;
use crate::tui::Event;
use crossterm::event::KeyEvent;
use ratatui::Frame;
use ratatui::layout::Rect;

pub enum EventResult {
    Ignored,
    Consumed(Option<Action>),
}

pub trait Component {
    fn handle_event(&mut self, event: Event) -> color_eyre::Result<EventResult> {
        let result = match event {
            Event::Key(key_event) => self.handle_key_event(key_event)?,
            Event::Mouse(mouse_event) => self.handle_mouse_event(mouse_event)?,
            _ => Ignored,
        };
        Ok(result)
    }

    fn handle_key_event(&mut self, key: KeyEvent) -> color_eyre::Result<EventResult> {
        let _ = key;
        Ok(Ignored)
    }

    fn handle_mouse_event(
        &mut self,
        mouse_event: crossterm::event::MouseEvent,
    ) -> color_eyre::Result<EventResult> {
        let _ = mouse_event;
        Ok(Ignored)
    }

    fn update(&mut self, command: Action) -> color_eyre::Result<Option<Action>> {
        let _ = command;
        Ok(None)
    }

    fn render(&mut self, frame: &mut Frame, area: Rect);

    fn breadcrumbs(&self) -> Vec<String> {
        vec![]
    }
}
