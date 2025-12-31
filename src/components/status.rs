use crate::action::Action;
use crate::components::Component;
use crate::context::Context;
use color_eyre::Result;
use ratatui::{
    layout::Rect,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub struct Status {
    active_context: Option<Context>,
}

impl Status {
    pub fn new() -> Self {
        Self {
            active_context: None,
        }
    }
}

impl Component for Status {
    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        if let Action::SelectContext(context) = action {
            self.active_context = Some(context);
        }
        Ok(None)
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let context_name = match &self.active_context {
            Some(Context::Gcp(gcp)) => format!("GCP: {}", gcp.project_id),
            Some(Context::Aws(aws)) => format!("AWS: {}", aws.profile),
            Some(Context::Azure(azure)) => format!("Azure: {}", azure.subscription_id),
            None => "None".to_string(),
        };
        let block = Block::default().borders(Borders::ALL).title("Status");
        let paragraph =
            Paragraph::new(format!("Lazycloud - Status OK | Context: {}", context_name))
                .block(block);
        frame.render_widget(paragraph, area);
    }
}
