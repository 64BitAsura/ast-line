use std::path::{Path, PathBuf};

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

pub fn has_git_dir(path: &Path) -> bool {
    path.join(".git").is_dir()
}
