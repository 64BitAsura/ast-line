use ignore::overrides::OverrideBuilder;
use ignore::WalkBuilder;
use std::path::Path;

/// Hard-coded paths that are always excluded from indexing, regardless of
/// .gitignore contents. Mirrors `DEFAULT_IGNORE_LIST` from the TypeScript
/// implementation.
const ALWAYS_IGNORE: &[&str] = &[
    // Version control
    ".git",
    ".svn",
    ".hg",
    ".bzr",
    // IDEs
    ".idea",
    ".vscode",
    ".vs",
    ".eclipse",
    ".settings",
    ".DS_Store",
    "Thumbs.db",
    // Dependencies
    "node_modules",
    "bower_components",
    "jspm_packages",
    "vendor",
    "venv",
    ".venv",
    "env",
    ".env",
    "__pycache__",
    ".pytest_cache",
    ".mypy_cache",
    "site-packages",
    ".tox",
    // Build outputs
    "dist",
    "build",
    "out",
    "output",
    "bin",
    "obj",
    "target",
    ".next",
    ".nuxt",
    ".output",
    ".vercel",
    ".netlify",
    ".serverless",
    "_build",
    ".parcel-cache",
    // Caches / lock files / generated
    ".cache",
    ".turbo",
    "coverage",
    ".nyc_output",
    // GitNexus own data
    ".gitnexus",
    // Large generated / vendored blobs
    "*.min.js",
    "*.min.css",
    "*.bundle.js",
    "*.chunk.js",
    "*.map",
];

/// Returns a pre-configured [`WalkBuilder`] that respects `.gitignore` files
/// and the hard-coded ignore list.
///
/// Callers can further customise the builder before calling `.build()`.
pub fn build_ignore_filter(repo_path: &Path) -> anyhow::Result<WalkBuilder> {
    let mut builder = WalkBuilder::new(repo_path);
    builder
        .hidden(false) // we manage hidden dirs ourselves
        .ignore(true) // respect .gitignore
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true);

    let mut overrides = OverrideBuilder::new(repo_path);
    for pattern in ALWAYS_IGNORE {
        // Prefix with `!` to negate (exclude) the pattern from the walk.
        overrides.add(&format!("!{pattern}"))?;
    }
    builder.overrides(overrides.build()?);

    Ok(builder)
}
