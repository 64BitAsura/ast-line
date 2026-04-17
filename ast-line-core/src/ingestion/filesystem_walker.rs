use anyhow::Result;
use std::path::Path;
use crate::config::build_ignore_filter;

#[derive(Debug, Clone)]
pub struct ScannedFile {
    pub path: String,
    pub size: u64,
}

const MAX_FILE_SIZE: u64 = 512 * 1024;

pub fn walk_repository_paths(
    repo_path: &Path,
    mut on_progress: impl FnMut(usize, usize, &str),
) -> Result<Vec<ScannedFile>> {
    let builder = build_ignore_filter(repo_path)?;
    let walker = builder.build();

    let mut candidate_paths: Vec<std::path::PathBuf> = Vec::new();
    for entry in walker.flatten() {
        if entry.file_type().map(|ft| ft.is_file()).unwrap_or(false) {
            candidate_paths.push(entry.into_path());
        }
    }

    let total = candidate_paths.len();
    let mut entries: Vec<ScannedFile> = Vec::with_capacity(total);
    let mut skipped_large = 0usize;

    for (i, full_path) in candidate_paths.iter().enumerate() {
        let relative = full_path
            .strip_prefix(repo_path)
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .unwrap_or_default();

        match std::fs::metadata(full_path) {
            Ok(meta) => {
                if meta.len() > MAX_FILE_SIZE {
                    skipped_large += 1;
                } else {
                    entries.push(ScannedFile {
                        path: relative.clone(),
                        size: meta.len(),
                    });
                }
            }
            Err(_) => {}
        }

        on_progress(i + 1, total, &relative);
    }

    if skipped_large > 0 {
        eprintln!("  Skipped {skipped_large} large files (>{}KB)", MAX_FILE_SIZE / 1024);
    }

    Ok(entries)
}
