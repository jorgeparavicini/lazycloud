use crate::provider::gcp::secret_manager::message::SecretManagerMsg;
use crate::provider::gcp::secret_manager::model::Secret;
use crate::provider::gcp::secret_manager::SecretManagerView;
use crate::view::{ColumnDef, TableEvent, TableRow, TableView, View};
use crate::Theme;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Rect};
use ratatui::widgets::Cell;
use ratatui::Frame;

impl TableRow for Secret {
    fn columns() -> &'static [ColumnDef] {
        static COLUMNS: &[ColumnDef] = &[
            ColumnDef::new("Name", Constraint::Min(20)),
            ColumnDef::new("Created", Constraint::Length(20)),
            ColumnDef::new("Labels", Constraint::Length(10)),
        ];
        COLUMNS
    }

    fn render_cells(&self, _theme: &Theme) -> Vec<Cell<'static>> {
        vec![
            Cell::from(self.name.clone()),
            Cell::from(self.created_at.clone()),
            Cell::from(format!("{}", self.labels.len())),
        ]
    }
}

pub struct SecretListView {
    table: TableView<Secret>,
}

impl SecretListView {
    pub fn new(secrets: Vec<Secret>) -> Self {
        Self {
            table: TableView::new(secrets).with_title(" Secrets "),
        }
    }
}

impl SecretManagerView for SecretListView {
    fn handle_key(&mut self, key: KeyEvent) -> Option<SecretManagerMsg> {
        match key.code {
            KeyCode::Char('r') => return Some(SecretManagerMsg::ReloadData),
            _ => {}
        };

        if let Some(TableEvent::Activated(secret)) = self.table.handle_key(key) {
            return Some(SecretManagerMsg::SelectSecret(secret));
        }

        None
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        self.table.render(frame, area, theme);
    }
}
