use crate::config::actions::*;
use crate::config::keybindings::KeybindingsConfig;
use crossterm::event::KeyEvent;
use std::sync::Arc;

pub struct KeyResolver {
    pub keybindings: Arc<KeybindingsConfig>,
}

impl KeyResolver {
    pub fn new(keybindings: Arc<KeybindingsConfig>) -> Self {
        Self { keybindings }
    }

    // Global actions
    pub fn matches_global(&self, event: &KeyEvent, action: GlobalAction) -> bool {
        let kb = &self.keybindings.global;
        match action {
            GlobalAction::Quit => kb.quit.matches(event),
            GlobalAction::Help => kb.help.matches(event),
            GlobalAction::Theme => kb.theme.matches(event),
            GlobalAction::Back => kb.back.matches(event),
            GlobalAction::CommandsToggle => kb.commands_toggle.matches(event),
        }
    }

    pub fn display_global(&self, action: GlobalAction) -> String {
        let kb = &self.keybindings.global;
        match action {
            GlobalAction::Quit => kb.quit.display(),
            GlobalAction::Help => kb.help.display(),
            GlobalAction::Theme => kb.theme.display(),
            GlobalAction::Back => kb.back.display(),
            GlobalAction::CommandsToggle => kb.commands_toggle.display(),
        }
    }

    // Navigation actions
    pub fn matches_nav(&self, event: &KeyEvent, action: NavAction) -> bool {
        let kb = &self.keybindings.navigation;
        match action {
            NavAction::Up => kb.up.matches(event),
            NavAction::Down => kb.down.matches(event),
            NavAction::PageUp => kb.page_up.matches(event),
            NavAction::PageDown => kb.page_down.matches(event),
            NavAction::Home => kb.home.matches(event),
            NavAction::End => kb.end.matches(event),
            NavAction::Select => kb.select.matches(event),
        }
    }

    pub fn display_nav(&self, action: NavAction) -> String {
        let kb = &self.keybindings.navigation;
        match action {
            NavAction::Up => kb.up.display(),
            NavAction::Down => kb.down.display(),
            NavAction::PageUp => kb.page_up.display(),
            NavAction::PageDown => kb.page_down.display(),
            NavAction::Home => kb.home.display(),
            NavAction::End => kb.end.display(),
            NavAction::Select => kb.select.display(),
        }
    }

    // Search actions
    pub fn matches_search(&self, event: &KeyEvent, action: SearchAction) -> bool {
        let kb = &self.keybindings.search;
        match action {
            SearchAction::Toggle => kb.toggle.matches(event),
            SearchAction::Exit => kb.exit.matches(event),
        }
    }

    pub fn display_search(&self, action: SearchAction) -> String {
        let kb = &self.keybindings.search;
        match action {
            SearchAction::Toggle => kb.toggle.display(),
            SearchAction::Exit => kb.exit.display(),
        }
    }

    // Secrets actions
    pub fn matches_secrets(&self, event: &KeyEvent, action: SecretsAction) -> bool {
        let kb = &self.keybindings.secrets;
        match action {
            SecretsAction::ViewPayload => kb.view_payload.matches(event),
            SecretsAction::Copy => kb.copy.matches(event),
            SecretsAction::Versions => kb.versions.matches(event),
            SecretsAction::New => kb.new.matches(event),
            SecretsAction::Delete => kb.delete.matches(event),
            SecretsAction::Labels => kb.labels.matches(event),
            SecretsAction::Iam => kb.iam.matches(event),
            SecretsAction::Replication => kb.replication.matches(event),
            SecretsAction::Reload => kb.reload.matches(event),
        }
    }

    pub fn display_secrets(&self, action: SecretsAction) -> String {
        let kb = &self.keybindings.secrets;
        match action {
            SecretsAction::ViewPayload => kb.view_payload.display(),
            SecretsAction::Copy => kb.copy.display(),
            SecretsAction::Versions => kb.versions.display(),
            SecretsAction::New => kb.new.display(),
            SecretsAction::Delete => kb.delete.display(),
            SecretsAction::Labels => kb.labels.display(),
            SecretsAction::Iam => kb.iam.display(),
            SecretsAction::Replication => kb.replication.display(),
            SecretsAction::Reload => kb.reload.display(),
        }
    }

    // Versions actions
    pub fn matches_versions(&self, event: &KeyEvent, action: VersionsAction) -> bool {
        let kb = &self.keybindings.versions;
        match action {
            VersionsAction::ViewPayload => kb.view_payload.matches(event),
            VersionsAction::Add => kb.add.matches(event),
            VersionsAction::Disable => kb.disable.matches(event),
            VersionsAction::Enable => kb.enable.matches(event),
            VersionsAction::Destroy => kb.destroy.matches(event),
            VersionsAction::Reload => kb.reload.matches(event),
        }
    }

    pub fn display_versions(&self, action: VersionsAction) -> String {
        let kb = &self.keybindings.versions;
        match action {
            VersionsAction::ViewPayload => kb.view_payload.display(),
            VersionsAction::Add => kb.add.display(),
            VersionsAction::Disable => kb.disable.display(),
            VersionsAction::Enable => kb.enable.display(),
            VersionsAction::Destroy => kb.destroy.display(),
            VersionsAction::Reload => kb.reload.display(),
        }
    }

    // Payload actions
    pub fn matches_payload(&self, event: &KeyEvent, action: PayloadAction) -> bool {
        let kb = &self.keybindings.payload;
        match action {
            PayloadAction::Copy => kb.copy.matches(event),
            PayloadAction::Reload => kb.reload.matches(event),
        }
    }

    pub fn display_payload(&self, action: PayloadAction) -> String {
        let kb = &self.keybindings.payload;
        match action {
            PayloadAction::Copy => kb.copy.display(),
            PayloadAction::Reload => kb.reload.display(),
        }
    }

    // Dialog actions
    pub fn matches_dialog(&self, event: &KeyEvent, action: DialogAction) -> bool {
        let kb = &self.keybindings.dialog;
        match action {
            DialogAction::Confirm => kb.confirm.matches(event),
            DialogAction::Cancel => kb.cancel.matches(event),
            DialogAction::Dismiss => kb.dismiss.matches(event),
        }
    }

    pub fn display_dialog(&self, action: DialogAction) -> String {
        let kb = &self.keybindings.dialog;
        match action {
            DialogAction::Confirm => kb.confirm.display(),
            DialogAction::Cancel => kb.cancel.display(),
            DialogAction::Dismiss => kb.dismiss.display(),
        }
    }
}
