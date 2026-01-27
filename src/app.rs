use std::sync::Arc;

use color_eyre::eyre::eyre;
use log::debug;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Paragraph};
use tokio::sync::mpsc;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::Theme;
use crate::cli::Args;
use crate::commands::{Command, CommandEnv};
use crate::config::{AppConfig, GlobalAction, KeyResolver, save_last_context, save_theme};
use crate::context;
use crate::context::{CloudContext, ContextSelectorView, load_contexts};
use crate::registry::{ServiceId, ServiceRegistry};
use crate::service::ServiceSelectorView;
use crate::service::{Service, ServiceMsg};
use crate::theme::ThemeInfo;
use crate::theme::{ThemeEvent, ThemeSelectorView};
use crate::tui::{Event, Tui};
use crate::ui::{
    CommandId, CommandPanel, ErrorDialog, ErrorDialogEvent, HelpEvent, HelpOverlay,
    KeybindingSection, Screen, StatusBar, Toast, ToastManager, ToastType,
};
use crate::ui::{Component, EventResult};
use color_eyre::Result;

#[derive(Debug, Clone)]
pub enum AppMessage {
    Tick,
    Render,
    Resize(u16, u16),
    Suspend,
    Resume,
    Quit,
    ClearScreen,

    DisplayError(String),
    DisplayHelp,
    DisplayThemeSelector,
    ClosePopup,

    CommandCompleted {
        id: CommandId,
        success: bool,
    },
    ToggleCommandStatus,
    ShowToast {
        message: String,
        toast_type: ToastType,
    },

    SelectContext(CloudContext),
    SelectService(ServiceId),
    SelectTheme(ThemeInfo),
    GoBack,
}

/// Application state - what the user is currently doing.
enum AppState {
    /// Selecting a cloud context (GCP project, AWS account, etc.)
    SelectingContext(ContextSelectorView),
    /// Selecting a service within the chosen context
    SelectingService(ServiceSelectorView),
    /// Using an active cloud service
    ActiveService(Box<dyn Service>),
}

enum ActivePopup {
    Help(HelpOverlay),
    ThemeSelector(ThemeSelectorView),
    Error(ErrorDialog),
}

pub struct App {
    state: AppState,
    theme: Theme,
    popup: Option<ActivePopup>,
    status_bar: StatusBar,
    command_tracker: CommandPanel,
    toast_manager: ToastManager,
    cmd_env: CommandEnv,
    should_quit: bool,
    should_suspend: bool,
    active_context: Option<CloudContext>,
    registry: Arc<ServiceRegistry>,
    msg_tx: UnboundedSender<AppMessage>,
    msg_rx: UnboundedReceiver<AppMessage>,
    config: Arc<AppConfig>,
    resolver: Arc<KeyResolver>,
    pending_service: Option<String>,
}

impl App {
    pub fn new(
        registry: ServiceRegistry,
        config: Arc<AppConfig>,
        resolver: Arc<KeyResolver>,
        theme: Theme,
    ) -> Result<Self> {
        let (msg_tx, msg_rx) = mpsc::unbounded_channel();

        let cmd_env = CommandEnv::new(msg_tx.clone());

        Ok(Self {
            state: AppState::SelectingContext(ContextSelectorView::new(resolver.clone())?),
            theme,
            popup: None,
            status_bar: StatusBar::new(resolver.clone()),
            command_tracker: CommandPanel::new(),
            toast_manager: ToastManager::new(),
            cmd_env,
            should_quit: false,
            should_suspend: false,
            active_context: None,
            registry: Arc::new(registry),
            msg_tx,
            msg_rx,
            config,
            resolver,
            pending_service: None,
        })
    }

