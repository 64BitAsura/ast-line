use anyhow::Result;
use std::path::Path;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GrepResult {
    pub file_path: String,
    pub line_number: usize,
    pub line_content: String,
    pub match_start: usize,
    pub match_end: usize,
}

pub fn grep_files(
    repo_path: &Path,
    query: &str,
    case_insensitive: bool,
) -> Result<Vec<GrepResult>> {
    use crate::config::build_ignore_filter;

    let builder = build_ignore_filter(repo_path)?;
    let walker = builder.build();

    let mut results: Vec<GrepResult> = Vec::new();

    for entry in walker.flatten() {
        if !entry.file_type().map(|ft| ft.is_file()).unwrap_or(false) {
            continue;
        }
        let path = entry.path();
        let Ok(content) = std::fs::read_to_string(path) else { continue };

        let relative = path
            .strip_prefix(repo_path)
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .unwrap_or_default();

        for (line_num, line) in content.lines().enumerate() {
            let haystack = if case_insensitive { line.to_lowercase() } else { line.to_string() };
            let needle = if case_insensitive { query.to_lowercase() } else { query.to_string() };
            if let Some(pos) = haystack.find(&needle) {
                results.push(GrepResult {
                    file_path: relative.clone(),
                    line_number: line_num + 1,
                    line_content: line.to_string(),
                    match_start: pos,
                    match_end: pos + needle.len(),
                });
            }
        }
    }

    Ok(results)
}
