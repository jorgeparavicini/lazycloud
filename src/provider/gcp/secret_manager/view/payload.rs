use crate::Theme;
use crate::provider::gcp::secret_manager::message::SecretManagerMsg;
use crate::provider::gcp::secret_manager::model::{Secret, SecretPayload, SecretVersion};
use crate::provider::gcp::secret_manager::view::SecretManagerView;
use crate::view::{KeyResult, View};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};

pub struct PayloadView {
    secret: Secret,
    version: Option<SecretVersion>,
    payload: SecretPayload,
}

impl PayloadView {
    pub fn new(secret: Secret, version: Option<SecretVersion>, payload: SecretPayload) -> Self {
        Self {
            secret,
            version,
            payload,
        }
    }
}

impl SecretManagerView for PayloadView {
    fn breadcrumbs(&self) -> Vec<String> {
        vec!["Payload".to_string()]
    }

    fn reload(&self) -> SecretManagerMsg {
        SecretManagerMsg::LoadPayload(self.secret.clone(), self.version.clone())
    }
}

impl View for PayloadView {
    type Event = SecretManagerMsg;

    fn handle_key(&mut self, key: KeyEvent) -> KeyResult<Self::Event> {
        match key.code {
            KeyCode::Char('r') => SecretManagerMsg::ReloadData.into(),
            KeyCode::Char('y') => SecretManagerMsg::CopyPayload(self.payload.data.clone()).into(),
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
