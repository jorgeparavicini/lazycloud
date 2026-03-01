use std::sync::Arc;

use async_trait::async_trait;
use crossterm::event::KeyEvent;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::Paragraph;
use tokio::sync::mpsc::UnboundedSender;

use crate::Theme;
use crate::app::AppMessage;
use crate::commands::{Command, CopyToClipboardCmd};
use crate::config::{KeyResolver, PayloadAction};
use crate::provider::gcp::secret_manager::SecretManager;
use crate::provider::gcp::secret_manager::client::SecretManagerClient;
use crate::provider::gcp::secret_manager::secrets::Secret;
use crate::provider::gcp::secret_manager::service::SecretManagerMsg;
use crate::provider::gcp::secret_manager::versions::SecretVersion;
use crate::service::ServiceMsg;
use crate::ui::{EventResult, Keybinding, Result, Screen};

// === Models ===

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretPayload {
    pub data: String,
    pub is_binary: bool,
}

// === Messages ===

#[derive(Debug, Clone)]
pub enum PayloadMsg {
    Load {
        secret: Secret,
        version: Option<SecretVersion>,
    },
    Loaded {
        secret: Secret,
        version: Option<SecretVersion>,
        payload: SecretPayload,
    },
    Copy {
        data: String,
        description: String,
    },
    Edit {
        secret: Secret,
        data: String,
    },
    SaveEdit {
        secret: Secret,
        new_data: String,
    },
    EditSaved {
        secret: Secret,
    },
}

impl From<PayloadMsg> for SecretManagerMsg {
    fn from(msg: PayloadMsg) -> Self {
        Self::Payload(msg)
    }
}

impl From<PayloadMsg> for EventResult<SecretManagerMsg> {
    fn from(msg: PayloadMsg) -> Self {
        Self::Event(SecretManagerMsg::Payload(msg))
    }
}

// === Screens ===

pub struct PayloadScreen {
    secret: Secret,
    version: Option<SecretVersion>,
    payload: SecretPayload,
    resolver: Arc<KeyResolver>,
}

impl PayloadScreen {
    pub const fn new(
        secret: Secret,
        version: Option<SecretVersion>,
        payload: SecretPayload,
        resolver: Arc<KeyResolver>,
    ) -> Self {
        Self {
            secret,
            version,
            payload,
            resolver,
        }
    }
}

impl Screen for PayloadScreen {
    type Output = SecretManagerMsg;

    fn handle_key(&mut self, key: KeyEvent) -> Result<EventResult<Self::Output>> {
        if self.resolver.matches_payload(&key, PayloadAction::Reload) {
            return Ok(PayloadMsg::Load {
                secret: self.secret.clone(),
                version: self.version.clone(),
            }
            .into());
        }
        if self.resolver.matches_payload(&key, PayloadAction::Copy) {
            let description = match &self.version {
                Some(v) => format!("payload for '{}' (v{})", self.secret.name, v.version_id),
                None => format!("payload for '{}' (latest)", self.secret.name),
            };
            return Ok(PayloadMsg::Copy {
                data: self.payload.data.clone(),
                description,
            }
            .into());
        }
        if self.resolver.matches_payload(&key, PayloadAction::Edit) && !self.payload.is_binary {
            return Ok(PayloadMsg::Edit {
                secret: self.secret.clone(),
                data: self.payload.data.clone(),
            }
            .into());
        }
        Ok(EventResult::Ignored)
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let version = self
            .version
            .as_ref()
            .map_or("latest", |v| v.version_id.as_str());
        let title = format!(" {} - v{} ", self.secret.name, version);

        let p = Paragraph::new(self.payload.data.as_str())
            .style(Style::default().fg(theme.text()))
            .block(
                theme.block()
                    .title(title)
                    .title_style(theme.title_style()),
            );

        frame.render_widget(p, area);
    }

    fn keybindings(&self) -> Vec<Keybinding> {
        let mut bindings = vec![
            Keybinding::hint(self.resolver.display_payload(PayloadAction::Copy), "Copy"),
            Keybinding::hint(self.resolver.display_payload(PayloadAction::Edit), "Edit"),
        ];
        bindings.push(Keybinding::new(
            self.resolver.display_payload(PayloadAction::Reload),
            "Reload",
        ));
        bindings
    }
}

// === Update Logic ===

