use std::sync::Arc;

use clap::Parser;
use color_eyre::Result;
use tracing::info;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::app::App;
use crate::config::KeyResolver;
use crate::registry::ServiceRegistry;

mod app;
mod cli;
pub mod commands;
mod config;
mod context;
mod provider;
mod registry;
mod search;
pub mod service;
mod theme;
pub mod tui;
mod ui;

pub use theme::Theme;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    let _guard = initialize_logging()?;
    info!("Starting lazycloud");

    let args = cli::Args::parse();

    let config = Arc::new(config::load()?);
    let resolver = Arc::new(KeyResolver::new(Arc::new(config.keybindings.clone())));
    let theme = theme::theme_from_name(&config.theme.name);

    let mut registry = ServiceRegistry::new();
    provider::register_all(&mut registry);

    let mut app = App::new(registry, config, resolver, theme)?;
    app.apply_cli_args(&args)?;
    app.run().await?;

    Ok(())
}

fn initialize_logging() -> Result<WorkerGuard> {
    let directory = dirs::data_local_dir().map_or_else(
        || std::path::PathBuf::from("logs"),
        |path| path.join("lazycloud").join("logs"),
    );
    std::fs::create_dir_all(&directory)?;

    let file_appender = tracing_appender::rolling::daily(&directory, "lazycloud.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(non_blocking)
                .with_ansi(false)
                .with_file(true)
                .with_line_number(true)
                .with_thread_ids(true),
        )
        .init();

    Ok(guard)
}
