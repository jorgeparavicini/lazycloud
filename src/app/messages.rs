use color_eyre::Result;
use ratatui::layout::Rect;
use tracing::{debug, error, warn};

use super::{ActivePopup, App, AppMessage, AppState};
use crate::config::save_theme;
use crate::context::{AuthMethodEditor, ContextSelectorView, ContextSyncPopup};
use crate::theme::ThemeSelectorView;
use crate::tui::Tui;
use crate::ui::{ErrorDialog, Toast};

impl App {
    #[allow(clippy::too_many_lines)]
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
                self.popup = Some(ActivePopup::Error(ErrorDialog::new(err)));
            }
            AppMessage::DisplayExpectedError(err) => {
                warn!("Actionable error: {err}");
                self.popup = Some(ActivePopup::Error(ErrorDialog::new(err).expected()));
            }
            AppMessage::DisplayHelp => self.open_help_overlay(),
            AppMessage::DisplayThemeSelector => {
                self.popup = Some(ActivePopup::ThemeSelector(ThemeSelectorView::new()));
            }
            AppMessage::ToggleLogs => self.toggle_logs(),
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
                    let service = provider.create_service(ctx);
                    self.go_to_active_service(service);
                }
            }
            AppMessage::GoBack => {
                self.go_back();
            }
            AppMessage::RefreshContexts => {
                let diff = self.context_manager.diff_gcloud();
                if diff.is_empty() {
                    self.toast_manager
                        .show(Toast::info("Contexts already in sync with gcloud"));
                } else {
                    self.popup = Some(ActivePopup::ContextSync(ContextSyncPopup::new(diff)));
                }
            }
            AppMessage::ApplyContextSync(decisions) => {
                match self.context_manager.apply_sync(decisions) {
                    Ok(summary) if summary.is_empty() => {
                        self.toast_manager.show(Toast::info("No changes applied"));
                    }
                    Ok(summary) => {
                        let contexts = self.context_manager.get_all();
                        self.state =
                            AppState::SelectingContext(ContextSelectorView::new(contexts));
                        self.toast_manager.show(Toast::success(format!(
                            "Synced contexts: +{} ~{} -{}",
                            summary.added, summary.updated, summary.removed
                        )));
                    }
                    Err(e) => {
                        error!("Failed to apply context sync: {e}");
                        self.toast_manager
                            .show(Toast::info("Failed to sync contexts"));
                    }
                }
                self.popup = None;
            }
            AppMessage::EditContextAuth(context) => {
                self.popup = Some(ActivePopup::AuthEditor(AuthMethodEditor::new(&context)));
            }
            AppMessage::SetContextAuth { name, auth } => {
                if let Err(e) = self.context_manager.set_auth(&name, auth) {
                    error!("Failed to update auth method: {e}");
                    self.toast_manager
                        .show(Toast::info("Failed to update auth method"));
                } else {
                    let contexts = self.context_manager.get_all();
                    self.state =
                        AppState::SelectingContext(ContextSelectorView::new(contexts));
                    self.toast_manager
                        .show(Toast::success(format!("Updated auth for '{name}'")));
                }
                self.popup = None;
            }
        }

        Ok(())
    }
}
