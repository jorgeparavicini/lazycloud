use crate::core::command::Command;
use crate::core::event::Event;
use crate::core::message::AppMessage;
use crate::core::service::{Service, UpdateResult};
use crate::core::tui::Tui;
use crate::model::CloudContext;
use crate::registry::ServiceRegistry;
use crate::view::{
    CommandStatusView, ContextSelectorView, HelpEvent, HelpView, Keybinding, KeyResult,
    ServiceSelectorView, StatusBarView, ThemeEvent, ThemeSelectorView, View,
};
use crate::Theme;
use crossterm::event::KeyCode;
use log::debug;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Paragraph};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

/// Global keybindings shown in the help overlay.
const GLOBAL_KEYBINDINGS: &[Keybinding] = &[
    Keybinding::new("?", "Toggle help"),
    Keybinding::new("t", "Select theme"),
    Keybinding::new("q / Ctrl+C", "Quit application"),
    Keybinding::new("Esc", "Go back / Close"),
    Keybinding::new("Enter", "Select item"),
    Keybinding::new("j / Down", "Move down"),
    Keybinding::new("k / Up", "Move up"),
    Keybinding::new("r", "Reload current view"),
    Keybinding::new("y", "Copy to clipboard"),
    Keybinding::new("c", "Toggle command status"),
];

/// Application state - what the user is currently doing.
enum AppState {
    /// Selecting a cloud context (GCP project, AWS account, etc.)
    SelectingContext(ContextSelectorView),
    /// Selecting a service within the chosen context
    SelectingService(ServiceSelectorView),
    /// Using an active cloud service
    ActiveService(Box<dyn Service>),
}

/// Active popup overlay - only one can be open at a time.
enum ActivePopup {
    Help(HelpView),
    ThemeSelector(ThemeSelectorView),
}

pub struct App {
    state: AppState,
    theme: Theme,
    popup: Option<ActivePopup>,
    status_bar: StatusBarView,
    command_tracker: CommandStatusView,
    should_quit: bool,
    should_suspend: bool,
    active_context: Option<CloudContext>,
    registry: Arc<ServiceRegistry>,
    msg_tx: UnboundedSender<AppMessage>,
    msg_rx: UnboundedReceiver<AppMessage>,
}

impl App {
    pub fn new(registry: ServiceRegistry) -> Self {
        let (msg_tx, msg_rx) = mpsc::unbounded_channel();

        Self {
            state: AppState::SelectingContext(ContextSelectorView::new()),
            theme: Theme::default(),
            popup: None,
            status_bar: StatusBarView::new(),
            command_tracker: CommandStatusView::new(),
            should_quit: false,
            should_suspend: false,
            active_context: None,
            registry: Arc::new(registry),
            msg_tx,
            msg_rx,
        }
    }

    pub async fn run(&mut self) -> color_eyre::Result<()> {
        let mut tui = Tui::new(60.0, 4.0)?;
        tui.enter()?;

        loop {
            tokio::select! {
                event = tui.next_event() => {
                    if let Some(event) = event {
                        self.handle_event(&event)?;
                    }
                }
                Some(message) = self.msg_rx.recv() => {
                    self.handle_message(&mut tui, message)?;
                }
            }

            if self.should_suspend {
                tui.suspend()?;
                self.msg_tx.send(AppMessage::Resume)?;
                self.msg_tx.send(AppMessage::ClearScreen)?;
                tui.enter()?;
            } else if self.should_quit {
                break;
            }
        }

        tui.exit()?;
        Ok(())
    }

    /// Spawn commands and signal when complete.
    fn spawn_commands(&mut self, commands: Vec<Box<dyn Command>>) {
        for cmd in commands {
            let id = self.command_tracker.start(cmd.name());
            let msg_tx = self.msg_tx.clone();
            tokio::spawn(async move {
                let success = match cmd.execute().await {
                    Ok(()) => true,
                    Err(e) => {
                        let _ = msg_tx.send(AppMessage::DisplayError(e.to_string()));
                        false
                    }
                };
                // Signal that a command completed - service should process messages
                let _ = msg_tx.send(AppMessage::CommandCompleted { id, success });
            });
        }
    }

    /// Process the result from service.update().
    fn process_update_result(&mut self, result: UpdateResult) {
        match result {
            UpdateResult::Idle => {}
            UpdateResult::Commands(commands) => {
                self.spawn_commands(commands);
            }
            UpdateResult::Close => {
                let _ = self.msg_tx.send(AppMessage::GoBack);
            }
            UpdateResult::Error(err) => {
                let _ = self.msg_tx.send(AppMessage::DisplayError(err));
            }
        }
    }

