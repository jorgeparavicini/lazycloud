use crate::Theme;
use ratatui::layout::{Constraint, Rect};
use ratatui::style::Style;
use ratatui::Frame;
use throbber_widgets_tui::WhichUse::Spin;
use throbber_widgets_tui::{Throbber, ThrobberState, BRAILLE_SIX};

pub struct Spinner {
    throbber_state: ThrobberState,
    label: Option<&'static str>,
}

impl Spinner {
    pub fn new() -> Self {
        Self {
            throbber_state: ThrobberState::default(),
            label: None,
        }
    }

    pub fn set_label(&mut self, label: &'static str) {
        self.label = Some(label);
    }

    pub fn on_tick(&mut self) {
        self.throbber_state.calc_next()
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {

        let mut throbber = Throbber::default()
            .throbber_set(BRAILLE_SIX)
            .use_type(Spin)
            .throbber_style(Style::default().fg(theme.lavender()))
            .style(Style::default().fg(theme.subtext1()));

        // The throbber itself uses 1-character width
        let mut width = 1u16;

        if let Some(label) = self.label {
            throbber = throbber.label(label);
            width += label.len() as u16 + 1; // +1 for space between throbber and label
        }

        let area = area.centered(Constraint::Length(width), Constraint::Length(1));

        frame.render_stateful_widget(throbber, area, &mut self.throbber_state)
    }
}
