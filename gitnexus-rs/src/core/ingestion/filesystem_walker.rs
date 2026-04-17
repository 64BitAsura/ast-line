use anyhow::Result;
use std::path::Path;

use crate::config::build_ignore_filter;

/// A scanned file entry — relative path and size in bytes.
/// No file contents are loaded at this stage.
#[derive(Debug, Clone)]
pub struct ScannedFile {
    /// Repository-relative path using forward slashes.
    pub path: String,
    /// File size in bytes.
    pub size: u64,
}

/// Maximum file size to include (512 KiB). Larger files are likely generated
/// or vendored and may crash parsers.
const MAX_FILE_SIZE: u64 = 512 * 1024;

/// Walk `repo_path`, collect all eligible file paths and their sizes.
///
/// Respects `.gitignore` files and the hard-coded ignore list.
/// Does **not** read file contents — that happens in downstream pipeline phases.
///
/// `on_progress(current, total, relative_path)` is called for each file
/// after it is processed.
pub fn walk_repository_paths(
    repo_path: &Path,
    mut on_progress: impl FnMut(usize, usize, &str),
) -> Result<Vec<ScannedFile>> {
    let builder = build_ignore_filter(repo_path)?;
    let walker = builder.build();

    // Collect all matching regular-file entries first so we know the total.
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
            Err(_) => {
                // Skip unreadable files silently.
            }
        }

        on_progress(i + 1, total, &relative);
    }

    if skipped_large > 0 {
        eprintln!(
            "  Skipped {skipped_large} large files (>{}KB, likely generated/vendored)",
            MAX_FILE_SIZE / 1024
        );
    }

    Ok(entries)
}
