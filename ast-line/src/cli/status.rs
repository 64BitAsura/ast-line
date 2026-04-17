use anyhow::Result;
use clap::Args;
use std::path::PathBuf;

use ast_line_core::storage::repo_manager::get_storage_paths;

#[derive(Args, Debug)]
pub struct StatusCommand {
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
        println!("Indexed at: {}", meta.get("indexedAt").and_then(|v| v.as_str()).unwrap_or("unknown"));
        println!("Nodes: {}", meta.get("nodeCount").and_then(|v| v.as_u64()).unwrap_or(0));
        println!("Relationships: {}", meta.get("relationshipCount").and_then(|v| v.as_u64()).unwrap_or(0));
    } else {
        println!("This repository has not been indexed yet.");
        println!("Run `ast-line analyze` to build the knowledge graph.");
    }

    Ok(())
}
