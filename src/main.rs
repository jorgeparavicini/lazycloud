use crate::app::App;
use crate::config::KeyResolver;
use crate::registry::ServiceRegistry;
use clap::Parser;
use std::sync::Arc;

mod app;
mod cli;
mod component;
mod config;
mod core;
mod model;
mod provider;
mod registry;
mod search;
mod theme;
mod ui;

pub use theme::Theme;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let args = cli::Args::parse();

    // Load config (creates default if doesn't exist)
    let config = Arc::new(config::load()?);

    // Create key resolver
    let resolver = Arc::new(KeyResolver::new(Arc::new(config.keybindings.clone())));

    // Load theme from config
    let theme = theme::theme_from_name(&config.theme.name);

    let mut registry = ServiceRegistry::new();
    provider::register_all(&mut registry);

    let mut app = App::new(registry, config, resolver, theme);
    app.apply_cli_args(args)?;
    app.run().await?;

    Ok(())
}
