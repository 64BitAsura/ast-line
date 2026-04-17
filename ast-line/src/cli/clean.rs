use anyhow::Result;
use clap::Args;
use std::path::PathBuf;

use ast_line_core::storage::repo_manager::get_storage_paths;

#[derive(Args, Debug)]
pub struct CleanCommand {
    #[arg(value_name = "PATH")]
    pub path: Option<PathBuf>,
}

pub async fn run(cmd: CleanCommand) -> Result<()> {
    let repo_path = match cmd.path {
        Some(p) => p.canonicalize()?,
        None => std::env::current_dir()?,
    };
    let paths = get_storage_paths(&repo_path);
    if paths.gitnexus_dir.exists() {
        tokio::fs::remove_dir_all(&paths.gitnexus_dir).await?;
        println!("Removed {}", paths.gitnexus_dir.display());
    } else {
        println!("No .gitnexus directory found.");
    }
    Ok(())
}
