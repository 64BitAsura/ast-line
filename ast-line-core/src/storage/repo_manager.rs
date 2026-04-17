use anyhow::Result;
use std::path::{Path, PathBuf};
use crate::types::meta::{RepoMeta, RepoStats};

#[derive(Debug, Clone)]
pub struct StoragePaths {
    pub gitnexus_dir: PathBuf,
    pub lbug_dir: PathBuf,
    pub meta_path: PathBuf,
}

pub fn get_storage_paths(repo_path: &Path) -> StoragePaths {
    let gitnexus_dir = repo_path.join(".gitnexus");
    let lbug_dir = gitnexus_dir.join("lbug");
    let meta_path = gitnexus_dir.join("meta.json");
    StoragePaths { gitnexus_dir, lbug_dir, meta_path }
}

pub fn get_global_registry_path() -> Option<PathBuf> {
    home::home_dir().map(|h| h.join(".gitnexus").join("registry.json"))
}

pub async fn load_graph(repo_path: &Path) -> Result<crate::graph::KnowledgeGraph> {
    let paths = get_storage_paths(repo_path);
    let mut graph = crate::graph::KnowledgeGraph::new();

    let nodes_path = paths.lbug_dir.join("nodes.json");
    let rels_path = paths.lbug_dir.join("relationships.json");

    if nodes_path.exists() {
        let data = tokio::fs::read_to_string(&nodes_path).await?;
        let nodes: Vec<crate::types::GraphNode> = serde_json::from_str(&data)?;
        for node in nodes {
            graph.add_node(node);
        }
    }

    if rels_path.exists() {
        let data = tokio::fs::read_to_string(&rels_path).await?;
        let rels: Vec<crate::types::GraphRelationship> = serde_json::from_str(&data)?;
        for rel in rels {
            graph.add_relationship(rel);
        }
    }

    Ok(graph)
}

pub async fn load_meta(repo_path: &Path) -> Result<Option<RepoMeta>> {
    let paths = get_storage_paths(repo_path);
    let meta_file = paths.gitnexus_dir.join("repo_meta.json");
    if !meta_file.exists() {
        return Ok(None);
    }
    let data = tokio::fs::read_to_string(&meta_file).await?;
    let meta: RepoMeta = serde_json::from_str(&data)?;
    Ok(Some(meta))
}

pub async fn save_meta(repo_path: &Path, meta: &RepoMeta) -> Result<()> {
    let paths = get_storage_paths(repo_path);
    tokio::fs::create_dir_all(&paths.gitnexus_dir).await?;
    let meta_file = paths.gitnexus_dir.join("repo_meta.json");
    let data = serde_json::to_string_pretty(meta)?;
    tokio::fs::write(&meta_file, data).await?;
    Ok(())
}

pub async fn register_repo(meta: &RepoMeta) -> Result<()> {
    let mut repos = list_repos().await?;
    repos.retain(|r| r.name != meta.name);
    repos.push(meta.clone());
    save_registry(&repos).await
}

pub async fn list_repos() -> Result<Vec<RepoMeta>> {
    let Some(registry_path) = get_global_registry_path() else {
        return Ok(vec![]);
    };
    if !registry_path.exists() {
        return Ok(vec![]);
    }
    let data = tokio::fs::read_to_string(&registry_path).await?;
    let repos: Vec<RepoMeta> = serde_json::from_str(&data).unwrap_or_default();
    Ok(repos)
}

pub async fn unregister_repo(name: &str) -> Result<()> {
    let mut repos = list_repos().await?;
    repos.retain(|r| r.name != name);
    save_registry(&repos).await
}

async fn save_registry(repos: &[RepoMeta]) -> Result<()> {
    let Some(registry_path) = get_global_registry_path() else {
        anyhow::bail!("Could not determine home directory");
    };
    if let Some(parent) = registry_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let data = serde_json::to_string_pretty(repos)?;
    tokio::fs::write(&registry_path, data).await?;
    Ok(())
}

pub fn make_repo_meta(
    name: &str,
    repo_path: &Path,
    nodes: usize,
    edges: usize,
    files: usize,
    communities: usize,
) -> RepoMeta {
    let gitnexus_dir = repo_path.join(".gitnexus");
    RepoMeta {
        name: name.to_string(),
        path: gitnexus_dir.to_string_lossy().to_string(),
        repo_path: repo_path.to_string_lossy().to_string(),
        indexed_at: chrono_now(),
        last_commit: None,
        stats: RepoStats {
            files,
            nodes,
            edges,
            communities,
            processes: 0,
        },
    }
}

fn chrono_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let mins = secs / 60;
    let hours = mins / 60;
    let days_total = hours / 24;
    let hour = hours % 24;
    let min = mins % 60;
    let sec = secs % 60;
    let (year, month, day) = days_to_date(days_total);
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{min:02}:{sec:02}Z")
}

fn days_to_date(mut days: u64) -> (u64, u64, u64) {
    let mut year = 1970u64;
    loop {
        let leap = is_leap(year);
        let days_in_year = if leap { 366 } else { 365 };
        if days < days_in_year { break; }
        days -= days_in_year;
        year += 1;
    }
    let leap = is_leap(year);
    let month_days: [u64; 12] = [31, if leap { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut month = 1u64;
    for &md in &month_days {
        if days < md { break; }
        days -= md;
        month += 1;
    }
    (year, month, days + 1)
}

fn is_leap(year: u64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}
