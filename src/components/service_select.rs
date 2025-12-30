use crate::app::AppContext;
use crate::components::services::{GcpService, Service};
use crate::{action::Action, components::Component};
use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState},
};
use tokio::sync::mpsc::UnboundedSender;
use crate::components::ComponentResult;
use crate::components::ComponentResult::Ignored;

pub struct ServiceSelector {
    services: Vec<Service>,
    state: ListState,
    action_tx: UnboundedSender<Action>,
}

impl ServiceSelector {
    pub fn new(app_context: &AppContext) -> Self {
        let mut state = ListState::default();
        state.select(Some(0));
        Self {
            services: vec![Service::Gcp(GcpService::SecretManager)],
            state,
            action_tx: app_context.action_tx.clone(),
        }
    }

    fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.services.len() - 1 {
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
                    self.services.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    fn selected_service(&self) -> Result<()> {
        if let Some(i) = self.state.selected() {
            let service = &self.services[i];
            self.action_tx
                .send(Action::SelectService(service.clone()))?;
        }
        Ok(())
    }
}

impl Component for ServiceSelector {
    fn handle_key_event(&mut self, key: KeyEvent) -> Result<ComponentResult> {
        match key.code {
            KeyCode::Down => self.previous(),
            KeyCode::Up => self.next(),
            KeyCode::Enter => self.selected_service()?,
            _ => {}
        }

        Ok(Ignored)
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        let items: Vec<ListItem> = self
            .services
            .iter()
            .map(|i| ListItem::new(i.name()))
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Select Service"),
            )
            .highlight_style(
                Style::default()
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(">> ");

        frame.render_stateful_widget(list, area, &mut self.state);
        Ok(())
    }
}
