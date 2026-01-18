use crate::provider::gcp::secret_manager::message::SecretManagerMsg;
use crate::provider::gcp::secret_manager::model::{IamBinding, IamPolicy, Secret};
use crate::provider::gcp::secret_manager::view::SecretManagerView;
use crate::search::Matcher;
use crate::view::{ColumnDef, Keybinding, KeyResult, TableRow, TableView, View};
use crate::Theme;

const IAM_POLICY_KEYBINDINGS: &[Keybinding] = &[
    Keybinding::hint("/", "Search"),
    Keybinding::new("r", "Reload"),
];
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Rect};
use ratatui::widgets::Cell;
use ratatui::Frame;

impl TableRow for IamBinding {
    fn columns() -> &'static [ColumnDef] {
        static COLUMNS: &[ColumnDef] = &[
            ColumnDef::new("Role", Constraint::Min(30)),
            ColumnDef::new("Members", Constraint::Min(40)),
        ];
        COLUMNS
    }

    fn render_cells(&self, _theme: &Theme) -> Vec<Cell<'static>> {
        // Format members as comma-separated list, truncated if too long
        let members_str = if self.members.is_empty() {
            "(none)".to_string()
        } else if self.members.len() <= 3 {
            self.members.join(", ")
        } else {
            format!(
                "{}, ... (+{} more)",
                self.members[..2].join(", "),
                self.members.len() - 2
            )
        };

        vec![Cell::from(self.role.clone()), Cell::from(members_str)]
    }

    fn matches(&self, query: &str) -> bool {
        let matcher = Matcher::new();
        matcher.matches(&self.role, query)
            || self.members.iter().any(|m| matcher.matches(m, query))
    }
}

pub struct IamPolicyView {
    secret: Secret,
    table: TableView<IamBinding>,
}

impl IamPolicyView {
    pub fn new(secret: Secret, policy: IamPolicy) -> Self {
        let title = format!(" {} - IAM Policy ", secret.name);
        Self {
            secret,
            table: TableView::new(policy.bindings).with_title(title),
        }
    }
}

impl SecretManagerView for IamPolicyView {
    fn breadcrumbs(&self) -> Vec<String> {
        vec!["IAM Policy".to_string()]
    }

    fn reload(&self) -> SecretManagerMsg {
        SecretManagerMsg::ShowIamPolicy(self.secret.clone())
    }
}

impl View for IamPolicyView {
    type Event = SecretManagerMsg;

    fn handle_key(&mut self, key: KeyEvent) -> KeyResult<Self::Event> {
        // Delegate to table first (handles search mode, navigation, etc.)
        let result = self.table.handle_key(key);
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

    fn keybindings(&self) -> &'static [Keybinding] {
        IAM_POLICY_KEYBINDINGS
    }
}
