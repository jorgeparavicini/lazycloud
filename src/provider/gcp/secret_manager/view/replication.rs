use crate::provider::gcp::secret_manager::message::SecretManagerMsg;
use crate::provider::gcp::secret_manager::model::{ReplicationConfig, Secret};
use crate::provider::gcp::secret_manager::view::SecretManagerView;
use crate::view::{Keybinding, KeyResult, View};
use crate::Theme;

const REPLICATION_KEYBINDINGS: &[Keybinding] = &[
    Keybinding::new("r", "Reload"),
];
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};
use ratatui::Frame;

pub struct ReplicationView {
    secret: Secret,
    replication: ReplicationConfig,
}

impl ReplicationView {
    pub fn new(secret: Secret, replication: ReplicationConfig) -> Self {
        Self {
            secret,
            replication,
        }
    }
}

impl SecretManagerView for ReplicationView {
    fn breadcrumbs(&self) -> Vec<String> {
        vec!["Replication".to_string()]
    }

    fn reload(&self) -> SecretManagerMsg {
        SecretManagerMsg::ShowReplicationInfo(self.secret.clone())
    }
}

impl View for ReplicationView {
    type Event = SecretManagerMsg;

    fn handle_key(&mut self, key: KeyEvent) -> KeyResult<Self::Event> {
        match key.code {
            KeyCode::Char('r') => SecretManagerMsg::ReloadData.into(),
            _ => KeyResult::Ignored,
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let title = format!(" {} - Replication ", self.secret.name);

        let label_style = Style::default()
            .fg(theme.subtext0())
            .add_modifier(Modifier::BOLD);
        let value_style = Style::default().fg(theme.text());
        let location_style = Style::default().fg(theme.green());

        let lines = match &self.replication {
            ReplicationConfig::Automatic => {
                vec![
                    Line::from(""),
                    Line::from(vec![
                        Span::styled("Type: ", label_style),
                        Span::styled("Automatic", value_style),
                    ]),
                    Line::from(""),
                    Line::from(Span::styled(
                        "Secret is automatically replicated across all GCP regions.",
                        Style::default().fg(theme.overlay1()),
                    )),
                ]
            }
            ReplicationConfig::UserManaged { locations } => {
                let mut lines = vec![
                    Line::from(""),
                    Line::from(vec![
                        Span::styled("Type: ", label_style),
                        Span::styled("User-Managed", value_style),
                    ]),
                    Line::from(""),
                    Line::from(Span::styled("Locations:", label_style)),
                ];

                for location in locations {
                    lines.push(Line::from(vec![
                        Span::raw("  - "),
                        Span::styled(location.clone(), location_style),
                    ]));
                }

                if locations.is_empty() {
                    lines.push(Line::from(Span::styled(
                        "  (no locations configured)",
                        Style::default().fg(theme.overlay1()),
                    )));
                }

                lines
            }
        };

        let block = Block::default()
            .title(title)
            .title_style(
                Style::default()
                    .fg(theme.mauve())
                    .add_modifier(Modifier::BOLD),
            )
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.surface1()))
            .style(Style::default().bg(theme.base()));

        let paragraph = Paragraph::new(lines).block(block);

        frame.render_widget(paragraph, area);
    }

    fn keybindings(&self) -> &'static [Keybinding] {
        REPLICATION_KEYBINDINGS
    }
}
