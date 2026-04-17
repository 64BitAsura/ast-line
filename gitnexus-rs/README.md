# gitnexus-rs

A Rust rewrite of [GitNexus](https://github.com/abhigyanpatwari/GitNexus) — graph-powered code intelligence for AI agents.

## Status

🚧 **Work in progress** — initial scaffold.

## What's implemented

| Component | Status |
|-----------|--------|
| CLI (`analyze`, `status`) | ✅ Scaffold |
| Config / ignore service | ✅ Respects `.gitignore` + hardcoded excludes |
| `scan` phase — filesystem walker | ✅ Implemented |
| `structure` phase — File/Folder graph nodes | ✅ Implemented |
| Graph persistence (JSON) | ✅ Implemented |
| `parse` phase — tree-sitter AST | 🔲 Pending |
| `crossFile` phase — type propagation | 🔲 Pending |
| `mro` phase — method resolution | 🔲 Pending |
| `communities` phase — Leiden clustering | 🔲 Pending |
| `processes` phase — execution flows | 🔲 Pending |
| MCP server | 🔲 Pending |
| LadybugDB graph store | 🔲 Pending |
| Embeddings | 🔲 Pending |

## Building

```bash
cargo build --release
```

## Usage

```bash
# Index the current repository
gitnexus analyze

# Index a specific path
gitnexus analyze /path/to/repo

# Check indexing status
gitnexus status
```

## Architecture

The ingestion pipeline mirrors the TypeScript original's DAG of named phases:

```
scan → structure → parse → [routes, tools, orm]
  → crossFile → mro → communities → processes
```

Each phase lives in `src/core/ingestion/pipeline_phases/` with a single
`execute()` method that takes typed inputs from upstream phases.

See [ARCHITECTURE.md](../GitNexus/ARCHITECTURE.md) in the reference
submodule for the full design spec.
