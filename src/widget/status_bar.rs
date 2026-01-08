use crate::model::CloudContext;
use crate::Theme;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

pub struct StatusBar {
    active_context: Option<CloudContext>,
    error_message: Option<String>,
}

impl StatusBar {
    pub fn new() -> Self {
        Self {
            active_context: None,
            error_message: None,
        }
    }

    pub fn set_active_context(&mut self, context: CloudContext) {
        self.active_context = Some(context);
    }

    pub fn clear_context(&mut self) {
        self.active_context = None;
    }

    pub fn set_error(&mut self, message: String) {
        self.error_message = Some(message);
    }

    pub fn clear_error(&mut self) {
        self.error_message = None;
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {

        let context_name = match &self.active_context {
            Some(CloudContext::Gcp(gcp)) => format!("GCP: {}", gcp.project_id),
            Some(CloudContext::Aws(aws)) => format!("AWS: {}", aws.profile),
            Some(CloudContext::Azure(azure)) => format!("Azure: {}", azure.subscription_id),
            None => "None".to_string(),
        };

        let (status_text, style) = if let Some(err) = &self.error_message {
            (
                format!("Error: {} | Context: {}", err, context_name),
                Style::default().fg(theme.error()),
            )
        } else {
            (
                format!("Lazycloud | Context: {}", context_name),
                Style::default().fg(theme.subtext0()),
            )
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.surface1()))
            .title("Status")
            .title_style(Style::default().fg(theme.blue()).add_modifier(Modifier::BOLD));
        let paragraph = Paragraph::new(status_text).style(style).block(block);
        frame.render_widget(paragraph, area);
    }
}

impl Default for StatusBar {
    fn default() -> Self {
        Self::new()
    }
}
