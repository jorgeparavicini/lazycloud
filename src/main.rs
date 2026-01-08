use crate::app::App;
use crate::registry::ServiceRegistry;

mod app;
mod core;
mod model;
mod provider;
mod registry;
mod theme;
mod view;

pub use theme::{available_themes, Theme, ThemeInfo};

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let mut registry = ServiceRegistry::new();
    provider::register_all(&mut registry);

    let mut app = App::new(registry);
    app.run().await?;

    Ok(())
}
