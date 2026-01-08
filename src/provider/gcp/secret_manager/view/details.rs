use crate::provider::gcp::secret_manager::message::SecretManagerMsg;
use crate::provider::gcp::secret_manager::model::{Secret, SecretPayload, SecretVersion};
use crate::view::{KeyResult, View};
use crate::Theme;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};
use ratatui::Frame;

pub struct PayloadView {
    secret: Secret,
    version: SecretVersion,
    payload: SecretPayload,
    scroll: u16,
}

impl PayloadView {
    pub fn new(secret: Secret, version: SecretVersion, payload: SecretPayload) -> Self {
        Self {
            secret,
            version,
            payload,
            scroll: 0,
        }
    }

    pub fn secret(&self) -> &Secret {
        &self.secret
    }

    pub fn version(&self) -> &SecretVersion {
        &self.version
    }

    pub fn payload(&self) -> &SecretPayload {
        &self.payload
    }
}

impl View for PayloadView {
    type Event = SecretManagerMsg;

    fn handle_key(&mut self, key: KeyEvent) -> KeyResult<Self::Event> {
        match key.code {
            KeyCode::Char('r') => SecretManagerMsg::ReloadData.into(),
            KeyCode::Char('y') => SecretManagerMsg::CopyPayload.into(),
            KeyCode::Down | KeyCode::Char('j') => {
                self.scroll = self.scroll.saturating_add(1);
                KeyResult::Consumed
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.scroll = self.scroll.saturating_sub(1);
                KeyResult::Consumed
            }
            _ => KeyResult::Ignored,
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {

        let text = format!(
            "Secret: {}\nVersion: {}\n\nPayload:\n{}",
            self.secret.name, self.version.version_id, self.payload.data
        );

        let p = Paragraph::new(text)
            .style(Style::default().fg(theme.text()))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(theme.border()))
                    .title("Secret Payload")
                    .title_style(Style::default().fg(theme.mauve()).add_modifier(Modifier::BOLD)),
            )
            .scroll((self.scroll, 0));

        frame.render_widget(p, area);
    }
}
