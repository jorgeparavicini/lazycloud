use crate::provider::gcp::secret_manager::message::SecretManagerMsg;
use crate::provider::gcp::secret_manager::model::{Secret, SecretVersion};
use crate::provider::gcp::secret_manager::view::SecretManagerView;
use crate::search::Matcher;
use crate::view::{ColumnDef, Keybinding, KeyResult, TableEvent, TableRow, TableView, View};
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
        let matcher = Matcher::new();
        matcher.matches(&self.version_id, query) || matcher.matches(&self.state, query)
    }
}

const VERSION_LIST_KEYBINDINGS: &[Keybinding] = &[
    Keybinding::hint("Enter", "Payload"),
    Keybinding::hint("a", "Add version"),
    Keybinding::hint("/", "Search"),
    Keybinding::new("d", "Disable"),
    Keybinding::new("e", "Enable"),
    Keybinding::new("D", "Destroy"),
    Keybinding::new("r", "Reload"),
];

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

impl SecretManagerView for VersionListView {
    fn breadcrumbs(&self) -> Vec<String> {
        vec!["Versions".to_string()]
    }

    fn reload(&self) -> SecretManagerMsg {
        SecretManagerMsg::LoadVersions(self.secret.clone())
    }
}

impl View for VersionListView {
    type Event = SecretManagerMsg;

    fn handle_key(&mut self, key: KeyEvent) -> KeyResult<Self::Event> {
        // Delegate to table first (handles search mode, navigation, etc.)
        let result = self.table.handle_key(key);
        if let KeyResult::Event(TableEvent::Activated(version)) = result {
            return SecretManagerMsg::LoadPayload(self.secret.clone(), Some(version)).into();
        }
        if result.is_consumed() {
            return KeyResult::Consumed;
        }

        // Handle local shortcuts only if table didn't consume the key
        match key.code {
            KeyCode::Char('r') => SecretManagerMsg::ReloadData.into(),
            // Add new version
            KeyCode::Char('a') => {
                SecretManagerMsg::ShowCreateVersionDialog(self.secret.clone()).into()
            }
            // Disable version (only for Enabled versions)
            KeyCode::Char('d') => match self.table.selected_item() {
                Some(v) if v.state.contains("Enabled") => SecretManagerMsg::DisableVersion {
                    secret: self.secret.clone(),
                    version: v.clone(),
                }
                .into(),
                _ => KeyResult::Ignored,
            },
            // Enable version (only for Disabled versions)
            KeyCode::Char('e') => match self.table.selected_item() {
                Some(v) if v.state.contains("Disabled") => SecretManagerMsg::EnableVersion {
                    secret: self.secret.clone(),
                    version: v.clone(),
                }
                .into(),
                _ => KeyResult::Ignored,
            },
            // Destroy version (shift+D, only for non-Destroyed versions)
            KeyCode::Char('D') => match self.table.selected_item() {
                Some(v) if !v.state.contains("Destroyed") => SecretManagerMsg::ShowDestroyVersionDialog {
                    secret: self.secret.clone(),
                    version: v.clone(),
                }
                .into(),
                _ => KeyResult::Ignored,
            },
            _ => KeyResult::Ignored,
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        self.table.render(frame, area, theme);
    }

    fn keybindings(&self) -> &'static [Keybinding] {
        VERSION_LIST_KEYBINDINGS
    }
}
