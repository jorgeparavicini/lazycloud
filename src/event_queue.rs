use color_eyre::Result;
use tokio::sync::mpsc;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::commands::Command;
use crate::service::ServiceMsg;

pub struct EventQueue<Event> {
    tx: UnboundedSender<Event>,
    rx: UnboundedReceiver<Event>,
}

impl<Event> EventQueue<Event> {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        Self { tx, rx }
    }

    pub fn send(&self, event: Event) {
        let _ = self.tx.send(event);
    }

    pub fn clone_sender(&self) -> UnboundedSender<Event> {
        self.tx.clone()
    }

    pub fn drain(&mut self) -> Vec<Event> {
        let mut events = Vec::new();
        while let Ok(event) = self.rx.try_recv() {
            events.push(event);
        }
        events
    }

    /// Drain and process events in a loop until no more messages are pending.
    ///
    /// This loops because processing an event may enqueue new events (e.g.
    /// `ClientInitialized` queues `SecretsMsg::Load`). A single drain would
    /// miss those since `drain()` snapshots before processing starts.
    pub fn process_events<F>(events: Vec<Event>, mut process_fn: F) -> Result<ServiceMsg>
    where
        F: FnMut(Event) -> Result<ServiceMsg>,
    {
        let mut commands: Vec<Box<dyn Command>> = Vec::new();

        for event in events {
            match process_fn(event)? {
                ServiceMsg::Idle => {}
                ServiceMsg::Run(cmds) => commands.extend(cmds),
                ServiceMsg::Close => return Ok(ServiceMsg::Close),
                msg @ ServiceMsg::EditExternal { .. } => return Ok(msg),
            }
        }

        if commands.is_empty() {
            Ok(ServiceMsg::Idle)
        } else {
            Ok(ServiceMsg::Run(commands))
        }
    }
}
