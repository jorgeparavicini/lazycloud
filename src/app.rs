use crate::action::Action;
use crate::components::Component;
use crate::components::context_selector::ContextSelector;
use crate::components::service_selector::ServiceSelector;
use crate::components::services::gcp::secret_manager::{SecretManager, SecretManagerAction};
use crate::components::services::{GcpService, Service};
use crate::components::status::Status;
use crate::context::Context;
use crate::tui::{Event, Tui};
use crossterm::event::{KeyCode, KeyEvent};
use log::debug;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::Paragraph;
use tokio::sync::mpsc;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use crate::components::EventResult::Consumed;

#[derive(Clone)]
pub struct AppContext {
    pub active_context: Option<Context>,
    pub action_tx: UnboundedSender<Action>,
}

impl AppContext {
    pub fn send_action(&self, action: Action) -> color_eyre::Result<()> {
        self.action_tx.send(action)?;
        Ok(())
    }
}

pub struct App {
    navigation_stack: Vec<Box<dyn Component>>,
    status: Status,
    should_quit: bool,
    should_suspend: bool,
    app_context: AppContext,
    action_tx: UnboundedSender<Action>,
    action_rx: UnboundedReceiver<Action>,
}

impl App {
    pub fn new() -> Self {
        let (command_tx, command_rx) = mpsc::unbounded_channel();
        let app_context = AppContext {
            active_context: None,
            action_tx: command_tx.clone(),
        };
        Self {
            navigation_stack: vec![Box::new(ContextSelector::new(&app_context))],
            status: Status::new(),
            should_quit: false,
            should_suspend: false,
            app_context,
            action_tx: command_tx,
            action_rx: command_rx,
        }
    }

    pub async fn run(&mut self) -> color_eyre::Result<()> {
        let mut tui = Tui::new(60.0, 4.0)?;
        tui.enter()?;

        loop {
            self.handle_events(&mut tui).await?;
            self.handle_actions(&mut tui)?;
            if self.should_suspend {
                tui.suspend()?;
                self.action_tx.send(Action::Resume)?;
                self.action_tx.send(Action::ClearScreen)?;
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

        let mut consumed = false;
        if let Some(component) = self.navigation_stack.last_mut() {
            let command = component.handle_event(event.clone())?;
            match command {
                Consumed(Some(command)) => {
                    self.action_tx.send(command)?;
                    consumed = true;
                }
                Consumed(None) => {
                    consumed = true;
                }
                _ => {}
            }
        }

        if !consumed {
            match event {
                Event::Quit => self.action_tx.send(Action::Quit)?,
                Event::Tick => self.action_tx.send(Action::Tick)?,
                Event::Render => self.action_tx.send(Action::Render)?,
                Event::Resize(width, height) => self.action_tx.send(Action::Resize(width, height))?,
                Event::Key(key_event) => self.handle_key_event(key_event)?,
                _ => {}
            }
        }

        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) -> color_eyre::Result<()> {
        let command = match key_event.code {
            KeyCode::Char('q') => Action::Quit,
            KeyCode::Char('h') => Action::DisplayHelp,
            KeyCode::Esc => Action::Pop,
            _ => return Ok(()),
        };
        self.action_tx.send(command)?;
        Ok(())
    }

    fn handle_actions(&mut self, tui: &mut Tui) -> color_eyre::Result<()> {
        while let Ok(command) = self.action_rx.try_recv() {
            if command != Action::Tick && command != Action::Render {
                debug!("Handling command: {:?}", command);
            }

            match &command {
                Action::Tick => {
                    // TODO: Drain previously pressed keys
                }
                Action::Quit => self.should_quit = true,
                Action::Suspend => self.should_suspend = true,
                Action::Resume => self.should_suspend = false,
                Action::ClearScreen => tui.clear()?,
                Action::Resize(width, height) => self.handle_resize(tui, *width, *height)?,
                Action::Render => self.render(tui)?,
                Action::SelectContext(context) => {
                    self.app_context.active_context = Some(context.clone());
                    self.navigation_stack
                        .push(Box::new(ServiceSelector::new(&self.app_context)));
                }
                Action::SelectService(service) => {
                    let service_component = self.create_service_component(service);
                    self.navigation_stack.push(service_component);
                }
                Action::Pop => {
                    if self.navigation_stack.len() > 1 {
                        self.navigation_stack.pop();
                    }
                }
                _ => {}
            }

            if let Ok(Some(action)) = self.status.update(command.clone()) {
                self.action_tx.send(action)?;
            }

            if let Some(component) = self.navigation_stack.last_mut() {
                let result = component.update(command);
                if let Ok(Some(command)) = result {
                    self.action_tx.send(command)?;
                } else if let Err(error) = result {
                    self.action_tx.send(Action::DisplayError(format!(
                        "Error encountered while updating component: {}",
                        error
                    )))?;
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
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3), // Status
                    Constraint::Min(0),    // Main content
                    Constraint::Length(1), // Breadcrumbs
                ])
                .split(frame.area());

            if let Some(component) = self.navigation_stack.last_mut() {
                component.render(frame, chunks[1]);
            }

            // Render Breadcrumbs
            let mut breadcrumbs = Vec::new();
            for component in &self.navigation_stack {
                breadcrumbs.extend(component.breadcrumbs());
            }
            let bc_text = breadcrumbs.join(" > ");
            let bc_paragraph = Paragraph::new(bc_text).style(
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::ITALIC),
            );
            frame.render_widget(bc_paragraph, chunks[2]);
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
