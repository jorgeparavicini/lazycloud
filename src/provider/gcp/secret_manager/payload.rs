use crate::Theme;
use crate::core::command::CopyToClipboardCmd;
use crate::core::{Command, UpdateResult};
use crate::provider::gcp::secret_manager::SecretManager;
use crate::provider::gcp::secret_manager::client::SecretManagerClient;
use crate::provider::gcp::secret_manager::secrets::Secret;
use crate::provider::gcp::secret_manager::service::SecretManagerMsg;
use crate::provider::gcp::secret_manager::versions::SecretVersion;
use crate::view::{KeyResult, View};
use async_trait::async_trait;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Borders, Paragraph};
use tokio::sync::mpsc::UnboundedSender;

// === Models ===

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretPayload {
    pub data: String,
    pub is_binary: bool,
}

// === Messages ===

#[derive(Debug, Clone)]
pub enum PayloadMsg {
    /// Load payload for a secret (optionally for a specific version)
    Load {
        secret: Secret,
        version: Option<SecretVersion>,
    },
    /// Payload loaded successfully
    Loaded {
        secret: Secret,
        version: Option<SecretVersion>,
        payload: SecretPayload,
    },
    /// Copy payload to clipboard
    Copy(String),
}

impl From<PayloadMsg> for SecretManagerMsg {
    fn from(msg: PayloadMsg) -> Self {
        SecretManagerMsg::Payload(msg)
    }
}

impl From<PayloadMsg> for KeyResult<SecretManagerMsg> {
    fn from(msg: PayloadMsg) -> Self {
        KeyResult::Event(SecretManagerMsg::Payload(msg))
    }
}

// === Screens ===

pub struct PayloadScreen {
    secret: Secret,
    version: Option<SecretVersion>,
    payload: SecretPayload,
}

impl PayloadScreen {
    pub fn new(secret: Secret, version: Option<SecretVersion>, payload: SecretPayload) -> Self {
        Self {
            secret,
            version,
            payload,
        }
    }
}

impl View for PayloadScreen {
    type Event = SecretManagerMsg;

    fn handle_key(&mut self, key: KeyEvent) -> KeyResult<Self::Event> {
        match key.code {
            KeyCode::Char('r') => PayloadMsg::Load {
                secret: self.secret.clone(),
                version: self.version.clone(),
            }
            .into(),
            KeyCode::Char('y') => PayloadMsg::Copy(self.payload.data.clone()).into(),
            _ => KeyResult::Ignored,
        }
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
}

// === Update Logic ===

pub(super) fn update(
    state: &mut SecretManager,
    msg: PayloadMsg,
) -> color_eyre::Result<UpdateResult> {
    match msg {
        PayloadMsg::Load { secret, version } => {
            // Use cached payload if available
            if let Some(payload) = state.get_cached_payload(&secret, &version) {
                state.push_view(PayloadScreen::new(secret, version, payload));
                return Ok(UpdateResult::Idle);
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
            state.push_view(PayloadScreen::new(secret, version, payload));
            Ok(UpdateResult::Idle)
        }

        PayloadMsg::Copy(data) => Ok(CopyToClipboardCmd::new(data).into()),
    }
}

// === Commands ===

/// Fetch payload for a specific version.
struct FetchPayloadCmd {
    client: SecretManagerClient,
    secret: Secret,
    version: SecretVersion,
    tx: UnboundedSender<SecretManagerMsg>,
}

#[async_trait]
impl Command for FetchPayloadCmd {
    fn name(&self) -> &'static str {
        "Loading secret payload"
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

/// Fetch payload for the latest version.
struct FetchLatestPayloadCmd {
    client: SecretManagerClient,
    secret: Secret,
    tx: UnboundedSender<SecretManagerMsg>,
}

#[async_trait]
impl Command for FetchLatestPayloadCmd {
    fn name(&self) -> &'static str {
        "Loading latest secret payload"
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