pub(super) fn update(state: &mut SecretManager, msg: PayloadMsg) -> Result<ServiceMsg> {
    match msg {
        PayloadMsg::Load { secret, version } => {
            // Use cached payload if available
            if let Some(payload) = state.get_cached_payload(&secret, version.as_ref()) {
                state.push_view(PayloadScreen::new(
                    secret,
                    version,
                    payload,
                    state.get_resolver(),
                ));
                return Ok(ServiceMsg::Idle);
            }

            state.display_loading_spinner("Loading payload...");

            match version {
                Some(v) => Ok(FetchPayloadCmd {
                    secret,
                    version: v,
                    client: state.get_client()?,
                    tx: state.get_msg_sender(),
                }
                .into()),
                None => Ok(FetchLatestPayloadCmd {
                    secret,
                    client: state.get_client()?,
                    tx: state.get_msg_sender(),
                }
                .into()),
            }
        }

        PayloadMsg::Loaded {
            secret,
            version,
            payload,
        } => {
            state.hide_loading_spinner();
            state.cache_payload(&secret, version.as_ref(), payload.clone());
            state.push_view(PayloadScreen::new(
                secret,
                version,
                payload,
                state.get_resolver(),
            ));
            Ok(ServiceMsg::Idle)
        }

        PayloadMsg::Copy { data, description } => {
            Ok(CopyToClipboardCmd::new(data, description).into())
        }

        PayloadMsg::Edit { secret, data } => {
            state.set_editing_secret(secret);
            Ok(ServiceMsg::EditExternal { content: data })
        }

        PayloadMsg::SaveEdit { secret, new_data } => {
            state.display_loading_spinner("Saving new version...");
            state.invalidate_payload_cache(&secret);
            state.invalidate_versions_cache(&secret);

            Ok(SaveEditCmd {
                secret,
                new_data,
                client: state.get_client()?,
                tx: state.get_msg_sender(),
            }
            .into())
        }

        PayloadMsg::EditSaved { secret } => {
            state.hide_loading_spinner();
            state.pop_view();
            state.queue(
                PayloadMsg::Load {
                    secret,
                    version: None,
                }
                .into(),
            );
            Ok(ServiceMsg::Idle)
        }
    }
}

// === Commands ===

struct FetchPayloadCmd {
    client: SecretManagerClient,
    secret: Secret,
    version: SecretVersion,
    tx: UnboundedSender<SecretManagerMsg>,
}

#[async_trait]
impl Command for FetchPayloadCmd {
    fn name(&self) -> String {
        format!(
            "Loading '{}' v{}",
            self.secret.name, self.version.version_id
        )
    }

    async fn execute(self: Box<Self>, _action_tx: UnboundedSender<AppMessage>) -> Result<()> {
        let payload = self
            .client
            .access_version(&self.secret.name, &self.version.version_id)
            .await?;
        self.tx.send(
            PayloadMsg::Loaded {
                secret: self.secret,
                version: Some(self.version),
                payload,
            }
            .into(),
        )?;
        Ok(())
    }
}

struct SaveEditCmd {
    client: SecretManagerClient,
    secret: Secret,
    new_data: String,
    tx: UnboundedSender<SecretManagerMsg>,
}

#[async_trait]
impl Command for SaveEditCmd {
    fn name(&self) -> String {
        format!("Saving edit to '{}'", self.secret.name)
    }

    async fn execute(self: Box<Self>, action_tx: UnboundedSender<AppMessage>) -> Result<()> {
        self.client
            .add_secret_version(&self.secret.name, self.new_data.as_bytes())
            .await?;
        let _ = action_tx.send(AppMessage::ShowToast {
            message: format!("New version created for '{}'", self.secret.name),
            toast_type: crate::ui::ToastType::Success,
        });
        self.tx.send(
            PayloadMsg::EditSaved {
                secret: self.secret,
            }
            .into(),
        )?;
        Ok(())
    }
}

struct FetchLatestPayloadCmd {
    client: SecretManagerClient,
    secret: Secret,
    tx: UnboundedSender<SecretManagerMsg>,
}

#[async_trait]
impl Command for FetchLatestPayloadCmd {
    fn name(&self) -> String {
        format!("Loading '{}' (latest)", self.secret.name)
    }

    async fn execute(self: Box<Self>, _action_tx: UnboundedSender<AppMessage>) -> Result<()> {
        let payload = self.client.access_latest_version(&self.secret.name).await?;
        self.tx.send(
            PayloadMsg::Loaded {
                secret: self.secret,
                version: None,
                payload,
            }
            .into(),
        )?;
        Ok(())
    }
}