    /// Initialize app state based on CLI args.
    /// Handles the following cases:
    /// - Both context and service provided: go directly to service
    /// - Only context provided: go to service selection
    /// - Only service provided: use last context if compatible, else show filtered context selector
    /// - Neither provided: normal flow (context selection)
    ///
    pub fn apply_cli_args(&mut self, args: &Args) -> Result<()> {
        let contexts = load_contexts();

        match (&args.context, &args.service) {
            (Some(ctx_name), Some(svc_name)) => {
                let context = context::find_by_name(&contexts, ctx_name)?;
                let service_id = self.registry.find_service_by_name(&context, svc_name)?;
                self.start_service(&context, &service_id);
            }

            (Some(ctx_name), None) => {
                let context = context::find_by_name(&contexts, ctx_name)?;
                self.go_to_service_selection(&context);
            }

            (None, Some(svc_name)) => {
                let provider = self.registry.find_provider_by_name(svc_name)?;

                // Try last context if compatible
                if let Some(ctx_name) = &self.config.last_context
                    && let Ok(context) = context::find_by_name(&contexts, ctx_name)
                    && context.provider() == provider
                {
                    let service_id = self.registry.find_service_by_name(&context, svc_name)?;
                    self.start_service(&context, &service_id);
                    return Ok(());
                }

                // Last context incompatible or missing: show filtered context selector
                let filtered: Vec<_> = contexts
                    .into_iter()
                    .filter(|c| c.provider() == provider)
                    .collect();

                if filtered.is_empty() {
                    return Err(eyre!("No {} contexts found", provider.display_name()));
                }

                self.pending_service = Some(svc_name.clone());
                self.go_to_filtered_context_selection(filtered);
            }

            (None, None) => {}
        }
        Ok(())
    }

    fn start_service(&mut self, context: &CloudContext, service_id: &ServiceId) {
        self.active_context = Some(context.clone());
        self.status_bar.set_active_context(context.clone());
        if let Some(provider) = self.registry.get(service_id) {
            let service = provider.create_service(context, self.resolver.clone());
            self.go_to_active_service(service);
        }
    }

    fn go_to_filtered_context_selection(&mut self, contexts: Vec<CloudContext>) {
        self.state = AppState::SelectingContext(ContextSelectorView::with_contexts(
            contexts,
            self.resolver.clone(),
        ));
    }

