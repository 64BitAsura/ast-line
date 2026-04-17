use std::path::{Path, PathBuf};

/// Paths used for a single indexed repository.
#[derive(Debug, Clone)]
pub struct StoragePaths {
    /// The `.gitnexus/` directory inside the repository.
    pub gitnexus_dir: PathBuf,
    /// Path to the graph data directory (`lbug/`).
    pub lbug_dir: PathBuf,
    /// Path to the metadata file.
    pub meta_path: PathBuf,
}

/// Derive the storage paths for a given repository root.
pub fn get_storage_paths(repo_path: &Path) -> StoragePaths {
    let gitnexus_dir = repo_path.join(".gitnexus");
    let lbug_dir = gitnexus_dir.join("lbug");
    let meta_path = gitnexus_dir.join("meta.json");
    StoragePaths {
        gitnexus_dir,
        lbug_dir,
        meta_path,
    }
}

/// Path to the global registry file (`~/.gitnexus/registry.json`).
pub fn get_global_registry_path() -> Option<PathBuf> {
    home::home_dir().map(|h| h.join(".gitnexus").join("registry.json"))
}
