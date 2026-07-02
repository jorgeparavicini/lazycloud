use std::sync::Arc;

use clap::Parser;
use color_eyre::Result;
use tracing::info;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::Layer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use crate::app::App;
use crate::logging::{LogBuffer, LogBufferLayer};
use crate::registry::ServiceRegistry;

mod app;
mod cache;
mod cli;
pub mod commands;
mod config;
mod context;
mod event_queue;
mod logging;
mod provider;
mod registry;
mod search;
pub mod service;
mod theme;
pub mod tui;
mod ui;
pub mod utility;
mod view_stack;

pub use theme::Theme;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    let (log_buffer, _guard) = initialize_logging()?;
    info!("Starting lazycloud");

    let args = cli::Args::parse();

    let config = Arc::new(config::load()?);
    let theme = theme::theme_from_name(&config.theme.name);

    let mut registry = ServiceRegistry::new();
    provider::register_all(&mut registry);

    let mut app = App::new(registry, config, theme, log_buffer);
    app.apply_cli_args(&args)?;
    app.run().await?;

    Ok(())
}

fn initialize_logging() -> Result<(LogBuffer, WorkerGuard)> {
    let directory = dirs::data_local_dir().map_or_else(
        || std::path::PathBuf::from("logs"),
        |path| path.join("lazycloud").join("logs"),
    );
    std::fs::create_dir_all(&directory)?;

    let file_appender = tracing_appender::rolling::daily(&directory, "lazycloud.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    // Per-layer filters: the file mirrors RUST_LOG (unchanged behavior), while
    // the in-app buffer falls back to `info` so the live log view is useful
    // out of the box even without RUST_LOG set.
    let file_filter = tracing_subscriber::EnvFilter::from_default_env();
    let buffer_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

    let log_buffer = LogBuffer::new();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(non_blocking)
                .with_ansi(false)
                .with_file(true)
                .with_line_number(true)
                .with_thread_ids(true)
                .with_filter(file_filter),
        )
        .with(LogBufferLayer::new(log_buffer.clone()).with_filter(buffer_filter))
        .init();

    Ok((log_buffer, guard))
}
