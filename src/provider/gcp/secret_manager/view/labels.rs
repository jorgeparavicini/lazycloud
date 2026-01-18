use crate::provider::gcp::secret_manager::message::SecretManagerMsg;
use crate::provider::gcp::secret_manager::model::Secret;
use crate::provider::gcp::secret_manager::view::SecretManagerView;
use crate::search::Matcher;
use crate::view::{ColumnDef, Keybinding, KeyResult, TableEvent, TableRow, TableView, View};
use crate::Theme;

const LABELS_KEYBINDINGS: &[Keybinding] = &[
    Keybinding::hint("/", "Search"),
    Keybinding::new("r", "Reload"),
];
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Rect};
use ratatui::widgets::Cell;
use ratatui::Frame;

/// A label entry for display in the table.
#[derive(Clone, Debug)]
pub struct LabelEntry {
    pub key: String,
    pub value: String,
}

impl TableRow for LabelEntry {
    fn columns() -> &'static [ColumnDef] {
        static COLUMNS: &[ColumnDef] = &[
            ColumnDef::new("Key", Constraint::Min(20)),
            ColumnDef::new("Value", Constraint::Min(30)),
        ];
        COLUMNS
    }

    fn render_cells(&self, _theme: &Theme) -> Vec<Cell<'static>> {
        vec![Cell::from(self.key.clone()), Cell::from(self.value.clone())]
    }

    fn matches(&self, query: &str) -> bool {
        let matcher = Matcher::new();
        matcher.matches(&self.key, query) || matcher.matches(&self.value, query)
    }
}

pub struct LabelsView {
    secret: Secret,
    table: TableView<LabelEntry>,
}

impl LabelsView {
    pub fn new(secret: Secret) -> Self {
        let labels: Vec<LabelEntry> = secret
            .labels
            .iter()
            .map(|(k, v)| LabelEntry {
                key: k.clone(),
                value: v.clone(),
            })
            .collect();

        let title = format!(" {} - Labels ", secret.name);
        Self {
            secret,
            table: TableView::new(labels).with_title(title),
        }
    }

    /// Get the selected label entry.
    pub fn selected_label(&self) -> Option<&LabelEntry> {
        self.table.selected_item()
    }
}

impl SecretManagerView for LabelsView {
    fn breadcrumbs(&self) -> Vec<String> {
        vec!["Labels".to_string()]
    }

    fn reload(&self) -> SecretManagerMsg {
        SecretManagerMsg::ShowLabels(self.secret.clone())
    }
}

impl View for LabelsView {
    type Event = SecretManagerMsg;

    fn handle_key(&mut self, key: KeyEvent) -> KeyResult<Self::Event> {
        // Delegate to table first (handles search mode, navigation, etc.)
        let result = self.table.handle_key(key);
        if let KeyResult::Event(TableEvent::Activated(_)) = result {
            // Enter on a label could open edit mode in the future
            return KeyResult::Consumed;
        }
        if result.is_consumed() {
            return KeyResult::Consumed;
        }

        // Handle local shortcuts only if table didn't consume the key
        match key.code {
            KeyCode::Char('r') => SecretManagerMsg::ReloadData.into(),
            // Note: Label editing/deletion requires additional TableView methods
            // that are not yet implemented. For now, this is a read-only view.
            _ => KeyResult::Ignored,
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        self.table.render(frame, area, theme);
    }

    fn keybindings(&self) -> &'static [Keybinding] {
        LABELS_KEYBINDINGS
    }
}
