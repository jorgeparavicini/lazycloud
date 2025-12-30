use crate::app::App;

mod action;
mod components;
mod tui;
mod app;
mod context;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    let mut app = App::new();
    app.run().await?;
    Ok(())
}
