use crate::config::key::{Key, KeyBinding};
use crate::config::keybindings::*;
use crossterm::event::KeyCode;

impl Default for GlobalKeybindings {
    fn default() -> Self {
        Self {
            quit: Key::new(KeyCode::Char('q')).into(),
            help: Key::new(KeyCode::Char('?')).into(),
            theme: Key::new(KeyCode::Char('t')).into(),
            back: Key::new(KeyCode::Esc).into(),
            commands_toggle: Key::new(KeyCode::Char('c')).into(),
        }
    }
}

impl Default for NavigationKeybindings {
    fn default() -> Self {
        Self {
            up: KeyBinding::multiple(vec![
                Key::new(KeyCode::Char('k')),
                Key::new(KeyCode::Up),
            ]),
            down: KeyBinding::multiple(vec![
                Key::new(KeyCode::Char('j')),
                Key::new(KeyCode::Down),
            ]),
            page_up: Key::new(KeyCode::PageUp).into(),
            page_down: Key::new(KeyCode::PageDown).into(),
            home: KeyBinding::multiple(vec![
                Key::new(KeyCode::Char('g')),
                Key::new(KeyCode::Home),
            ]),
            end: KeyBinding::multiple(vec![
                Key::new(KeyCode::Char('G')),
                Key::new(KeyCode::End),
            ]),
            select: Key::new(KeyCode::Enter).into(),
        }
    }
}

impl Default for SearchKeybindings {
    fn default() -> Self {
        Self {
            toggle: Key::new(KeyCode::Char('/')).into(),
            exit: Key::new(KeyCode::Esc).into(),
        }
    }
}

impl Default for SecretListKeybindings {
    fn default() -> Self {
        Self {
            view_payload: Key::new(KeyCode::Enter).into(),
            copy: Key::new(KeyCode::Char('y')).into(),
            versions: Key::new(KeyCode::Char('v')).into(),
            new: Key::new(KeyCode::Char('n')).into(),
            delete: KeyBinding::multiple(vec![
                Key::new(KeyCode::Char('d')),
                Key::new(KeyCode::Delete),
            ]),
            labels: Key::new(KeyCode::Char('l')).into(),
            iam: Key::new(KeyCode::Char('i')).into(),
            replication: Key::new(KeyCode::Char('R')).into(),
            reload: Key::new(KeyCode::Char('r')).into(),
        }
    }
}

impl Default for VersionListKeybindings {
    fn default() -> Self {
        Self {
            view_payload: Key::new(KeyCode::Enter).into(),
            add: Key::new(KeyCode::Char('a')).into(),
            disable: Key::new(KeyCode::Char('d')).into(),
            enable: Key::new(KeyCode::Char('e')).into(),
            destroy: Key::new(KeyCode::Char('D')).into(),
            reload: Key::new(KeyCode::Char('r')).into(),
        }
    }
}

impl Default for PayloadKeybindings {
    fn default() -> Self {
        Self {
            copy: Key::new(KeyCode::Char('y')).into(),
            reload: Key::new(KeyCode::Char('r')).into(),
        }
    }
}

impl Default for DialogKeybindings {
    fn default() -> Self {
        Self {
            confirm: KeyBinding::multiple(vec![
                Key::new(KeyCode::Char('y')),
                Key::new(KeyCode::Char('Y')),
                Key::new(KeyCode::Enter),
            ]),
            cancel: KeyBinding::multiple(vec![
                Key::new(KeyCode::Char('n')),
                Key::new(KeyCode::Char('N')),
                Key::new(KeyCode::Esc),
            ]),
            dismiss: KeyBinding::multiple(vec![
                Key::new(KeyCode::Enter),
                Key::new(KeyCode::Esc),
                Key::new(KeyCode::Char('q')),
            ]),
        }
    }
}

impl Default for KeybindingsConfig {
    fn default() -> Self {
        Self {
            global: GlobalKeybindings::default(),
            navigation: NavigationKeybindings::default(),
            search: SearchKeybindings::default(),
            secrets: SecretListKeybindings::default(),
            versions: VersionListKeybindings::default(),
            payload: PayloadKeybindings::default(),
            dialog: DialogKeybindings::default(),
        }
    }
}
