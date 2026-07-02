mod events;
mod messages;
mod navigation;
mod render;

use std::sync::Arc;

use color_eyre::Result;
use color_eyre::eyre::eyre;
use tokio::sync::mpsc;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::Theme;
use crate::cli::Args;
use crate::commands::{Command, CommandCtx};
use crate::config::AppConfig;
use crate::context::{
    AuthMethod,
    AuthMethodEditor,
    CloudContext,
    ContextManager,
    ContextSelectorView,
    ContextSyncPopup,
    CredentialError,
    SyncDecision,
};
use crate::logging::LogBuffer;
use crate::registry::ServiceRegistry;
use crate::service::{Service, ServiceMsg, ServiceSelectorView};
use crate::theme::ThemeSelectorView;
use crate::tui::Tui;
use crate::ui::{
    CommandId,
    CommandPanel,
    ErrorDialog,
    HelpOverlay,
    KeybindingSection,
    LogView,
    Screen,
    StatusBar,
    ToastManager,
    ToastType,
};

/// App-backed implementation of [`CommandCtx`].
///
/// Adapts the narrow capability port that commands see onto the App's internal
/// message channel, so commands stay decoupled from `AppMessage`.
struct AppCommandCtx {
    msg_tx: UnboundedSender<AppMessage>,
}

impl CommandCtx for AppCommandCtx {
    fn toast(&self, message: String, toast_type: ToastType) {
        let _ = self.msg_tx.send(AppMessage::ShowToast {
            message,
            toast_type,
        });
    }
}

#[derive(Debug, Clone)]
pub enum AppMessage {
    // Matched by the message handler but not yet wired to a producer; kept as
    // scaffolding for the tick/suspend lifecycle.
    #[allow(dead_code)]
    Tick,
    Render,
    Resize(u16, u16),
    #[allow(dead_code)]
    Suspend,
    Resume,
    Quit,
    ClearScreen,

    DisplayError(String),
    DisplayExpectedError(String),
    DisplayHelp,
    DisplayThemeSelector,
    ToggleLogs,
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
    SelectService(crate::registry::ServiceId),
    SelectTheme(crate::theme::ThemeInfo),
    GoBack,

    RefreshContexts,
    ApplyContextSync(Vec<SyncDecision>),
    EditContextAuth(CloudContext),
    SetContextAuth {
        name: String,
        auth: AuthMethod,
    },
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
    ContextSync(ContextSyncPopup),
    AuthEditor(AuthMethodEditor),
    Logs(LogView),
}

pub struct App {
    context_manager: ContextManager,
    state: AppState,
    theme: Theme,
    popup: Option<ActivePopup>,
    status_bar: StatusBar,
    command_tracker: CommandPanel,
    toast_manager: ToastManager,
    should_quit: bool,
    should_suspend: bool,
    active_context: Option<CloudContext>,
    registry: Arc<ServiceRegistry>,
    msg_tx: UnboundedSender<AppMessage>,
    msg_rx: UnboundedReceiver<AppMessage>,
    config: Arc<AppConfig>,
    pending_service: Option<String>,
    pending_editor: Option<String>,
    log_buffer: LogBuffer,
}

impl App {
    pub fn new(
        registry: ServiceRegistry,
        config: Arc<AppConfig>,
        theme: Theme,
        log_buffer: LogBuffer,
    ) -> Self {
        let (msg_tx, msg_rx) = mpsc::unbounded_channel();
        let context_manager = ContextManager::new();
        let contexts = context_manager.get_all();

        Self {
            context_manager,
            state: AppState::SelectingContext(ContextSelectorView::new(contexts)),
            theme,
            popup: None,
            status_bar: StatusBar::new(),
            command_tracker: CommandPanel::new(),
            toast_manager: ToastManager::new(),
            should_quit: false,
            should_suspend: false,
            active_context: None,
            registry: Arc::new(registry),
            msg_tx,
            msg_rx,
            config,
            pending_service: None,
            pending_editor: None,
            log_buffer,
        }
    }

