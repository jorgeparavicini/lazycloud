use crate::provider::gcp::secret_manager::message::SecretManagerMsg;
use crate::provider::gcp::secret_manager::model::{Secret, SecretVersion};
use crate::provider::gcp::secret_manager::SecretManagerView;
use crate::view::{ColumnDef, TableEvent, TableRow, TableView, View};
use crate::Theme;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Rect};
use ratatui::widgets::Cell;
use ratatui::Frame;

impl TableRow for SecretVersion {
    fn columns() -> &'static [ColumnDef] {
        static COLUMNS: &[ColumnDef] = &[
            ColumnDef::new("Version", Constraint::Length(10)),
            ColumnDef::new("State", Constraint::Length(12)),
            ColumnDef::new("Created", Constraint::Min(20)),
        ];
        COLUMNS
    }

    fn render_cells(&self, _theme: &Theme) -> Vec<Cell<'static>> {
        vec![
            Cell::from(self.version_id.clone()),
            Cell::from(self.state.clone()),
            Cell::from(self.created_at.clone()),
        ]
    }
}

pub struct VersionListView {
    secret: Secret,
    table: TableView<SecretVersion>,
}

impl VersionListView {
    pub fn new(secret: Secret, versions: Vec<SecretVersion>) -> Self {
        let title = format!(" Versions: {} ", secret.name);
        Self {
            secret,
            table: TableView::new(versions).with_title(title),
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

        if let Some(TableEvent::Activated(version)) = self.table.handle_key(key) {
            return Some(SecretManagerMsg::SelectVersion(
                self.secret.clone(),
                version,
            ));
        }

        None
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        self.table.render(frame, area, theme);
    }
}
