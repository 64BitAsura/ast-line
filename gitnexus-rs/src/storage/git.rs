use std::path::{Path, PathBuf};

/// Walk upward from `start` looking for a `.git` directory.
/// Returns the first ancestor directory that contains `.git`, or `None`.
pub fn get_git_root(start: &Path) -> Option<PathBuf> {
    let mut current = start.to_path_buf();
    loop {
        if current.join(".git").exists() {
            return Some(current);
        }
        if !current.pop() {
            return None;
        }
    }
}

/// Return `true` if `path` contains a `.git` directory.
pub fn has_git_dir(path: &Path) -> bool {
    path.join(".git").is_dir()
}
