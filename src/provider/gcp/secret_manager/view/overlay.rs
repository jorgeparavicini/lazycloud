//! Overlay views for Secret Manager dialogs.

use crate::provider::gcp::secret_manager::message::SecretManagerMsg;
use crate::provider::gcp::secret_manager::model::{Secret, SecretVersion};
use crate::view::{ConfirmDialog, ConfirmEvent, KeyResult, TextInputEvent, TextInputView, View};
use crate::Theme;
use crossterm::event::KeyEvent;
use ratatui::layout::Rect;
use ratatui::Frame;

// === Create Secret Flow ===

pub struct CreateSecretNameOverlay {
    input: TextInputView,
}

impl CreateSecretNameOverlay {
    pub fn new() -> Self {
        Self {
            input: TextInputView::new("Secret Name").with_placeholder("my-secret"),
        }
    }
}

impl View for CreateSecretNameOverlay {
    type Event = SecretManagerMsg;

    fn handle_key(&mut self, key: KeyEvent) -> KeyResult<Self::Event> {
        match self.input.handle_key(key) {
            KeyResult::Event(TextInputEvent::Submitted(name)) if !name.is_empty() => {
                SecretManagerMsg::CreateSecretStep2 { name }.into()
            }
            KeyResult::Event(_) => SecretManagerMsg::DialogCancelled.into(),
            _ => KeyResult::Consumed,
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        self.input.render(frame, area, theme);
    }
}

pub struct CreateSecretPayloadOverlay {
    name: String,
    input: TextInputView,
}

impl CreateSecretPayloadOverlay {
    pub fn new(name: String) -> Self {
        Self {
            name,
            input: TextInputView::new("Initial Payload (optional)"),
        }
    }
}

impl View for CreateSecretPayloadOverlay {
    type Event = SecretManagerMsg;

    fn handle_key(&mut self, key: KeyEvent) -> KeyResult<Self::Event> {
        match self.input.handle_key(key) {
            KeyResult::Event(TextInputEvent::Submitted(payload)) => {
                let payload = if payload.is_empty() { None } else { Some(payload) };
                SecretManagerMsg::CreateSecret {
                    name: self.name.clone(),
                    payload,
                }
                .into()
            }
            KeyResult::Event(TextInputEvent::Cancelled) => SecretManagerMsg::DialogCancelled.into(),
            _ => KeyResult::Consumed,
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        self.input.render(frame, area, theme);
    }
}

// === Delete Secret ===

pub struct DeleteSecretOverlay {
    secret: Secret,
    dialog: ConfirmDialog,
}

impl DeleteSecretOverlay {
    pub fn new(secret: Secret) -> Self {
        let dialog = ConfirmDialog::new(format!(
            "Delete secret '{}'? This cannot be undone.",
            secret.name
        ))
        .with_title("Delete Secret")
        .with_confirm_text("Delete")
        .with_cancel_text("No")
        .danger();

        Self { secret, dialog }
    }
}

impl View for DeleteSecretOverlay {
    type Event = SecretManagerMsg;

    fn handle_key(&mut self, key: KeyEvent) -> KeyResult<Self::Event> {
        match self.dialog.handle_key(key) {
            KeyResult::Event(ConfirmEvent::Confirmed) => {
                SecretManagerMsg::DeleteSecret(self.secret.clone()).into()
            }
            KeyResult::Event(ConfirmEvent::Cancelled) => SecretManagerMsg::DialogCancelled.into(),
            _ => KeyResult::Consumed,
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        self.dialog.render(frame, area, theme);
    }
}

// === Create Version ===

pub struct CreateVersionOverlay {
    secret: Secret,
    input: TextInputView,
}

impl CreateVersionOverlay {
    pub fn new(secret: Secret) -> Self {
        Self {
            secret,
            input: TextInputView::new("New Version Payload"),
        }
    }
}

impl View for CreateVersionOverlay {
    type Event = SecretManagerMsg;

    fn handle_key(&mut self, key: KeyEvent) -> KeyResult<Self::Event> {
        match self.input.handle_key(key) {
            KeyResult::Event(TextInputEvent::Submitted(payload)) if !payload.is_empty() => {
                SecretManagerMsg::CreateVersion {
                    secret: self.secret.clone(),
                    payload,
                }
                .into()
            }
            KeyResult::Event(_) => SecretManagerMsg::DialogCancelled.into(),
            _ => KeyResult::Consumed,
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        self.input.render(frame, area, theme);
    }
}

// === Destroy Version ===

pub struct DestroyVersionOverlay {
    secret: Secret,
    version: SecretVersion,
    dialog: ConfirmDialog,
}

impl DestroyVersionOverlay {
    pub fn new(secret: Secret, version: SecretVersion) -> Self {
        let dialog = ConfirmDialog::new(format!(
            "Destroy version '{}'? This is permanent and cannot be undone.",
            version.version_id
        ))
        .with_title("Destroy Version")
        .with_confirm_text("Destroy")
        .danger();

        Self {
            secret,
            version,
            dialog,
        }
    }
}

impl View for DestroyVersionOverlay {
    type Event = SecretManagerMsg;

    fn handle_key(&mut self, key: KeyEvent) -> KeyResult<Self::Event> {
        match self.dialog.handle_key(key) {
            KeyResult::Event(ConfirmEvent::Confirmed) => SecretManagerMsg::DestroyVersion {
                secret: self.secret.clone(),
                version: self.version.clone(),
            }
            .into(),
            KeyResult::Event(ConfirmEvent::Cancelled) => SecretManagerMsg::DialogCancelled.into(),
            _ => KeyResult::Consumed,
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        self.dialog.render(frame, area, theme);
    }
}
