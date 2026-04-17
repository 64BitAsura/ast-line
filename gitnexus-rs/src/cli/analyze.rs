use anyhow::{bail, Result};
use clap::Args;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::PathBuf;
use std::time::Duration;

use crate::core::ingestion::pipeline::{PipelineOptions, run_pipeline_from_repo};
use crate::storage::git::get_git_root;

/// Index a repository and build its knowledge graph.
#[derive(Args, Debug)]
pub struct AnalyzeCommand {
    /// Path to the repository root. Defaults to the current working directory.
    #[arg(value_name = "PATH")]
    pub path: Option<PathBuf>,

    /// Force re-index even if the graph is up to date.
    #[arg(short, long)]
    pub force: bool,

    /// Enable verbose output during indexing.
    #[arg(short, long)]
    pub verbose: bool,

    /// Skip .git directory check (index any folder).
    #[arg(long)]
    pub skip_git: bool,
}

pub async fn run(cmd: AnalyzeCommand) -> Result<()> {
    println!("\n  {}\n", "GitNexus Analyzer".bold().cyan());

    // Resolve the repository path.
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
                        bail!(
                            "No .git directory found. Run from inside a git repository or pass a path.\n\
                             Use --skip-git to index a non-git folder."
                        );
                    }
                }
            }
        }
    };

    println!(
        "  {} {}",
        "Indexing:".bold(),
        repo_path.display().to_string().yellow()
    );

    // Set up progress bar.
    let pb = ProgressBar::new(100);
    pb.set_style(
        ProgressStyle::with_template(
            "  {spinner:.cyan} [{bar:40.cyan/blue}] {percent}% {msg}",
        )?
        .progress_chars("=>-"),
    );
    pb.enable_steady_tick(Duration::from_millis(80));

    let options = PipelineOptions {
        force: cmd.force,
        verbose: cmd.verbose,
    };

    run_pipeline_from_repo(&repo_path, options, |progress| {
        pb.set_position(progress.percent as u64);
        pb.set_message(progress.message.clone());
    })
    .await
    .map_err(|e| {
        pb.abandon_with_message(format!("{}", "Failed".red()));
        e
    })?;

    pb.finish_with_message(format!("{}", "Done".green().bold()));

    println!(
        "\n  {} Knowledge graph stored in {}",
        "✓".green().bold(),
        repo_path.join(".gitnexus").display().to_string().dimmed()
    );

    Ok(())
}
