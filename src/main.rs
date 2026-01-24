use std::sync::Arc;

use clap::Parser;

use crate::app::App;
use crate::config::KeyResolver;
use crate::registry::ServiceRegistry;

mod app;
mod cli;
mod config;
mod context;
mod provider;
mod registry;
mod search;
mod theme;
mod components;
pub mod tui;
pub mod service;
pub mod commands;

pub use theme::Theme;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let args = cli::Args::parse();

    let config = Arc::new(config::load()?);
    let resolver = Arc::new(KeyResolver::new(Arc::new(config.keybindings.clone())));
    let theme = theme::theme_from_name(&config.theme.name);

    let mut registry = ServiceRegistry::new();
    provider::register_all(&mut registry);

    let mut app = App::new(registry, config, resolver, theme);
    app.apply_cli_args(&args)?;
    app.run().await?;

    Ok(())
}