    // App is single-threaded; making dyn Service Send would cascade through the entire trait hierarchy
    #[allow(clippy::future_not_send)]
    pub async fn run(&mut self) -> Result<()> {
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
            let cmd_env = self.cmd_env.clone();
            tokio::spawn(async move {
                let success = match cmd.execute(cmd_env).await {
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

    fn process_update_result(&mut self, result: Result<ServiceMsg>) {
        match result {
            Ok(ServiceMsg::Idle) => {}
            Ok(ServiceMsg::Run(commands)) => {
                self.spawn_commands(commands);
            }
            Ok(ServiceMsg::Close) => {
                let _ = self.msg_tx.send(AppMessage::GoBack);
            }
            Err(err) => {
                let _ = self.msg_tx.send(AppMessage::DisplayError(err.to_string()));
            }
        }
    }

    /// Transition to context selection.
    fn go_to_context_selection(&mut self) {
        self.active_context = None;
        self.status_bar.clear_context();
        self.state = AppState::SelectingContext(ContextSelectorView::new(self.resolver.clone()).unwrap());
    }

    /// Transition to service selection.
    fn go_to_service_selection(&mut self, context: &CloudContext) {
        self.active_context = Some(context.clone());
        self.status_bar.set_active_context(context.clone());
        self.state = AppState::SelectingService(ServiceSelectorView::new(
            &self.registry,
            context,
            self.resolver.clone(),
        ));
    }

    /// Transition to active service.
    fn go_to_active_service(&mut self, mut service: Box<dyn Service>) {
        // Save last context for -s flag
        if let Some(ctx) = &self.active_context {
            let _ = save_last_context(ctx.name());
        }

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
                if let Some(ref ctx) = self.active_context.clone() {
                    self.go_to_service_selection(ctx);
                } else {
                    self.go_to_context_selection();
                }
            }
        }
    }

    fn open_help_overlay(&mut self) {
        let local = match &self.state {
            AppState::ActiveService(service) => service.keybindings(),
            _ => vec![],
        };
        let local_title = match &self.state {
            AppState::ActiveService(service) => service
                .breadcrumbs()
                .last()
                .cloned()
                .unwrap_or_else(|| "Current View".to_string()),
            _ => "Navigation".to_string(),
        };
        self.popup = Some(ActivePopup::Help(HelpOverlay::with_sections(vec![
            KeybindingSection::new(&local_title, local),
            KeybindingSection::new("Global", self.status_bar.global_keybindings()),
        ])));
    }

    fn handle_popup_event(&mut self, key: crossterm::event::KeyEvent) -> Result<()> {
        let Some(ref mut popup) = self.popup else {
            return Ok(());
        };
        match popup {
            ActivePopup::Help(help) => {
                if matches!(
                    help.handle_key(key),
                    Ok(EventResult::Event(HelpEvent::Close))
                ) {
                    self.msg_tx.send(AppMessage::ClosePopup)?;
                }
            }
            ActivePopup::ThemeSelector(selector) => match selector.handle_key(key) {
                Ok(EventResult::Event(ThemeEvent::Selected(theme_info))) => {
                    self.msg_tx.send(AppMessage::SelectTheme(theme_info))?;
                }
                Ok(EventResult::Event(ThemeEvent::Cancelled)) => {
                    self.msg_tx.send(AppMessage::ClosePopup)?;
                }
                _ => {}
            },
            ActivePopup::Error(dialog) => {
                if matches!(
                    dialog.handle_key(key),
                    Ok(EventResult::Event(ErrorDialogEvent::Dismissed))
                ) {
                    self.msg_tx.send(AppMessage::ClosePopup)?;
                }
            }
        }
        Ok(())
    }

    fn handle_global_event(&self, event: &Event) -> Result<()> {
        match event {
            Event::Quit => self.msg_tx.send(AppMessage::Quit)?,
            Event::Render => self.msg_tx.send(AppMessage::Render)?,
            Event::Resize(width, height) => {
                self.msg_tx.send(AppMessage::Resize(*width, *height))?;
            }
            Event::Key(key) => {
                if self.resolver.matches_global(key, GlobalAction::Quit) {
                    self.msg_tx.send(AppMessage::Quit)?;
                } else if self.resolver.matches_global(key, GlobalAction::Help) {
                    self.msg_tx.send(AppMessage::DisplayHelp)?;
                } else if self.resolver.matches_global(key, GlobalAction::Theme) {
                    self.msg_tx.send(AppMessage::DisplayThemeSelector)?;
                } else if self
                    .resolver
                    .matches_global(key, GlobalAction::CommandsToggle)
                {
                    self.msg_tx.send(AppMessage::ToggleCommandStatus)?;
                } else if self.resolver.matches_global(key, GlobalAction::Back) {
                    self.msg_tx.send(AppMessage::GoBack)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_event(&mut self, event: &Event) -> Result<()> {
        // Popup intercepts all key events when visible
        if self.popup.is_some() {
            if let Event::Key(key) = event {
                self.handle_popup_event(*key)?;
            }
            return Ok(());
        }

        // Handle tick separately - always goes to service, commands tracker, and toast manager
        if matches!(event, Event::Tick) {
            self.command_tracker.handle_tick();
            self.toast_manager.handle_tick();
            if let AppState::ActiveService(service) = &mut self.state {
                service.handle_tick();
            }
            return Ok(());
        }

        // Route input event based on current state
        let handled = match &mut self.state {
            AppState::SelectingContext(selector) => {
                if let Event::Key(key) = event {
                    match selector.handle_key(*key) {
                        Ok(EventResult::Event(context)) => {
                            self.msg_tx.send(AppMessage::SelectContext(context))?;
                            return Ok(());
                        }
                        Ok(EventResult::Consumed) => true,
                        Ok(EventResult::Ignored) | Err(_) => false,
                    }
                } else {
                    false
                }
            }
            AppState::SelectingService(selector) => {
                if let Event::Key(key) = event {
                    match selector.handle_key(*key) {
                        Ok(EventResult::Event(service_id)) => {
                            self.msg_tx.send(AppMessage::SelectService(service_id))?;
                            return Ok(());
                        }
                        Ok(EventResult::Consumed) => true,
                        Ok(EventResult::Ignored) | Err(_) => false,
                    }
                } else {
                    false
                }
            }
            AppState::ActiveService(service) => {
                if let Event::Key(key) = event {
                    let result = service.handle_key(*key);
                    if result.is_consumed() {
                        let msg = service.update();
                        self.process_update_result(msg);
                    }
                    result.is_consumed()
                } else {
                    false
                }
            }
        };

        if !handled {
            self.handle_global_event(event)?;
        }

        Ok(())
    }

    fn handle_message(&mut self, tui: &mut Tui, msg: AppMessage) -> Result<()> {
        if !matches!(
            msg,
            AppMessage::Tick | AppMessage::Render | AppMessage::CommandCompleted { .. }
        ) {
            debug!("Handling message: {msg:?}");
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
                log::error!("Error: {err}");
                self.popup = Some(ActivePopup::Error(ErrorDialog::new(
                    err,
                    self.resolver.clone(),
                )));
            }
            AppMessage::DisplayHelp => self.open_help_overlay(),
            AppMessage::DisplayThemeSelector => {
                self.popup = Some(ActivePopup::ThemeSelector(ThemeSelectorView::new(
                    self.resolver.clone(),
                )));
            }
            AppMessage::ClosePopup => {
                self.popup = None;
            }
            AppMessage::SelectTheme(theme_info) => {
                // Persist theme to config file
                if let Err(e) = save_theme(theme_info.name) {
                    log::warn!("Failed to persist theme: {e}");
                }
                self.theme = theme_info.theme;
                self.popup = None;
            }
            AppMessage::CommandCompleted { id, success } => {
                // Mark commands as complete in tracker
                self.command_tracker.complete(id, success);
                // A command finished, tell service to process its messages
                if let AppState::ActiveService(service) = &mut self.state {
                    let result = service.update();
                    self.process_update_result(result);
                }
                // Render after commands completion
                self.render(tui)?;
            }
            AppMessage::ToggleCommandStatus => {
                self.command_tracker.toggle_expanded();
            }
            AppMessage::ShowToast {
                message,
                toast_type,
            } => {
                let toast = match toast_type {
                    ToastType::Success => Toast::success(message),
                    ToastType::Info => Toast::info(message),
                };
                self.toast_manager.show(toast);
            }
            AppMessage::SelectContext(context) => {
                // Check for pending service from CLI args
                if let Some(svc_name) = self.pending_service.take()
                    && let Ok(service_id) = self.registry.find_service_by_name(&context, &svc_name)
                {
                    self.start_service(&context, &service_id);
                    return Ok(());
                }

                self.go_to_service_selection(&context);
            }
            AppMessage::SelectService(service_id) => {
                if let Some(ctx) = &self.active_context
                    && let Some(provider) = self.registry.get(&service_id)
                {
                    let service = provider.create_service(ctx, self.resolver.clone());
                    self.go_to_active_service(service);
                }
            }
            AppMessage::GoBack => {
                self.go_back();
            }
        }

        Ok(())
    }

    fn render(&mut self, tui: &mut Tui) -> Result<()> {
        tui.draw(|frame| {
            // Fill background with theme base color
            frame.render_widget(
                Block::default().style(Style::default().bg(self.theme.base())),
                frame.area(),
            );

            // Get keybindings for status bar
            let local_keybindings = match &self.state {
                AppState::ActiveService(service) => service.keybindings(),
                _ => vec![],
            };

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(9), // Status bar (logo + keybindings + context)
                    Constraint::Min(0),    // Main content
                    Constraint::Length(1), // Breadcrumbs
                ])
                .split(frame.area());

            // Render status bar with keybinding hints
            self.status_bar.render_with_keybindings(
                frame,
                chunks[0],
                &self.theme,
                &local_keybindings,
            );

            // Render current state
            match &mut self.state {
                AppState::SelectingContext(selector) => {
                    selector.render(frame, chunks[1], &self.theme);
                }
                AppState::SelectingService(selector) => {
                    selector.render(frame, chunks[1], &self.theme);
                }
                AppState::ActiveService(service) => {
                    service.render(frame, chunks[1], &self.theme);
                }
            }

            // Render breadcrumbs (left) and inline commands status (right)
            let breadcrumbs = self.build_breadcrumbs();
            let bc_text = breadcrumbs.join(" > ");

            // First render inline commands status to get its width
            let cmd_width = self
                .command_tracker
                .render_inline(frame, chunks[2], &self.theme);

            // Render breadcrumbs in remaining space
            let bc_area = Rect::new(
                chunks[2].x,
                chunks[2].y,
                chunks[2].width.saturating_sub(cmd_width + 2),
                chunks[2].height,
            );
            let bc_widget = Paragraph::new(bc_text).style(
                Style::default()
                    .fg(self.theme.overlay1())
                    .add_modifier(Modifier::ITALIC),
            );
            frame.render_widget(bc_widget, bc_area);

            // Render expanded commands panel (overlay on main content)
            self.command_tracker.render(frame, chunks[1], &self.theme);

            // Render toasts (bottom right of main content)
            self.toast_manager.render(frame, chunks[1], &self.theme);

            // Render popup overlay on top
            if let Some(ref mut popup) = self.popup {
                match popup {
                    ActivePopup::Help(help) => {
                        help.render(frame, frame.area(), &self.theme);
                    }
                    ActivePopup::ThemeSelector(selector) => {
                        selector.render(frame, frame.area(), &self.theme);
                    }
                    ActivePopup::Error(dialog) => {
                        dialog.render(frame, frame.area(), &self.theme);
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
