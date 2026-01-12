use crate::provider::gcp::secret_manager::message::SecretManagerMsg;
use crate::provider::gcp::secret_manager::model::Secret;
use crate::provider::gcp::secret_manager::view::SecretManagerView;
use crate::search::Matcher;
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
            ColumnDef::new("Created", Constraint::Length(18)),
            ColumnDef::new("Labels", Constraint::Min(30)),
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
        let matcher = Matcher::new();

        // Check for label filter syntax: "key:value" or "key:"
        if let Some(colon_pos) = query.find(':') {
            let key_pattern = &query[..colon_pos];
            let value_pattern = &query[colon_pos + 1..];

            // Find labels matching the key pattern (fuzzy)
            for (key, value) in &self.labels {
                if matcher.matches(key, key_pattern) {
                    // If value pattern is empty, match any value
                    if value_pattern.is_empty() {
                        return true;
                    }
                    // Otherwise, check if value matches (fuzzy)
                    if matcher.matches(value, value_pattern) {
                        return true;
                    }
                }
            }
            return false;
        }

        // Regular fuzzy search: match name or any label key/value
        if matcher.matches(&self.name, query) {
            return true;
        }

        // Check label keys and values
        for (key, value) in &self.labels {
            if matcher.matches(key, query) || matcher.matches(value, query) {
                return true;
            }
        }

        false
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

    pub fn selected_secret(&self) -> Option<&Secret> {
        self.table.selected_item()
    }
}

impl SecretManagerView for SecretListView {
    fn breadcrumbs(&self) -> Vec<String> {
        vec!["Secrets".to_string()]
    }

    fn reload(&self) -> SecretManagerMsg {
        SecretManagerMsg::LoadSecrets
    }
}

impl View for SecretListView {
    type Event = SecretManagerMsg;

    fn handle_key(&mut self, key: KeyEvent) -> KeyResult<Self::Event> {
        // Delegate to table first (handles search mode, navigation, etc.)
        let result = self.table.handle_key(key);
        if let KeyResult::Event(TableEvent::Activated(secret)) = result {
            return SecretManagerMsg::LoadPayload(secret, None).into();
        }
        if result.is_consumed() {
            return KeyResult::Consumed;
        }

        // Handle local shortcuts only if table didn't consume the key
        match key.code {
            KeyCode::Char('r') => SecretManagerMsg::ReloadData.into(),
            KeyCode::Char('v') => match self.selected_secret() {
                None => KeyResult::Ignored,
                Some(secret) => SecretManagerMsg::LoadVersions(secret.clone()).into(),
            },
            // Create new secret
            KeyCode::Char('n') | KeyCode::Char('c') => {
                SecretManagerMsg::ShowCreateSecretDialog.into()
            }
            // Delete selected secret
            KeyCode::Char('d') | KeyCode::Delete => match self.selected_secret() {
                None => KeyResult::Ignored,
                Some(secret) => SecretManagerMsg::ShowDeleteSecretDialog(secret.clone()).into(),
            },
            // View/edit labels
            KeyCode::Char('l') => match self.selected_secret() {
                None => KeyResult::Ignored,
                Some(secret) => SecretManagerMsg::ShowLabels(secret.clone()).into(),
            },
            // View IAM policy
            KeyCode::Char('i') => match self.selected_secret() {
                None => KeyResult::Ignored,
                Some(secret) => SecretManagerMsg::ShowIamPolicy(secret.clone()).into(),
            },
            // View replication info
            KeyCode::Char('R') => match self.selected_secret() {
                None => KeyResult::Ignored,
                Some(secret) => SecretManagerMsg::ShowReplicationInfo(secret.clone()).into(),
            },
            _ => KeyResult::Ignored,
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        self.table.render(frame, area, theme);
    }
}
