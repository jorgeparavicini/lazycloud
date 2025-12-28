use crate::command::Command;
use crate::components::Component;
use crate::tui::{Event, Tui};
use crossterm::event::{KeyCode, KeyEvent};
use log::debug;
use ratatui::layout::Rect;
use tokio::sync::mpsc;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

pub struct App {
    components: Vec<Box<dyn Component>>,
    should_quit: bool,
    should_suspend: bool,
    command_tx: UnboundedSender<Command>,
    command_rx: UnboundedReceiver<Command>,
}

impl App {
    pub fn new() -> Self {
        let (command_tx, command_rx) = mpsc::unbounded_channel();
        Self {
            components: Vec::new(),
            should_quit: false,
            should_suspend: false,
            command_tx,
            command_rx,
        }
    }

    pub async fn run(&mut self) -> color_eyre::Result<()> {
        let mut tui = Tui::new(60.0, 4.0)?;
        tui.enter()?;

        loop {
            self.handle_events(&mut tui).await?;
            self.handle_commands(&mut tui)?;
            if self.should_suspend {
                tui.suspend()?;
                self.command_tx.send(Command::Resume)?;
                self.command_tx.send(Command::ClearScreen)?;
                tui.enter()?;
            } else if self.should_quit {
                break;
            }
        }

        tui.exit()?;
        Ok(())
    }

    async fn handle_events(&mut self, tui: &mut Tui) -> color_eyre::Result<()> {
        let Some(event) = tui.next_event().await else {
            return Ok(());
        };

        match event {
            Event::Quit => self.command_tx.send(Command::Quit)?,
            Event::Tick => self.command_tx.send(Command::Tick)?,
            Event::Render => self.command_tx.send(Command::Render)?,
            Event::Resize(width, height) => self.command_tx.send(Command::Resize(width, height))?,
            Event::Key(key_event) => self.handle_key_event(key_event)?,
            _ => {}
        }

        for component in &mut self.components {
            if let Some(command) = component.handle_event(event.clone())? {
                self.command_tx.send(command)?;
            }
        }

        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) -> color_eyre::Result<()> {
        // TODO: handle keys and multi-key combinations
        let command = match key_event.code {
            KeyCode::Char('q') => Command::Quit,
            KeyCode::Char('h') => Command::DisplayHelp,
            _ => return Ok(()),
        };
        self.command_tx.send(command)?;
        Ok(())
    }

    fn handle_commands(&mut self, tui: &mut Tui) -> color_eyre::Result<()> {
        while let Ok(command) = self.command_rx.try_recv() {
            if command != Command::Tick && command != Command::Render {
                debug!("Handling command: {:?}", command);
            }

            match command {
                Command::Tick => {
                    // TODO: Drain previously pressed keys
                }
                Command::Quit => self.should_quit = true,
                Command::Suspend => self.should_suspend = true,
                Command::Resume => self.should_suspend = false,
                Command::ClearScreen => tui.clear()?,
                Command::Resize(width, height) => self.handle_resize(tui, width, height)?,
                Command::Render => self.render(tui)?,
                _ => {}
            }

            for component in &mut self.components {
                if let Some(command) = component.update(command.clone())? {
                    self.command_tx.send(command)?;
                }
            }
        }
        Ok(())
    }

    fn handle_resize(&mut self, tui: &mut Tui, width: u16, height: u16) -> color_eyre::Result<()> {
        tui.resize(Rect::new(0, 0, width, height))?;
        self.render(tui)?;
        Ok(())
    }

    fn render(&mut self, tui: &mut Tui) -> color_eyre::Result<()> {
        tui.draw(|frame| {
            for component in &mut self.components {
                if let Err(error) = component.render(frame, frame.area()) {
                    self.command_tx
                        .send(Command::DisplayError(format!(
                            "Component '{}' failed to render: {}",
                            component.name(),
                            error
                        )))
                        .expect("Failed to send error command");
                }
            }
        })?;
        Ok(())
    }
}
