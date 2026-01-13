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
use std::collections::HashMap;

/// Format labels for display in the table.
/// When a query is provided, shows the best matching label first.
fn format_labels(labels: &HashMap<String, String>, query: &str) -> String {
    if labels.is_empty() {
        return "—".to_string();
    }

    // Find the best matching label if there's a query
    let best_label = if !query.is_empty() {
        let matcher = Matcher::new();
        labels
            .iter()
            .filter(|(key, value)| {
                matcher.matches(format!("{}:{}", key, value).as_str(), query)
            })
            .next()
            .or_else(|| labels.iter().next())
    } else {
        labels.iter().next()
    };

    if let Some((key, value)) = best_label {
        let label = if value.is_empty() {
            key.clone()
        } else {
            format!("{}:{}", key, value)
        };

        // Truncate if too long
        if label.len() > 20 {
            let suffix = if labels.len() > 1 {
                format!("… +{}", labels.len() - 1)
            } else {
                "…".to_string()
            };
            format!("{}{}", &label[..17], suffix)
        } else if labels.len() > 1 {
            format!("{} +{}", label, labels.len() - 1)
        } else {
            label
        }
    } else {
        "—".to_string()
    }
}

impl TableRow for Secret {
    fn columns() -> &'static [ColumnDef] {
        static COLUMNS: &[ColumnDef] = &[
            ColumnDef::new("Name", Constraint::Min(20)),
            ColumnDef::new("Replication", Constraint::Length(14)),
            ColumnDef::new("Created", Constraint::Length(18)),
            ColumnDef::new("Expiration", Constraint::Length(18)),
            ColumnDef::new("Labels", Constraint::Length(23)),
        ];
        COLUMNS
    }

    fn render_cells(&self, theme: &Theme) -> Vec<Cell<'static>> {
        self.render_cells_with_query(theme, "")
    }

    fn render_cells_with_query(&self, _theme: &Theme, query: &str) -> Vec<Cell<'static>> {
        let labels_display = format_labels(&self.labels, query);
        let expiration = self.expire_time.clone().unwrap_or_else(|| "—".to_string());

        vec![
            Cell::from(self.name.clone()),
            Cell::from(self.replication.short_display()),
            Cell::from(self.created_at.clone()),
            Cell::from(expiration),
            Cell::from(labels_display),
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
