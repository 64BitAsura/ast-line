use anyhow::Result;
use clap::Args;
use colored::Colorize;
use std::path::PathBuf;

use crate::storage::repo_manager::get_storage_paths;

/// Show the indexing status of a repository.
#[derive(Args, Debug)]
pub struct StatusCommand {
    /// Path to the repository root. Defaults to the current working directory.
    #[arg(value_name = "PATH")]
    pub path: Option<PathBuf>,
}

pub async fn run(cmd: StatusCommand) -> Result<()> {
    let repo_path = match cmd.path {
        Some(p) => p.canonicalize()?,
        None => std::env::current_dir()?,
    };

    let paths = get_storage_paths(&repo_path);
    let meta_path = paths.gitnexus_dir.join("meta.json");

    if meta_path.exists() {
        let meta_raw = tokio::fs::read_to_string(&meta_path).await?;
        let meta: serde_json::Value = serde_json::from_str(&meta_raw)?;

        let commit = meta
            .get("lastCommit")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let indexed_at = meta
            .get("indexedAt")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        println!(
            "\n  {} {}\n",
            "GitNexus Status".bold().cyan(),
            repo_path.display().to_string().yellow()
        );
        println!("  {} {}", "Last indexed commit:".bold(), commit.dimmed());
        println!("  {} {}", "Indexed at:".bold(), indexed_at.dimmed());
        println!(
            "\n  Graph: {}",
            paths.gitnexus_dir.display().to_string().dimmed()
        );
    } else {
        println!(
            "\n  {} This repository has not been indexed yet.",
            "!".yellow().bold()
        );
        println!(
            "  Run {} to build the knowledge graph.",
            "gitnexus analyze".bold()
        );
    }

    Ok(())
}
