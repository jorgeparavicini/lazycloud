use crate::command::Command;
use crate::components::Component;
use crate::components::context_select::ContextSelector;
use crate::components::service_select::ServiceSelector;
use crate::components::services::secret_manager::SecretManager;
use crate::components::services::{GcpService, Service};
use crate::context::Context;
use crate::tui::{Event, Tui};
use crossterm::event::{KeyCode, KeyEvent};
use log::debug;
use ratatui::layout::Rect;
use tokio::sync::mpsc;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

enum Route {
    ContextSelect(ContextSelector),
    ServiceSelect(ServiceSelector),
    ActiveService(Box<dyn Component>),
}

pub struct AppContext {
    pub active_context: Option<Context>,
    pub command_tx: UnboundedSender<Command>,
}

pub struct App {
    route: Route,
    should_quit: bool,
    should_suspend: bool,
    app_context: AppContext,
    command_tx: UnboundedSender<Command>,
    command_rx: UnboundedReceiver<Command>,
}

impl App {
    pub fn new() -> Self {
        let (command_tx, command_rx) = mpsc::unbounded_channel();
        let app_context = AppContext {
            active_context: None,
            command_tx: command_tx.clone(),
        };
        Self {
            route: Route::ContextSelect(ContextSelector::new(&app_context)),
            should_quit: false,
            should_suspend: false,
            app_context,
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

        let command = match &mut self.route {
            Route::ContextSelect(component) => component.handle_event(event)?,
            Route::ServiceSelect(component) => component.handle_event(event)?,
            Route::ActiveService(component) => component.handle_event(event)?,
        };

        if let Some(command) = command {
            self.command_tx.send(command)?;
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

            match &command {
                Command::Tick => {
                    // TODO: Drain previously pressed keys
                }
                Command::Quit => self.should_quit = true,
                Command::Suspend => self.should_suspend = true,
                Command::Resume => self.should_suspend = false,
                Command::ClearScreen => tui.clear()?,
                Command::Resize(width, height) => self.handle_resize(tui, *width, *height)?,
                Command::Render => self.render(tui)?,
                Command::SelectContext(context) => {
                    self.app_context.active_context = Some(context.clone());
                    self.route = Route::ServiceSelect(ServiceSelector::new(&self.app_context));
                }
                Command::SelectService(service) => {
                    let service_component = self.create_service_component(service);
                    self.route = Route::ActiveService(service_component);
                }
                _ => {}
            }

            let result = match &mut self.route {
                Route::ContextSelect(component) => component.update(command),
                Route::ServiceSelect(component) => component.update(command),
                Route::ActiveService(component) => component.update(command),
            };

            if let Ok(Some(command)) = result {
                self.command_tx.send(command)?;
            } else if let Err(error) = result {
                self.command_tx.send(Command::DisplayError(format!(
                    "Error encountered while updating component: {}",
                    error
                )))?;
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
            let result = match &mut self.route {
                Route::ContextSelect(component) => component.render(frame, frame.area()),
                Route::ServiceSelect(component) => component.render(frame, frame.area()),
                Route::ActiveService(component) => component.render(frame, frame.area()),
            };

            if let Err(error) = result {
                self.command_tx
                    .send(Command::DisplayError(format!(
                        "Error encountered while rendering component: {}",
                        error
                    )))
                    .expect("Failed to send error command");
            }
        })?;
        Ok(())
    }

    fn create_service_component(&self, service: &Service) -> Box<dyn Component> {
        match service {
            Service::Gcp(GcpService::SecretManager) => {
                Box::new(SecretManager::new(&self.app_context))
            }
        }
    }
}