    /// Initialize app state based on CLI args.
    /// Handles the following cases:
    /// - Both context and service provided: go directly to service
    /// - Only context provided: go to service selection
    /// - Only service provided: use last context if compatible, else show filtered context selector
    /// - Neither provided: normal flow (context selection)
    ///
    pub fn apply_cli_args(&mut self, args: &Args) -> Result<()> {
        match (&args.context, &args.service) {
            (Some(ctx_name), Some(svc_name)) => {
                let context = self.context_manager.find_by_name(ctx_name)?;
                let service_id = self.registry.find_service_by_name(&context, svc_name)?;
                self.start_service(&context, &service_id);
            }

            (Some(ctx_name), None) => {
                let context = self.context_manager.find_by_name(ctx_name)?;
                self.go_to_service_selection(&context);
            }

            (None, Some(svc_name)) => {
                let provider = self.registry.find_provider_by_name(svc_name)?;

                // Try last context if compatible
                if let Some(ctx_name) = &self.config.last_context
                    && let Ok(context) = self.context_manager.find_by_name(ctx_name)
                    && context.provider() == provider
                {
                    let service_id = self.registry.find_service_by_name(&context, svc_name)?;
                    self.start_service(&context, &service_id);
                    return Ok(());
                }

                // Last context incompatible or missing: show filtered context selector
                let filtered = self.context_manager.get_by_provider(provider);

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

            if let Some(content) = self.pending_editor.take() {
                tui.exit()?;
                let result = edit::edit(&content);
                tui.enter()?;
                tui.clear()?;

                if let AppState::ActiveService(service) = &mut self.state {
                    let edited = match result {
                        Ok(new) if new != content => Some(new),
                        Ok(_) => None,
                        Err(e) => {
                            let _ = self.msg_tx.send(AppMessage::DisplayError(format!(
                                "Failed to open editor: {e}"
                            )));
                            None
                        }
                    };
                    service.handle_editor_result(edited);
                    let svc_result = service.update();
                    self.process_update_result(svc_result);
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
    ///
    /// Each command is bracketed with start/finish logging (visible live in the
    /// log overlay) and an optional timeout, so a hung network/auth call turns
    /// into a surfaced error instead of an indefinite loading spinner.
    // `map_or_else` can't express the timeout branches: both arms `await` the
    // (moved) execute future, which sync closures cannot do.
    #[allow(clippy::option_if_let_else)]
    fn spawn_commands(&mut self, commands: Vec<Box<dyn Command>>) {
        for cmd in commands {
            let name = cmd.name();
            let timeout = cmd.timeout();
            let id = self.command_tracker.start(name.clone());
            let msg_tx = self.msg_tx.clone();
            let ctx: Arc<dyn CommandCtx> = Arc::new(AppCommandCtx {
                msg_tx: msg_tx.clone(),
            });
            tokio::spawn(async move {
                let started = std::time::Instant::now();
                tracing::info!(command = %name, ?timeout, "Command started");

                let result = match timeout {
                    Some(limit) => match tokio::time::timeout(limit, cmd.execute(ctx)).await {
                        Ok(res) => res,
                        Err(_) => Err(color_eyre::eyre::eyre!(
                            "'{name}' timed out after {}s — the operation may be blocked on \
                             authentication or network connectivity",
                            limit.as_secs()
                        )),
                    },
                    None => cmd.execute(ctx).await,
                };

                let elapsed_ms = started.elapsed().as_millis();
                let success = match result {
                    Ok(()) => {
                        tracing::info!(command = %name, elapsed_ms, "Command completed");
                        true
                    }
                    Err(e) => {
                        tracing::error!(command = %name, elapsed_ms, error = %e, "Command failed");
                        // Credential problems are expected and user-actionable, not
                        // bugs — surface them without the "report to developers" framing.
                        let expected = e
                            .chain()
                            .any(|src| src.downcast_ref::<CredentialError>().is_some());
                        let msg = e.to_string();
                        let _ = msg_tx.send(if expected {
                            AppMessage::DisplayExpectedError(msg)
                        } else {
                            AppMessage::DisplayError(msg)
                        });
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
            Ok(ServiceMsg::EditExternal { content }) => {
                self.pending_editor = Some(content);
            }
            Err(err) => {
                let _ = self.msg_tx.send(AppMessage::DisplayError(err.to_string()));
            }
        }
    }

    fn open_help_overlay(&mut self) {
        let (local, local_title) = match &self.state {
            AppState::ActiveService(service) => (
                service.keybindings(),
                service
                    .breadcrumbs()
                    .last()
                    .cloned()
                    .unwrap_or_else(|| "Current View".to_string()),
            ),
            AppState::SelectingContext(selector) => {
                (selector.keybindings(), "Contexts".to_string())
            }
            AppState::SelectingService(_) => (vec![], "Navigation".to_string()),
        };
        self.popup = Some(ActivePopup::Help(HelpOverlay::with_sections(vec![
            KeybindingSection::new(&local_title, local),
            KeybindingSection::new("Global", self.status_bar.global_keybindings()),
        ])));
    }

    /// Toggle the live log viewer. Closes it if already open, otherwise opens
    /// it on top of the current view.
    fn toggle_logs(&mut self) {
        if matches!(self.popup, Some(ActivePopup::Logs(_))) {
            self.popup = None;
        } else {
            self.popup = Some(ActivePopup::Logs(LogView::new(self.log_buffer.clone())));
        }
    }
}
