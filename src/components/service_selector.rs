use crate::app::AppContext;
use crate::components::services::{GcpService, Service};
use crate::{action::Action, components::Component};
use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::Rect,
};
use ratatui::prelude::{Color, Modifier, Style};
use ratatui::widgets::{List, ListItem, ListState};
use tokio::sync::mpsc::UnboundedSender;
use crate::components::EventResult;

pub struct ServiceSelector {
    services: Vec<Service>,
    state: ListState,
    action_tx: UnboundedSender<Action>
}

impl ServiceSelector {
    pub fn new(app_context: &AppContext) -> Self {
        let action_tx = app_context.action_tx.clone();
        Self {
            services: vec![
                Service::Gcp(GcpService::SecretManager)
            ],
            state: ListState::default(),
            action_tx
        }
    }
    
    fn select_service(&self, index: usize) -> Result<()> {
        if let Some(service) = self.services.get(index) {
            self.action_tx.send(Action::SelectService(service.clone()))?;
        }
        Ok(())
    }
}

impl Component for ServiceSelector {
    fn handle_key_event(&mut self, key: KeyEvent) -> Result<EventResult> {
        match key.code {
            KeyCode::Down => {
                self.state.select_next();
                Ok(EventResult::Consumed(None))
            }
            KeyCode::Up => {
                self.state.select_previous();
                Ok(EventResult::Consumed(None))
            }
            KeyCode::Enter => {
                if let Some(selected) = self.state.selected() {
                    self.select_service(selected)?;
                    Ok(EventResult::Consumed(None))
                } else {
                    Ok(EventResult::Ignored)
                }
            }
            _ => Ok(EventResult::Ignored),
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let items = self
            .services
            .iter()
            .map(|i| ListItem::new(i.name()))
            .collect::<Vec<ListItem>>();

        let list = List::new(items)
            .highlight_style(
                Style::default()
                    .bg(Color::Blue)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(">> ");

        frame.render_stateful_widget(list, area, &mut self.state)
    }

    fn breadcrumbs(&self) -> Vec<String> {
        vec!["Services".to_string()]
    }
}
