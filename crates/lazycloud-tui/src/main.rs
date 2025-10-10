use color_eyre::Result;
use ratatui::crossterm::event;
use ratatui::crossterm::event::Event;
use ratatui::prelude::{Alignment, Stylize};
use ratatui::widgets::Paragraph;
use ratatui::{DefaultTerminal, Frame};

fn main() -> Result<()> {
    color_eyre::install()?;
    let terminal = ratatui::init();
    let result = run(terminal);
    ratatui::restore();
    result
}

fn run(mut terminal: DefaultTerminal) -> Result<()> {
    loop {
        terminal.draw(render)?;
        if matches!(event::read()?, Event::Key(_)) {
            break Ok(());
        }
    }
}

fn render(frame: &mut Frame) {
    frame.render_widget(
        Paragraph::new("Welcome to lazycloud")
            .dark_gray()
            .alignment(Alignment::Center),
        frame.area(),
    )
}
