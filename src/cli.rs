use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "lazycloud", version, about = "TUI for managing cloud resources")]
pub struct Args {
    /// Context name (e.g., "default", "prod")
    #[arg(short, long)]
    pub context: Option<String>,

    /// Service name (e.g., "secret-manager")
    #[arg(short, long)]
    pub service: Option<String>,
}
