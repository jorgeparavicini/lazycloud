use crate::app::AppContext;
use crate::components::EventResult;
use crate::context::{get_available_contexts, Context};
use crate::{action::Action, components::Component};
use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{List, ListItem, ListState};
use ratatui::{
    layout::Rect,
    Frame,
};
use tokio::sync::mpsc::UnboundedSender;

pub struct ContextSelector {
    contexts: Vec<Context>,
    state: ListState,
    action_tx: UnboundedSender<Action>,
}

impl ContextSelector {
    pub fn new(app_context: &AppContext) -> Self {
        let action_tx = app_context.action_tx.clone();

        Self {
            contexts: get_available_contexts(),
            state: ListState::default(),
            action_tx,
        }
    }

    fn select_context(&self, index: usize) -> Result<()> {
        if let Some(context) = self.contexts.get(index) {
            self.action_tx.send(Action::SelectContext(context.clone()))?;
        }
        Ok(())
    }
}

impl Component for ContextSelector {
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
                    self.select_context(selected)?;
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
            .contexts
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
        vec!["Context".to_string()]
    }
}
