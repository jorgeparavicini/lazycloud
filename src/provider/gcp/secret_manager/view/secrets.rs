use crate::provider::gcp::secret_manager::message::SecretManagerMsg;
use crate::provider::gcp::secret_manager::model::Secret;
use crate::provider::gcp::secret_manager::SecretManagerView;
use crate::widget::TableEvent::Activated;
use crate::widget::{Column, SelectTable};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Rect};
use ratatui::Frame;

pub struct SecretListView {
    table: SelectTable<Secret>,
}

impl SecretListView {
    pub fn new(secrets: Vec<Secret>) -> Self {
        Self {
            table: SelectTable::new(secrets).with_title(" Secrets "),
        }
    }
}

impl SecretManagerView for SecretListView {
    fn handle_key(&mut self, key: KeyEvent) -> Option<SecretManagerMsg> {
        match key.code {
            KeyCode::Char('r') => return Some(SecretManagerMsg::ReloadData),
            _ => {}
        };

        if let Some(event) = self.table.handle_key_event(key) {
            if let Activated(secret) = event {
                return Some(SecretManagerMsg::SelectSecret(secret.clone()));
            }
        }

        None
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let columns = [
            Column::new("Name", Constraint::Min(20)),
            Column::new("Created", Constraint::Length(20)),
            Column::new("Labels", Constraint::Length(10)),
        ];

        self.table.render(frame, area, &columns, |secret| {
            vec![
                secret.name.clone(),
                secret.created_at.clone(),
                format!("{}", secret.labels.len()),
            ]
        });
    }
}
