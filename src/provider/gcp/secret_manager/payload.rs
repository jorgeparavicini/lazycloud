use async_trait::async_trait;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::Paragraph;
use std::sync::Arc;

use tokio::sync::mpsc::UnboundedSender;

use crate::Theme;
use crate::commands::{Command, CommandCtx, CopyToClipboardCmd};
use crate::provider::gcp::secret_manager::SecretManager;
use crate::provider::gcp::secret_manager::client::SecretManagerClient;
use crate::provider::gcp::secret_manager::secrets::Secret;
use crate::provider::gcp::secret_manager::service::{SecretManagerMsg, SmDomainMsg};
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
        Self::Domain(SmDomainMsg::Payload(msg))
    }
}

impl From<PayloadMsg> for EventResult<SecretManagerMsg> {
    fn from(msg: PayloadMsg) -> Self {
        Self::Event(SecretManagerMsg::from(msg))
    }
}

// === Screens ===

pub struct PayloadScreen {
    secret: Secret,
    version: Option<SecretVersion>,
    payload: SecretPayload,
}

impl PayloadScreen {
    pub const fn new(
        secret: Secret,
        version: Option<SecretVersion>,
        payload: SecretPayload,
    ) -> Self {
        Self {
            secret,
            version,
            payload,
        }
    }
}

impl Screen for PayloadScreen {
    type Output = SecretManagerMsg;

    fn handle_key(&mut self, key: KeyEvent) -> Result<EventResult<Self::Output>> {
        if key.code == KeyCode::Char('r') {
            return Ok(PayloadMsg::Load {
                secret: self.secret.clone(),
                version: self.version.clone(),
            }
            .into());
        }
        if key.code == KeyCode::Char('y') {
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
        if key.code == KeyCode::Char('e') && !self.payload.is_binary {
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
            .block(theme.block().title(title).title_style(theme.title_style()));

        frame.render_widget(p, area);
    }

    fn keybindings(&self) -> Vec<Keybinding> {
        vec![
            Keybinding::hint("y", "Copy"),
            Keybinding::hint("e", "Edit"),
            Keybinding::new("r", "Reload"),
        ]
    }
}

// === Update Logic ===

pub(super) fn update(state: &mut SecretManager, msg: PayloadMsg) -> Result<ServiceMsg> {
    match msg {
        PayloadMsg::Load { secret, version } => {
            // Use cached payload if available
            if let Some(payload) = state
                .cache
                .get::<_, SecretPayload>(&get_cache_key(&secret, version.as_ref()))
            {
                state.views.push(PayloadScreen::new(secret, version, payload.clone()));
                return Ok(ServiceMsg::Idle);
            }

            state.views.set_loading("Loading payload...");

            match version {
                Some(v) => Ok(FetchPayloadCmd {
                    secret,
                    version: v,
                    client: state.get_client()?,
                    tx: state.clone_sender(),
                }
                .into()),
                None => Ok(FetchLatestPayloadCmd {
                    secret,
                    client: state.get_client()?,
                    tx: state.clone_sender(),
                }
                .into()),
            }
        }

        PayloadMsg::Loaded {
            secret,
            version,
            payload,
        } => {
            state.views.clear_loading();
            state
                .cache
                .insert(get_cache_key(&secret, version.as_ref()), payload.clone());
            state.views.push(PayloadScreen::new(secret, version, payload));
            Ok(ServiceMsg::Idle)
        }

        PayloadMsg::Copy { data, description } => {
            Ok(CopyToClipboardCmd::new(data, description).into())
        }

        PayloadMsg::Edit { secret, data } => {
            state.logic.editing_secret = Some(secret);
            Ok(ServiceMsg::EditExternal { content: data })
        }

        PayloadMsg::SaveEdit { secret, new_data } => {
            state.views.set_loading("Saving new version...");
            state.cache.invalidate::<_, SecretVersion>(&secret);

            Ok(SaveEditCmd {
                secret,
                new_data,
                client: state.get_client()?,
                tx: state.clone_sender(),
            }
            .into())
        }

        PayloadMsg::EditSaved { secret } => {
            state.views.clear_loading();
            state.views.pop();
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

fn get_cache_key(secret: &Secret, version: Option<&SecretVersion>) -> String {
    version.map_or_else(
        || format!("{}:latest", secret.name),
        |v| format!("{}:{}", secret.name, v.version_id),
    )
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

    async fn execute(self: Box<Self>, _ctx: Arc<dyn CommandCtx>) -> Result<()> {
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

    async fn execute(self: Box<Self>, ctx: Arc<dyn CommandCtx>) -> Result<()> {
        self.client
            .add_secret_version(&self.secret.name, self.new_data.as_bytes())
            .await?;
        ctx.toast(
            format!("New version created for '{}'", self.secret.name),
            crate::ui::ToastType::Success,
        );
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

    async fn execute(self: Box<Self>, _ctx: Arc<dyn CommandCtx>) -> Result<()> {
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
