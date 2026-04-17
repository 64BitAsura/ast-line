use anyhow::Result;
use clap::Args;

use ast_line_core::search::{index_graph, search};
use ast_line_core::storage::repo_manager::{get_storage_paths, load_graph};

#[derive(Args, Debug)]
pub struct QueryCommand {
    pub query: String,
    #[arg(short, long)]
    pub name: Option<String>,
    #[arg(short, long, default_value = "20")]
    pub limit: usize,
}

pub async fn run(cmd: QueryCommand) -> Result<()> {
    let repo_path = match cmd.name {
        Some(name) => {
            let repos = ast_line_core::storage::repo_manager::list_repos().await?;
            let repo = repos.into_iter().find(|r| r.name == name)
                .ok_or_else(|| anyhow::anyhow!("Repository '{}' not found", name))?;
            std::path::PathBuf::from(repo.repo_path)
        }
        None => std::env::current_dir()?,
    };

    let _paths = get_storage_paths(&repo_path);
    let graph = load_graph(&repo_path).await?;
    let index = index_graph(&graph);
    let results = search(&index, &cmd.query, cmd.limit);

    if results.is_empty() {
        println!("No results found.");
    } else {
        for r in &results {
            println!("[{:.3}] {} — {}", r.score, r.node_id, r.snippet);
        }
    }

    Ok(())
}
