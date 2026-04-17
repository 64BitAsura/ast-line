use anyhow::Result;
use clap::{Parser, Subcommand};

pub mod analyze;
pub mod clean;
pub mod list;
pub mod query;
pub mod serve;
pub mod status;

#[derive(Parser, Debug)]
#[command(name = "ast-line", about = "Graph-powered code intelligence", version = "0.1.0")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Index a repository and build its knowledge graph
    Analyze(analyze::AnalyzeCommand),
    /// Start the HTTP API server
    Serve(serve::ServeCommand),
    /// Start the MCP stdio server
    Mcp,
    /// List all indexed repositories
    List,
    /// Show index status for a repository
    Status(status::StatusCommand),
    /// Delete the .gitnexus index for a repository
    Clean(clean::CleanCommand),
    /// Run a BM25 search query
    Query(query::QueryCommand),
}

pub async fn dispatch(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Analyze(cmd) => analyze::run(cmd).await,
        Commands::Serve(cmd) => serve::run(cmd).await,
        Commands::Mcp => crate::mcp::start_mcp_server().await,
        Commands::List => list::run().await,
        Commands::Status(cmd) => status::run(cmd).await,
        Commands::Clean(cmd) => clean::run(cmd).await,
        Commands::Query(cmd) => query::run(cmd).await,
    }
}
