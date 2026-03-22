use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::Frame;

use crate::event_queue::EventQueue;
use crate::ui::{Component, EventResult, EventResultExt, Keybinding, Modal, Screen, Spinner};
use crate::Theme;

pub struct ViewStack<M: 'static> {
    screens: Vec<Box<dyn Screen<Output = M>>>,
    modal: Option<Box<dyn Modal<Output = M>>>,
    spinner: Spinner,
    loading: Option<&'static str>,
}

impl<M> ViewStack<M> {
    pub fn new() -> Self {
        Self {
            screens: Vec::new(),
            modal: None,
            spinner: Spinner::new(),
            loading: None,
        }
    }

    pub fn push(&mut self, screen: impl Screen<Output = M> + 'static) {
        self.clear_loading();
        self.screens.push(Box::new(screen));
    }

    pub fn pop(&mut self) -> bool {
        if self.screens.len() > 1 {
            self.screens.pop();
            true
        } else {
            false
        }
    }

    pub fn pop_to_root(&mut self) {
        while self.screens.len() > 1 {
            self.screens.pop();
        }
        self.screens.clear();
    }

    pub fn current_screen(&self) -> Option<&dyn Screen<Output = M>> {
        self.screens.last().map(|b| &**b)
    }

    pub fn current_screen_mut(&mut self) -> Option<&mut Box<dyn Screen<Output = M>>> {
        self.screens.last_mut()
    }

    pub fn show_modal(&mut self, modal: impl Modal<Output = M> + 'static) {
        self.modal = Some(Box::new(modal));
    }

    pub fn close_modal(&mut self) {
        self.modal = None;
    }

    pub const fn set_loading(&mut self, label: &'static str) {
        self.loading = Some(label);
    }

    pub const fn clear_loading(&mut self) {
        self.loading = None;
    }

    pub const fn is_loading(&self) -> bool {
        self.loading.is_some()
    }

    pub fn has_screens(&self) -> bool {
        !self.screens.is_empty()
    }

    pub fn handle_tick(&mut self) {
        if self.loading.is_some() {
            self.spinner.handle_tick();
        }
    }

    pub fn handle_key(
        &mut self,
        key: KeyEvent,
        event_queue: &EventQueue<M>,
        navigate_back: M,
    ) -> EventResult<()> {
        if self.loading.is_some() {
            return EventResult::Ignored;
        }

        // Handle modal first if present (captures all input)
        if let Some(modal) = &mut self.modal {
            let (consumed, msg) = modal.handle_key(key).process();
            if let Some(msg) = msg {
                event_queue.send(msg);
            }
            if consumed {
                return EventResult::Consumed;
            }
        }

        // Handle current screen
        if let Some(screen) = self.current_screen_mut() {
            let (consumed, msg) = screen.handle_key(key).process();
            if let Some(msg) = msg {
                event_queue.send(msg);
            }
            if consumed {
                return EventResult::Consumed;
            }
        }

        // Global navigation
        if key.code == KeyCode::Esc {
            event_queue.send(navigate_back);
            return EventResult::Consumed;
        }

        EventResult::Ignored
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        if let Some(label) = self.loading {
            self.spinner.set_label(label);
            self.spinner.render(frame, area, theme);
        } else if let Some(screen) = self.current_screen_mut() {
            screen.render(frame, area, theme);
        }

        // Render modal on top if present
        if let Some(modal) = &mut self.modal {
            modal.render(frame, area, theme);
        }
    }

    pub fn breadcrumbs(&self, service_name: &str) -> Vec<String> {
        let mut bc = vec![service_name.to_string()];
        for screen in &self.screens {
            bc.extend(screen.breadcrumbs());
        }
        bc
    }

    pub fn keybindings(&self) -> Vec<Keybinding> {
        self.current_screen()
            .map(Screen::keybindings)
            .unwrap_or_default()
    }
}
