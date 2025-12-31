use crate::app::App;

mod action;
mod components;
mod tui;
mod app;
mod context;
mod widgets;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    let mut app = App::new();
    app.run().await?;
    Ok(())
}
