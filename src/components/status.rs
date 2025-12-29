use crate::components::Component;
use color_eyre::Result;
use ratatui::{
    Frame,
    layout::Rect,
    widgets::{Block, Borders, Paragraph},
};

pub struct Status {}

impl Status {
    pub fn new() -> Self {
        Self {}
    }
}

impl Component for Status {
    fn render(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        let block = Block::default().borders(Borders::ALL).title("Status");
        let paragraph = Paragraph::new("Lazycloud - Status OK | Context: None").block(block);
        frame.render_widget(paragraph, area);
        Ok(())
    }
}