    /// Transition to context selection.
    fn go_to_context_selection(&mut self) {
        self.active_context = None;
        self.status_bar.clear_context();
        self.state = AppState::SelectingContext(ContextSelectorView::new());
    }

    /// Transition to service selection.
    fn go_to_service_selection(&mut self, context: CloudContext) {
        self.active_context = Some(context.clone());
        self.status_bar.set_active_context(context.clone());
        self.state =
            AppState::SelectingService(ServiceSelectorView::new(self.registry.clone(), context));
    }

    /// Transition to active service.
    fn go_to_active_service(&mut self, mut service: Box<dyn Service>) {
        // Initialize the service (queues startup message)
        service.init();
        self.state = AppState::ActiveService(service);

        // Immediately process the startup message
        if let AppState::ActiveService(service) = &mut self.state {
            let result = service.update();
            self.process_update_result(result);
        }
    }

    /// Handle going back one state.
    fn go_back(&mut self) {
        match &mut self.state {
            AppState::SelectingContext(_) => {}
            AppState::SelectingService(_) => {
                self.go_to_context_selection();
            }
            AppState::ActiveService(service) => {
                service.destroy();
                if let Some(ctx) = self.active_context.clone() {
                    self.go_to_service_selection(ctx);
                } else {
                    self.go_to_context_selection();
                }
            }
        }
    }

