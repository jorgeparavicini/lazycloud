use color_eyre::Result;
use crossterm::event::KeyCode;

use super::{ActivePopup, App, AppMessage, AppState};
use crate::context::{AuthEditorEvent, ContextSelectorEvent, ContextSyncEvent};
use crate::theme::ThemeEvent;
use crate::tui::TuiEvent;
use crate::ui::{Component, ErrorDialogEvent, EventResult, HelpEvent, LogViewEvent, Screen};

impl App {
    pub(super) fn handle_popup_event(&mut self, key: crossterm::event::KeyEvent) -> Result<()> {
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
                    self.msg_tx.send(AppMessage::GoBack)?;
                    self.msg_tx.send(AppMessage::ClosePopup)?;
                }
            }
            ActivePopup::ContextSync(sync) => match sync.handle_key(key) {
                Ok(EventResult::Event(ContextSyncEvent::Apply(decisions))) => {
                    self.msg_tx.send(AppMessage::ApplyContextSync(decisions))?;
                }
                Ok(EventResult::Event(ContextSyncEvent::Cancel)) => {
                    self.msg_tx.send(AppMessage::ClosePopup)?;
                }
                _ => {}
            },
            ActivePopup::AuthEditor(editor) => match editor.handle_key(key) {
                Ok(EventResult::Event(AuthEditorEvent::Save { context_name, auth })) => {
                    self.msg_tx.send(AppMessage::SetContextAuth {
                        name: context_name,
                        auth,
                    })?;
                }
                Ok(EventResult::Event(AuthEditorEvent::Cancel)) => {
                    self.msg_tx.send(AppMessage::ClosePopup)?;
                }
                _ => {}
            },
            ActivePopup::Logs(logs) => {
                if matches!(
                    logs.handle_key(key),
                    Ok(EventResult::Event(LogViewEvent::Close))
                ) {
                    self.msg_tx.send(AppMessage::ClosePopup)?;
                }
            }
        }
        Ok(())
    }

    pub(super) fn handle_global_event(&self, event: &TuiEvent) -> Result<()> {
        match event {
            TuiEvent::Quit => self.msg_tx.send(AppMessage::Quit)?,
            TuiEvent::Render => self.msg_tx.send(AppMessage::Render)?,
            TuiEvent::Resize(width, height) => {
                self.msg_tx.send(AppMessage::Resize(*width, *height))?;
            }
            TuiEvent::Key(key) => {
                if key.code == KeyCode::Char('q') {
                    self.msg_tx.send(AppMessage::Quit)?;
                } else if key.code == KeyCode::Char('?') {
                    self.msg_tx.send(AppMessage::DisplayHelp)?;
                } else if key.code == KeyCode::Char('t') {
                    self.msg_tx.send(AppMessage::DisplayThemeSelector)?;
                } else if key.code == KeyCode::Char('c') {
                    self.msg_tx.send(AppMessage::ToggleCommandStatus)?;
                } else if key.code == KeyCode::Char('L') {
                    self.msg_tx.send(AppMessage::ToggleLogs)?;
                } else if key.code == KeyCode::Esc {
                    self.msg_tx.send(AppMessage::GoBack)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    pub(super) fn handle_event(&mut self, event: &TuiEvent) -> Result<()> {
        // Popup intercepts all key events when visible
        if self.popup.is_some()
            && let TuiEvent::Key(key) = event
        {
            self.handle_popup_event(*key)?;
            return Ok(());
        }

        if matches!(event, TuiEvent::Tick) {
            self.command_tracker.handle_tick();
            self.toast_manager.handle_tick();
            if let AppState::ActiveService(service) = &mut self.state {
                service.handle_tick();
            }
            return Ok(());
        }

        let handled = match &mut self.state {
            AppState::SelectingContext(selector) => {
                if let TuiEvent::Key(key) = event {
                    match selector.handle_key(*key) {
                        Ok(EventResult::Event(ContextSelectorEvent::Selected(context))) => {
                            self.msg_tx.send(AppMessage::SelectContext(context))?;
                            return Ok(());
                        }
                        Ok(EventResult::Event(ContextSelectorEvent::Refresh)) => {
                            self.msg_tx.send(AppMessage::RefreshContexts)?;
                            return Ok(());
                        }
                        Ok(EventResult::Event(ContextSelectorEvent::EditAuth(context))) => {
                            self.msg_tx.send(AppMessage::EditContextAuth(context))?;
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
                if let TuiEvent::Key(key) = event {
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
                if let TuiEvent::Key(key) = event {
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
}
