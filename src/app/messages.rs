use color_eyre::Result;
use ratatui::layout::Rect;
use tracing::{debug, error, warn};

use super::{ActivePopup, App, AppMessage, AppState};
use crate::config::save_theme;
use crate::context::{ContextMergePopup, ContextSelectorView};
use crate::theme::ThemeSelectorView;
use crate::tui::Tui;
use crate::ui::{ErrorDialog, Toast};

impl App {
    pub(super) fn handle_message(&mut self, tui: &mut Tui, msg: AppMessage) -> Result<()> {
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
                error!("Error: {err}");
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
                    warn!("Failed to persist theme: {e}");
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
                    crate::ui::ToastType::Success => Toast::success(message),
                    crate::ui::ToastType::Info => Toast::info(message),
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
            AppMessage::RefreshContexts => {
                let new_contexts = self.context_manager.discover_new();
                if new_contexts.is_empty() {
                    self.toast_manager
                        .show(Toast::info("No new contexts found"));
                } else {
                    self.popup = Some(ActivePopup::ContextMerge(ContextMergePopup::new(
                        new_contexts,
                        self.resolver.clone(),
                    )));
                }
            }
            AppMessage::ImportContexts(new_contexts) => {
                let count = new_contexts.len();
                if let Err(e) = self.context_manager.add_contexts(new_contexts) {
                    error!("Failed to import contexts: {e}");
                    self.toast_manager
                        .show(Toast::info("Failed to import contexts"));
                } else {
                    let contexts = self.context_manager.get_all();
                    self.state = AppState::SelectingContext(ContextSelectorView::new(
                        contexts,
                        self.resolver.clone(),
                    ));
                    self.toast_manager.show(Toast::success(format!(
                        "Imported {} context{}",
                        count,
                        if count == 1 { "" } else { "s" }
                    )));
                }
                self.popup = None;
            }
        }

        Ok(())
    }
}