    fn handle_event(&mut self, event: &Event) -> color_eyre::Result<()> {
        // Popup intercepts all key events when visible
        if let Some(ref mut popup) = self.popup {
            if let Event::Key(key) = event {
                match popup {
                    ActivePopup::Help(help) => {
                        if let KeyResult::Event(HelpEvent::Close) = help.handle_key(*key) {
                            self.msg_tx.send(AppMessage::ClosePopup)?;
                        }
                    }
                    ActivePopup::ThemeSelector(selector) => {
                        match selector.handle_key(*key) {
                            KeyResult::Event(ThemeEvent::Selected(theme)) => {
                                self.msg_tx.send(AppMessage::SelectTheme(theme))?;
                            }
                            KeyResult::Event(ThemeEvent::Cancelled) => {
                                self.msg_tx.send(AppMessage::ClosePopup)?;
                            }
                            _ => {}
                        }
                    }
                }
                return Ok(());
            }
        }

        // Handle tick separately - always goes to service and command tracker
        if matches!(event, Event::Tick) {
            self.command_tracker.on_tick();
            if let AppState::ActiveService(service) = &mut self.state {
                service.handle_tick();
            }
            return Ok(());
        }

        // Route input event based on current state
        let handled = match &mut self.state {
            AppState::SelectingContext(selector) => {
                if let Event::Key(key) = event {
                    let result = selector.handle_key(*key);
                    if let KeyResult::Event(context) = result {
                        self.msg_tx.send(AppMessage::SelectContext(context))?;
                        return Ok(());
                    }
                    result.is_consumed()
                } else {
                    false
                }
            }
            AppState::SelectingService(selector) => {
                if let Event::Key(key) = event {
                    let result = selector.handle_key(*key);
                    if let KeyResult::Event(service_id) = result {
                        self.msg_tx.send(AppMessage::SelectService(service_id))?;
                        return Ok(());
                    }
                    result.is_consumed()
                } else {
                    false
                }
            }
            AppState::ActiveService(service) => {
                let consumed = service.handle_input(event);
                if consumed {
                    // Input was consumed, process any queued messages
                    let result = service.update();
                    self.process_update_result(result);
                }
                consumed
            }
        };

        // Handle global events if not consumed
        if !handled {
            match event {
                Event::Quit => self.msg_tx.send(AppMessage::Quit)?,
                Event::Render => self.msg_tx.send(AppMessage::Render)?,
                Event::Resize(width, height) => {
                    self.msg_tx.send(AppMessage::Resize(*width, *height))?;
                }
                Event::Key(key) => {
                    match key.code {
                        KeyCode::Char('q') => self.msg_tx.send(AppMessage::Quit)?,
                        KeyCode::Char('?') => self.msg_tx.send(AppMessage::DisplayHelp)?,
                        KeyCode::Char('t') => self.msg_tx.send(AppMessage::DisplayThemeSelector)?,
                        KeyCode::Char('c') => self.msg_tx.send(AppMessage::ToggleCommandStatus)?,
                        KeyCode::Esc => self.msg_tx.send(AppMessage::GoBack)?,
                        _ => {}
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }

    fn handle_message(&mut self, tui: &mut Tui, msg: AppMessage) -> color_eyre::Result<()> {
        if !matches!(msg, AppMessage::Tick | AppMessage::Render | AppMessage::CommandCompleted { .. }) {
            debug!("Handling message: {:?}", msg);
        }

        match msg {
            AppMessage::Tick => {
                // Handled in handle_event
            }
            AppMessage::Quit => self.should_quit = true,
            AppMessage::Suspend => self.should_suspend = true,
            AppMessage::Resume => self.should_suspend = false,
            AppMessage::ClearScreen => tui.clear()?,
            AppMessage::Resize(width, height) => {
                tui.resize(Rect::new(0, 0, width, height))?;
                self.render(tui)?;
            }
            AppMessage::Render => self.render(tui)?,
            AppMessage::DisplayError(err) => {
                self.status_bar.set_error(err.clone());
                log::error!("Error: {}", err);
            }
            AppMessage::DisplayHelp => {
                self.popup = Some(ActivePopup::Help(HelpView::new(GLOBAL_KEYBINDINGS)));
            }
            AppMessage::DisplayThemeSelector => {
                self.popup = Some(ActivePopup::ThemeSelector(ThemeSelectorView::new()));
            }
            AppMessage::ClosePopup => {
                self.popup = None;
            }
            AppMessage::SelectTheme(theme) => {
                self.theme = theme;
                self.popup = None;
            }
            AppMessage::CommandCompleted { id, success } => {
                // Mark command as complete in tracker
                self.command_tracker.complete(id, success);
                // A command finished, tell service to process its messages
                if let AppState::ActiveService(service) = &mut self.state {
                    let result = service.update();
                    self.process_update_result(result);
                }
                // Render after command completion
                self.render(tui)?;
            }
            AppMessage::ToggleCommandStatus => {
                self.command_tracker.toggle_expanded();
            }
            AppMessage::SelectContext(context) => {
                self.go_to_service_selection(context);
            }
            AppMessage::SelectService(service_id) => {
                if let Some(ctx) = &self.active_context {
                    if let Some(provider) = self.registry.get(&service_id) {
                        let service = provider.create_service(ctx);
                        self.go_to_active_service(service);
                    }
                }
            }
            AppMessage::GoBack => {
                self.go_back();
            }
        }

        Ok(())
    }

    fn render(&mut self, tui: &mut Tui) -> color_eyre::Result<()> {
        tui.draw(|frame| {
            // Fill background with theme base color
            frame.render_widget(
                Block::default().style(Style::default().bg(self.theme.base())),
                frame.area(),
            );

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3), // Status bar
                    Constraint::Min(0),    // Main content
                    Constraint::Length(1), // Breadcrumbs
                ])
                .split(frame.area());

            // Render status bar
            self.status_bar.render(frame, chunks[0], &self.theme);

            // Render current state
            match &mut self.state {
                AppState::SelectingContext(selector) => {
                    selector.render(frame, chunks[1], &self.theme);
                }
                AppState::SelectingService(selector) => {
                    selector.render(frame, chunks[1], &self.theme);
                }
                AppState::ActiveService(service) => {
                    service.view(frame, chunks[1], &self.theme);
                }
            }

            // Render breadcrumbs
            let breadcrumbs = self.build_breadcrumbs();
            let bc_text = breadcrumbs.join(" > ");
            let bc_widget = Paragraph::new(bc_text).style(
                Style::default()
                    .fg(self.theme.overlay1())
                    .add_modifier(Modifier::ITALIC),
            );
            frame.render_widget(bc_widget, chunks[2]);

            // Render command status (bottom right)
            self.command_tracker.render(frame, chunks[1], &self.theme);

            // Render popup overlay on top
            if let Some(ref mut popup) = self.popup {
                match popup {
                    ActivePopup::Help(help) => {
                        help.render(frame, frame.area(), &self.theme);
                    }
                    ActivePopup::ThemeSelector(selector) => {
                        selector.render(frame, frame.area(), &self.theme);
                    }
                }
            }
        })?;
        Ok(())
    }

    fn build_breadcrumbs(&self) -> Vec<String> {
        match &self.state {
            AppState::SelectingContext(_) => {
                vec!["Select Context".to_string()]
            }
            AppState::SelectingService(_) => {
                let mut bc = vec![];
                if let Some(ctx) = &self.active_context {
                    bc.push(ctx.provider().display_name().to_string());
                }
                bc.push("Select Service".to_string());
                bc
            }
            AppState::ActiveService(service) => {
                let mut bc = vec![];
                if let Some(ctx) = &self.active_context {
                    bc.push(ctx.provider().display_name().to_string());
                }
                bc.extend(service.breadcrumbs());
                bc
            }
        }
    }
}
