#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlobalAction {
    Quit,
    Help,
    Theme,
    Back,
    CommandsToggle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavAction {
    Up,
    Down,
    PageUp,
    PageDown,
    Home,
    End,
    Select,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchAction {
    Toggle,
    Exit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretsAction {
    ViewPayload,
    Copy,
    Versions,
    New,
    Delete,
    Labels,
    Iam,
    Replication,
    Reload,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VersionsAction {
    ViewPayload,
    Add,
    Disable,
    Enable,
    Destroy,
    Reload,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PayloadAction {
    Copy,
    Reload,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogAction {
    Confirm,
    Cancel,
    Dismiss,
}
