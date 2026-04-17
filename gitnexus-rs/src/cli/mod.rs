mod analyze;
mod status;

pub use analyze::AnalyzeCommand;
pub use status::StatusCommand;

use anyhow::Result;
use clap::{Parser, Subcommand};

/// GitNexus — graph-powered code intelligence for AI agents.
///
/// Index any codebase into a knowledge graph, then query via CLI or MCP.
#[derive(Parser, Debug)]
#[command(
    name = "gitnexus",
    version,
    about,
    long_about = None,
    propagate_version = true
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Index a repository and build its knowledge graph.
    Analyze(AnalyzeCommand),
    /// Show the indexing status of a repository.
    Status(StatusCommand),
}

pub async fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Analyze(cmd) => analyze::run(cmd).await,
        Commands::Status(cmd) => status::run(cmd).await,
    }
}
