use anyhow::Result;
use clap::Parser;

mod cli;
mod mcp;
mod server;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let cli = cli::Cli::parse();
    cli::dispatch(cli).await
}
