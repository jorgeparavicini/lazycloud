//! In-memory log capture for the live log viewer.
//!
//! A [`LogBuffer`] is a bounded, shared ring buffer of recent log records.
//! [`LogBufferLayer`] is a [`tracing`] layer that feeds every emitted event
//! into the buffer, so the TUI can display logs live without reading the log
//! file back from disk.

use std::collections::VecDeque;
use std::fmt::Write as _;
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Local};
use tracing::field::{Field, Visit};
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::Layer;
use tracing_subscriber::layer::Context;

/// Maximum number of log entries retained in memory.
const MAX_ENTRIES: usize = 2000;

/// A single captured log record.
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp: DateTime<Local>,
    pub level: Level,
    pub target: String,
    pub message: String,
}

/// A bounded, thread-safe ring buffer of recent log records.
///
/// Cloning shares the same underlying buffer, so the logging layer and the
/// UI hold handles to the same data.
#[derive(Clone)]
pub struct LogBuffer {
    inner: Arc<Mutex<VecDeque<LogEntry>>>,
}

impl LogBuffer {
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(VecDeque::with_capacity(MAX_ENTRIES))),
        }
    }

    /// Append an entry, evicting the oldest once at capacity.
    fn push(&self, entry: LogEntry) {
        if let Ok(mut buf) = self.inner.lock() {
            if buf.len() == MAX_ENTRIES {
                buf.pop_front();
            }
            buf.push_back(entry);
        }
    }

    /// Snapshot the current entries (oldest first) for rendering.
    #[must_use]
    pub fn snapshot(&self) -> Vec<LogEntry> {
        self.inner
            .lock()
            .map(|buf| buf.iter().cloned().collect())
            .unwrap_or_default()
    }
}

impl Default for LogBuffer {
    fn default() -> Self {
        Self::new()
    }
}

/// Extracts the `message` field from a tracing event.
struct MessageVisitor {
    message: String,
}

impl Visit for MessageVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            let _ = write!(self.message, "{value:?}");
        } else {
            // Surface structured fields too, so they aren't silently lost.
            if !self.message.is_empty() {
                self.message.push(' ');
            }
            let _ = write!(self.message, "{}={value:?}", field.name());
        }
    }
}

/// A [`tracing`] layer that records every event into a [`LogBuffer`].
pub struct LogBufferLayer {
    buffer: LogBuffer,
}

impl LogBufferLayer {
    #[must_use]
    pub const fn new(buffer: LogBuffer) -> Self {
        Self { buffer }
    }
}

impl<S: Subscriber> Layer<S> for LogBufferLayer {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let mut visitor = MessageVisitor {
            message: String::new(),
        };
        event.record(&mut visitor);

        let metadata = event.metadata();
        self.buffer.push(LogEntry {
            timestamp: Local::now(),
            level: *metadata.level(),
            target: metadata.target().to_string(),
            message: visitor.message,
        });
    }
}
