use crate::provider::gcp::secret_manager::message::SecretManagerMsg;
use crate::provider::gcp::secret_manager::model::{Secret, SecretVersion};
use crate::provider::gcp::secret_manager::view::ServiceView;
use crate::view::{ColumnDef, KeyResult, TableEvent, TableRow, TableView, View};
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

    fn matches(&self, query: &str) -> bool {
        let query_lower = query.to_lowercase();
        self.version_id.to_lowercase().contains(&query_lower)
            || self.state.to_lowercase().contains(&query_lower)
    }
}

pub struct VersionListView {
    secret: Secret,
    table: TableView<SecretVersion>,
}

impl VersionListView {
    pub fn new(secret: Secret, versions: Vec<SecretVersion>) -> Self {
        let title = format!(" {} - Versions ", secret.name);
        Self {
            secret,
            table: TableView::new(versions).with_title(title),
        }
    }
}

impl ServiceView for VersionListView {
    fn breadcrumbs(&self) -> Vec<String> {
        vec!["Versions".to_string()]
    }

    fn reload(&self) -> SecretManagerMsg {
        SecretManagerMsg::SelectSecret(self.secret.clone())
    }
}

impl View for VersionListView {
    type Event = SecretManagerMsg;

    fn handle_key(&mut self, key: KeyEvent) -> KeyResult<Self::Event> {
        // Delegate to table first (handles search mode, navigation, etc.)
        let result = self.table.handle_key(key);
        if let KeyResult::Event(TableEvent::Activated(version)) = result {
            return SecretManagerMsg::SelectVersion(self.secret.clone(), version).into();
        }
        if result.is_consumed() {
            return KeyResult::Consumed;
        }

        // Handle local shortcuts only if table didn't consume the key
        match key.code {
            KeyCode::Char('r') => SecretManagerMsg::ReloadData.into(),
            _ => KeyResult::Ignored,
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        self.table.render(frame, area, theme);
    }
}
