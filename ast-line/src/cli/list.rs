use anyhow::Result;
use ast_line_core::storage::repo_manager::list_repos;

pub async fn run() -> Result<()> {
    let repos = list_repos().await?;
    if repos.is_empty() {
        println!("No indexed repositories.");
    } else {
        for repo in &repos {
            println!("{} — {} ({} nodes, {} edges)", repo.name, repo.repo_path, repo.stats.nodes, repo.stats.edges);
        }
    }
    Ok(())
}
