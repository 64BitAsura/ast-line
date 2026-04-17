use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoMeta {
    pub name: String,
    pub path: String,
    pub repo_path: String,
    pub indexed_at: String,
    pub last_commit: Option<String>,
    pub stats: RepoStats,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RepoStats {
    pub files: usize,
    pub nodes: usize,
    pub edges: usize,
    pub communities: usize,
    pub processes: usize,
}
