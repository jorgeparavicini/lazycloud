use crate::action::AppMsg;
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
    
    pub fn set_active_context(&mut self, context: Context) {
        self.active_context = Some(context);
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
