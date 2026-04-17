use anyhow::{bail, Result};
use clap::Args;
use std::path::PathBuf;

use ast_line_core::ingestion::pipeline::{PipelineOptions, run_pipeline_from_repo};
use ast_line_core::storage::git::get_git_root;

#[derive(Args, Debug)]
pub struct AnalyzeCommand {
    #[arg(value_name = "PATH")]
    pub path: Option<PathBuf>,
    #[arg(short, long)]
    pub force: bool,
    #[arg(short, long)]
    pub verbose: bool,
    #[arg(long)]
    pub skip_git: bool,
}

pub async fn run(cmd: AnalyzeCommand) -> Result<()> {
    let repo_path = match cmd.path {
        Some(p) => p.canonicalize()?,
        None => {
            let cwd = std::env::current_dir()?;
            match get_git_root(&cwd) {
                Some(root) => root,
                None => {
                    if cmd.skip_git {
                        cwd
                    } else {
                        bail!("No .git directory found. Use --skip-git to index a non-git folder.");
                    }
                }
            }
        }
    };

    println!("Indexing: {}", repo_path.display());

    let options = PipelineOptions {
        force: cmd.force,
        verbose: cmd.verbose,
    };

    run_pipeline_from_repo(&repo_path, options, |progress| {
        println!("[{}%] {}", progress.percent, progress.message);
    })
    .await?;

    println!("Done. Knowledge graph stored in {}", repo_path.join(".gitnexus").display());
    Ok(())
}
