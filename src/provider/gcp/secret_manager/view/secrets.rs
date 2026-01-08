use crate::provider::gcp::secret_manager::message::SecretManagerMsg;
use crate::provider::gcp::secret_manager::model::Secret;
use crate::view::{ColumnDef, KeyResult, TableEvent, TableRow, TableView, View};
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

    fn matches(&self, query: &str) -> bool {
        let query_lower = query.to_lowercase();
        self.name.to_lowercase().contains(&query_lower)
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

impl View for SecretListView {
    type Event = SecretManagerMsg;

    fn handle_key(&mut self, key: KeyEvent) -> KeyResult<Self::Event> {
        // Delegate to table first (handles search mode, navigation, etc.)
        let result = self.table.handle_key(key);
        if let KeyResult::Event(TableEvent::Activated(secret)) = result {
            return SecretManagerMsg::SelectSecret(secret).into();
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
