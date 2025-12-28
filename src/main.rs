use crate::app::App;

mod command;
mod components;
mod tui;
mod app;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    let mut app = App::new();
    app.run().await?;
    Ok(())
}
