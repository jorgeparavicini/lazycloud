use crate::config::key::KeyBinding;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalKeybindings {
    pub quit: KeyBinding,
    pub help: KeyBinding,
    pub theme: KeyBinding,
    pub back: KeyBinding,
    pub commands_toggle: KeyBinding,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NavigationKeybindings {
    pub up: KeyBinding,
    pub down: KeyBinding,
    pub page_up: KeyBinding,
    pub page_down: KeyBinding,
    pub home: KeyBinding,
    pub end: KeyBinding,
    pub select: KeyBinding,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchKeybindings {
    pub toggle: KeyBinding,
    pub exit: KeyBinding,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretListKeybindings {
    pub view_payload: KeyBinding,
    pub copy: KeyBinding,
    pub versions: KeyBinding,
    pub new: KeyBinding,
    pub delete: KeyBinding,
    pub labels: KeyBinding,
    pub iam: KeyBinding,
    pub replication: KeyBinding,
    pub reload: KeyBinding,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionListKeybindings {
    pub view_payload: KeyBinding,
    pub add: KeyBinding,
    pub disable: KeyBinding,
    pub enable: KeyBinding,
    pub destroy: KeyBinding,
    pub reload: KeyBinding,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayloadKeybindings {
    pub copy: KeyBinding,
    pub reload: KeyBinding,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogKeybindings {
    pub confirm: KeyBinding,
    pub cancel: KeyBinding,
    pub dismiss: KeyBinding,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeybindingsConfig {
    pub global: GlobalKeybindings,
    pub navigation: NavigationKeybindings,
    pub search: SearchKeybindings,
    pub secrets: SecretListKeybindings,
    pub versions: VersionListKeybindings,
    pub payload: PayloadKeybindings,
    pub dialog: DialogKeybindings,
}
