use crate::app::AppContext;
use crate::components::Component;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::widgets::Paragraph;

pub struct SecretManager {}

impl Component for SecretManager {
    fn render(&mut self, frame: &mut Frame, area: Rect) -> color_eyre::Result<()> {
        Ok(frame.render_widget(Paragraph::new("Secret Manager"), area))
    }
}

impl SecretManager {
    pub fn new(app_context: &AppContext) -> Self {
        SecretManager {}
    }
}
