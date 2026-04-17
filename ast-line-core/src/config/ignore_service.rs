use ignore::overrides::OverrideBuilder;
use ignore::WalkBuilder;
use std::path::Path;

const ALWAYS_IGNORE: &[&str] = &[
    ".git", ".svn", ".hg", ".bzr",
    ".idea", ".vscode", ".vs", ".eclipse", ".settings",
    ".DS_Store", "Thumbs.db",
    "node_modules", "bower_components", "jspm_packages",
    "vendor", "venv", ".venv", "env", ".env",
    "__pycache__", ".pytest_cache", ".mypy_cache", "site-packages", ".tox",
    "dist", "build", "out", "output", "bin", "obj", "target",
    ".next", ".nuxt", ".output", ".vercel", ".netlify", ".serverless",
    "_build", ".parcel-cache",
    ".cache", ".turbo", "coverage", ".nyc_output",
    ".gitnexus",
    "*.min.js", "*.min.css", "*.bundle.js", "*.chunk.js", "*.map",
];

pub fn build_ignore_filter(repo_path: &Path) -> anyhow::Result<WalkBuilder> {
    let mut builder = WalkBuilder::new(repo_path);
    builder
        .hidden(false)
        .ignore(true)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true);

    let mut overrides = OverrideBuilder::new(repo_path);
    for pattern in ALWAYS_IGNORE {
        overrides.add(&format!("!{pattern}"))?;
    }
    builder.overrides(overrides.build()?);

    Ok(builder)
}
