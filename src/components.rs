use crate::command::Command;
use crate::tui::Event;
use crossterm::event::KeyEvent;
use ratatui::Frame;
use ratatui::layout::Rect;

pub trait Component {
    fn name(&self) -> &str;

    fn render(&self, frame: &mut Frame, area: Rect) -> color_eyre::Result<()>;

    fn handle_event(&mut self, event: Event) -> color_eyre::Result<Option<Command>> {
        let command = match event {
            Event::Key(key_event) => self.handle_key_event(key_event)?,
            Event::Mouse(mouse_event) => self.handle_mouse_event(mouse_event)?,
            _ => None,
        };
        Ok(command)
    }

    fn handle_key_event(&mut self, key: KeyEvent) -> color_eyre::Result<Option<Command>> {
        let _ = key;
        Ok(None)
    }

    fn handle_mouse_event(
        &mut self,
        mouse_event: crossterm::event::MouseEvent,
    ) -> color_eyre::Result<Option<Command>> {
        let _ = mouse_event;
        Ok(None)
    }

    fn update(&mut self, command: Command) -> color_eyre::Result<Option<Command>> {
        let _ = command;
        Ok(None)
    }
}
