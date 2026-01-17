use crate::model::CloudContext;
use crate::ui::Component;
use crate::Theme;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

pub struct StatusBarView {
    active_context: Option<CloudContext>,
}

impl StatusBarView {
    pub fn new() -> Self {
        Self {
            active_context: None,
        }
    }

    pub fn set_active_context(&mut self, context: CloudContext) {
        self.active_context = Some(context);
    }

    pub fn clear_context(&mut self) {
        self.active_context = None;
    }
}

impl Default for StatusBarView {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for StatusBarView {
    type Output = ();

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let context_name = match &self.active_context {
            Some(CloudContext::Gcp(gcp)) => format!("GCP: {}", gcp.project_id),
            Some(CloudContext::Aws(aws)) => format!("AWS: {}", aws.profile),
            Some(CloudContext::Azure(azure)) => format!("Azure: {}", azure.subscription_id),
            None => "None".to_string(),
        };

        let status_text = format!("Lazycloud | Context: {}", context_name);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.surface1()))
            .title(" Status ")
            .title_style(Style::default().fg(theme.blue()).add_modifier(Modifier::BOLD));
        let paragraph = Paragraph::new(status_text)
            .style(Style::default().fg(theme.subtext0()))
            .block(block);
        frame.render_widget(paragraph, area);
    }
}
