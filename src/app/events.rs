use color_eyre::Result;

use super::{ActivePopup, App, AppMessage, AppState};
use crate::config::GlobalAction;
use crate::context::{ContextMergeEvent, ContextSelectorEvent};
use crate::theme::ThemeEvent;
use crate::tui::Event;
use crate::ui::{Component, ErrorDialogEvent, EventResult, HelpEvent, Screen};

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
                    self.msg_tx.send(AppMessage::ClosePopup)?;
                }
            }
            ActivePopup::ContextMerge(merge) => match merge.handle_key(key) {
                Ok(EventResult::Event(ContextMergeEvent::Apply(changes))) => {
                    self.msg_tx.send(AppMessage::ApplyContextChanges(changes))?;
                }
                Ok(EventResult::Event(ContextMergeEvent::Skip)) => {
                    self.msg_tx.send(AppMessage::ClosePopup)?;
                }
                _ => {}
            },
        }
        Ok(())
    }

    pub(super) fn handle_global_event(&self, event: &Event) -> Result<()> {
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

    pub(super) fn handle_event(&mut self, event: &Event) -> Result<()> {
        // Popup intercepts all key events when visible
        if self.popup.is_some()
            && let Event::Key(key) = event
        {
            self.handle_popup_event(*key)?;
            return Ok(());
        }

        if matches!(event, Event::Tick) {
            self.command_tracker.handle_tick();
            self.toast_manager.handle_tick();
            if let AppState::ActiveService(service) = &mut self.state {
                service.handle_tick();
            }
            return Ok(());
        }

        let handled = match &mut self.state {
            AppState::SelectingContext(selector) => {
                if let Event::Key(key) = event {
                    match selector.handle_key(*key) {
                        Ok(EventResult::Event(ContextSelectorEvent::Selected(context))) => {
                            self.msg_tx.send(AppMessage::SelectContext(context))?;
                            return Ok(());
                        }
                        Ok(EventResult::Event(ContextSelectorEvent::Refresh)) => {
                            self.msg_tx.send(AppMessage::RefreshContexts)?;
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
}
