use futures::FutureExt;
use crossterm::event::{EventStream, Event as CrosstermEvent, KeyEvent, KeyEventKind, MouseEvent, EnableMouseCapture, EnableBracketedPaste, DisableBracketedPaste, DisableMouseCapture};
use futures::StreamExt;
use ratatui::backend::CrosstermBackend;
use ratatui::{Terminal};
use std::io::Stdout;
use std::ops::{Deref, DerefMut};
use std::time::Duration;
use crossterm::cursor;
use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen};
use tokio::sync::mpsc;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::task::JoinHandle;
use tokio::time::interval;
use tokio_util::sync::CancellationToken;

const GRACEFUL_SHUTDOWN_TIMEOUT_MS: u64 = 500;
const FORCEFUL_SHUTDOWN_TIMEOUT_MS: u64 = 2000;

pub type Backend = CrosstermBackend<Stdout>;

#[derive(Clone, Debug)]
pub enum Event {
    Init,
    Quit,
    Error(String),
    Closed,
    Tick,
    Render,
    FocusGained,
    FocusLost,
    Paste(String),
    Key(KeyEvent),
    Mouse(MouseEvent),
    Resize(u16, u16),
}

pub struct Tui {
    terminal: Terminal<Backend>,
    task: JoinHandle<()>,
    cancellation_token: CancellationToken,
    event_rx: UnboundedReceiver<Event>,
    event_tx: UnboundedSender<Event>,
    frame_rate: f64,
    tick_rate: f64,
}

impl Tui {
    pub fn new(frame_rate: f64, tick_rate: f64) -> color_eyre::Result<Self> {
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        Ok(Self {
            terminal: Terminal::new(Backend::new(std::io::stdout()))?,
            task: tokio::spawn(async {}),
            cancellation_token: CancellationToken::new(),
            event_rx,
            event_tx,
            frame_rate,
            tick_rate,
        })
    }

    pub fn enter(&mut self) -> color_eyre::Result<()> {
        crossterm::terminal::enable_raw_mode()?;
        crossterm::execute!(std::io::stdout(), EnterAlternateScreen, cursor::Hide)?;
        crossterm::execute!(std::io::stdout(), EnableMouseCapture)?;
        crossterm::execute!(std::io::stdout(), EnableBracketedPaste)?;
        self.start();
        Ok(())
    }

    pub fn exit(&mut self) -> color_eyre::Result<()> {
        self.stop()?;
        if crossterm::terminal::is_raw_mode_enabled()? {
            self.flush()?;
            crossterm::execute!(std::io::stdout(), DisableBracketedPaste)?;
            crossterm::execute!(std::io::stdout(), DisableMouseCapture)?;
            crossterm::execute!(std::io::stdout(), LeaveAlternateScreen, cursor::Show)?;
            crossterm::terminal::disable_raw_mode()?;
        }
        Ok(())
    }

    pub fn suspend(&mut self) -> color_eyre::Result<()> {
        self.exit()?;
        #[cfg(not(windows))]
        signal_hook::low_level::raise(signal_hook::consts::SIGTSTP)?;
        Ok(())
    }

    pub fn resume(&mut self) -> color_eyre::Result<()> {
        self.enter()?;
        Ok(())
    }

    pub async fn next_event(&mut self) -> Option<Event> {
        self.event_rx.recv().await
    }

    fn start(&mut self) {
        self.cancel();
        self.cancellation_token = CancellationToken::new();
        let event_loop = Self::event_loop(
            self.event_tx.clone(),
            self.cancellation_token.clone(),
            self.tick_rate,
            self.frame_rate,
        );
        self.task = tokio::spawn(event_loop);
    }

    fn stop(&mut self) -> color_eyre::Result<()> {
        self.cancel();
        let mut shutdown_counter = 0;
        while !self.task.is_finished() {
            std::thread::sleep(Duration::from_millis(1));
            shutdown_counter += 1;
            if shutdown_counter >= GRACEFUL_SHUTDOWN_TIMEOUT_MS {
                self.task.abort();
            }
            if shutdown_counter >= FORCEFUL_SHUTDOWN_TIMEOUT_MS {
                return Err(color_eyre::eyre::eyre!("Failed to stop TUI task"));
            }
        }
        Ok(())
    }

    fn cancel(&self) {
        self.cancellation_token.cancel();
    }

    async fn event_loop(
        event_tx: UnboundedSender<Event>,
        cancellation_token: CancellationToken,
        tick_rate: f64,
        frame_rate: f64,
    ) {
        let mut event_stream = EventStream::new();
        let mut tick_interval = interval(Duration::from_secs_f64(1.0 / tick_rate));
        let mut frame_interval = interval(Duration::from_secs_f64(1.0 / frame_rate));

        event_tx.send(Event::Init).expect("Failed to send init event");

        loop {
            let event = tokio::select! {
                _ = cancellation_token.cancelled() => {
                    break;
                }
                _ = tick_interval.tick() => Event::Tick,
                _ = frame_interval.tick() => Event::Render,
                crossterm_event = event_stream.next().fuse() => {
                    match crossterm_event {
                        Some(Ok(event)) => match event {
                            CrosstermEvent::Key(key) => {
                                if key.kind == KeyEventKind::Press {
                                    Event::Key(key)
                                } else {
                                    continue;
                                }
                            },
                            CrosstermEvent::Mouse(mouse) => Event::Mouse(mouse),
                            CrosstermEvent::Resize(width, height) => Event::Resize(width, height),
                            CrosstermEvent::FocusGained => Event::FocusGained,
                            CrosstermEvent::FocusLost => Event::FocusLost,
                            CrosstermEvent::Paste(paste) => Event::Paste(paste),
                        },
                        Some(Err(e)) => Event::Error(e.to_string()),
                        None => break
                    }
                }
            };
            if event_tx.send(event).is_err() {
                break;
            }
        }
        cancellation_token.cancel();
    }
}

impl Deref for Tui {
    type Target = Terminal<Backend>;

    fn deref(&self) -> &Self::Target {
        &self.terminal
    }
}

impl DerefMut for Tui {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.terminal
    }
}

impl Drop for Tui {
    fn drop(&mut self) {
        self.exit().unwrap();
    }
}
