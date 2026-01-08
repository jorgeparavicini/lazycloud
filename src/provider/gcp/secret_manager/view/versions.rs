use crate::provider::gcp::secret_manager::message::SecretManagerMsg;
use crate::provider::gcp::secret_manager::model::{Secret, SecretVersion};
use crate::provider::gcp::secret_manager::SecretManagerView;
use crate::widget::TableEvent::Activated;
use crate::widget::{Column, SelectTable};
use crate::Theme;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Rect};
use ratatui::Frame;

pub struct VersionListView {
    secret: Secret,
    table: SelectTable<SecretVersion>,
}

impl VersionListView {
    pub fn new(secret: Secret, versions: Vec<SecretVersion>) -> Self {
        let title = format!(" Versions: {} ", secret.name);
        Self {
            secret,
            table: SelectTable::new(versions).with_title(title),
        }
    }

    pub fn secret(&self) -> &Secret {
        &self.secret
    }
}

impl SecretManagerView for VersionListView {
    fn handle_key(&mut self, key: KeyEvent) -> Option<SecretManagerMsg> {
        match key.code {
            KeyCode::Char('r') => return Some(SecretManagerMsg::ReloadData),
            _ => {}
        };

        if let Some(event) = self.table.handle_key_event(key) {
            if let Activated(version) = event {
                return Some(SecretManagerMsg::SelectVersion(
                    self.secret.clone(),
                    version.clone(),
                ));
            }
        }

        None
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let columns = [
            Column::new("Version", Constraint::Length(10)),
            Column::new("State", Constraint::Length(12)),
            Column::new("Created", Constraint::Min(20)),
        ];

        self.table.render(frame, area, &columns, |version| {
            vec![
                version.version_id.clone(),
                version.state.clone(),
                version.created_at.clone(),
            ]
        }, theme);
    }
}
