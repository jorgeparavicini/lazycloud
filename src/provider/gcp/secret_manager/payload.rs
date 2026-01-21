use std::sync::Arc;

use async_trait::async_trait;
use crossterm::event::KeyEvent;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Borders, Paragraph};
use tokio::sync::mpsc::UnboundedSender;

use crate::Theme;
use crate::components::Keybinding;
use crate::config::{KeyResolver, PayloadAction};
use crate::commands::CopyToClipboardCmd;
use crate::core::{Command, ServiceMsg};
use crate::provider::gcp::secret_manager::SecretManager;
use crate::provider::gcp::secret_manager::client::SecretManagerClient;
use crate::provider::gcp::secret_manager::secrets::Secret;
use crate::provider::gcp::secret_manager::service::SecretManagerMsg;
use crate::provider::gcp::secret_manager::versions::SecretVersion;
use crate::components::{Handled, Result, Screen};

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
}

impl From<PayloadMsg> for SecretManagerMsg {
    fn from(msg: PayloadMsg) -> Self {
        SecretManagerMsg::Payload(msg)
    }
}

impl From<PayloadMsg> for Handled<SecretManagerMsg> {
    fn from(msg: PayloadMsg) -> Self {
        Handled::Event(SecretManagerMsg::Payload(msg))
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
    pub fn new(
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
    type Msg = SecretManagerMsg;

    fn handle_key(&mut self, key: KeyEvent) -> Result<Handled<Self::Msg>> {
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
        Ok(Handled::Ignored)
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let version = match &self.version {
            Some(v) => v.version_id.as_str(),
            None => "latest",
        };
        let title = format!(" {} - v{} ", self.secret.name, version);

        let p = Paragraph::new(self.payload.data.as_str())
            .style(Style::default().fg(theme.text()))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(theme.border_type)
                    .border_style(Style::default().fg(theme.border()))
                    .title(title)
                    .title_style(
                        Style::default()
                            .fg(theme.mauve())
                            .add_modifier(Modifier::BOLD),
                    ),
            );

        frame.render_widget(p, area);
    }

    fn keybindings(&self) -> Vec<Keybinding> {
        vec![
            Keybinding::hint(self.resolver.display_payload(PayloadAction::Copy), "Copy"),
            Keybinding::new(
                self.resolver.display_payload(PayloadAction::Reload),
                "Reload",
            ),
        ]
    }
}

// === Update Logic ===

pub(super) fn update(
    state: &mut SecretManager,
    msg: PayloadMsg,
) -> color_eyre::Result<ServiceMsg> {
    match msg {
        PayloadMsg::Load { secret, version } => {
            // Use cached payload if available
            if let Some(payload) = state.get_cached_payload(&secret, &version) {
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
            state.cache_payload(&secret, &version, payload.clone());
            state.push_view(PayloadScreen::new(
                secret,
                version,
                payload,
                state.get_resolver(),
            ));
            Ok(ServiceMsg::Idle)
        }

        PayloadMsg::Copy { data, description } => {
            Ok(CopyToClipboardCmd::new(data, description, state.get_cmd_env()).into())
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

    async fn execute(self: Box<Self>) -> color_eyre::Result<()> {
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

    async fn execute(self: Box<Self>) -> color_eyre::Result<()> {
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
