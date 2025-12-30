use crate::app::AppContext;
use crate::context::GcpContext;
use crate::{action::Action, components::Component};
use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};
use tokio::sync::mpsc::UnboundedSender;
use crate::components::ComponentResult;
use crate::components::ComponentResult::Ignored;
use crate::context::Context::Gcp;

pub struct ContextSelector {
    contexts: Vec<String>,
    state: ListState,
    action_tx: UnboundedSender<Action>,
}

impl ContextSelector {
    pub fn new(app_context: &AppContext) -> Self {
        let mut state = ListState::default();
        state.select(Some(0));
        Self {
            contexts: vec![
                "GCP - Dev".to_string(),
                "AWS - Prod".to_string(),
                "Azure - Test".to_string(),
            ],
            state,
            action_tx: app_context.action_tx.clone(),
        }
    }

    fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.contexts.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.contexts.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    fn select_context(&mut self) -> Result<()> {
        if let Some(i) = self.state.selected() {
            let context = self.contexts[i].clone();
            self.action_tx
                .send(Action::SelectContext(Gcp(GcpContext {
                    project_id: context,
                    service_account_path: "".to_string(),
                    zone: "".to_string(),
                })))?;
        }
        Ok(())
    }
}

impl Component for ContextSelector {
    fn handle_key_event(&mut self, key: KeyEvent) -> Result<ComponentResult> {
        match key.code {
            KeyCode::Down => self.previous(),
            KeyCode::Up => self.next(),
            KeyCode::Enter => self.select_context()?,
            _ => {}
        }
        Ok(Ignored)
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        let items: Vec<ListItem> = self
            .contexts
            .iter()
            .map(|i| ListItem::new(i.clone()))
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Select Context"),
            )
            .highlight_style(
                Style::default()
                    .bg(Color::LightGreen)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(">> ");

        frame.render_stateful_widget(list, area, &mut self.state);
        Ok(())
    }
}
