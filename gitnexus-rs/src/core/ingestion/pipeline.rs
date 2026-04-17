//! Pipeline orchestrator — dependency-ordered ingestion pipeline.
//!
//! The pipeline is composed of named phases with explicit dependencies.
//! Each phase is defined in its own file under `pipeline_phases/`.
//!
//! Current phase dependency graph:
//!
//!   scan → structure → (parse → … future phases)
//!
//! To add a new phase:
//! 1. Create a new file in `pipeline_phases/`.
//! 2. Export it from `pipeline_phases/mod.rs`.
//! 3. Call it in the appropriate place in `run_pipeline_from_repo` below.

use anyhow::Result;
use std::path::Path;
use std::time::Instant;

use crate::core::graph::KnowledgeGraph;
use crate::core::ingestion::pipeline_phases::{ScanPhase, StructurePhase};
use crate::storage::repo_manager::get_storage_paths;
use crate::types::pipeline::{PipelinePhase, PipelineProgress, PipelineStats};

/// Options that control pipeline behaviour.
#[derive(Debug, Default, Clone)]
pub struct PipelineOptions {
    /// Re-index even if the graph appears up to date.
    pub force: bool,
    /// Print extra diagnostic output.
    pub verbose: bool,
}

/// Run the full ingestion pipeline for `repo_path`.
///
/// `on_progress` is called with a [`PipelineProgress`] update after each
/// sub-step. Errors from any phase are propagated immediately.
pub async fn run_pipeline_from_repo(
    repo_path: &Path,
    options: PipelineOptions,
    mut on_progress: impl FnMut(PipelineProgress),
) -> Result<()> {
    let start = Instant::now();
    let mut graph = KnowledgeGraph::new();

    // ── Phase 1: scan ────────────────────────────────────────────────────
    let scan_output = ScanPhase::execute(repo_path, &graph, &mut on_progress)?;
    let total_files = scan_output.total_files;

    // ── Phase 2: structure ───────────────────────────────────────────────
    let _structure_output = StructurePhase::execute(scan_output, &mut graph, &mut on_progress)?;

    // ── Persist ──────────────────────────────────────────────────────────
    on_progress(PipelineProgress {
        phase: PipelinePhase::Persisting,
        percent: 90,
        message: "Persisting knowledge graph...".into(),
        detail: None,
        stats: Some(PipelineStats {
            files_processed: total_files,
            total_files,
            nodes_created: graph.node_count(),
            relationships_created: graph.relationship_count(),
        }),
    });

    persist_graph(repo_path, &graph, options.verbose).await?;

    let elapsed = start.elapsed();

    on_progress(PipelineProgress {
        phase: PipelinePhase::Done,
        percent: 100,
        message: format!(
            "Indexed {total_files} files in {:.1}s — {} nodes, {} relationships",
            elapsed.as_secs_f64(),
            graph.node_count(),
            graph.relationship_count(),
        ),
        detail: None,
        stats: Some(PipelineStats {
            files_processed: total_files,
            total_files,
            nodes_created: graph.node_count(),
            relationships_created: graph.relationship_count(),
        }),
    });

    Ok(())
}

/// Serialise the graph to `.gitnexus/` inside the repository.
async fn persist_graph(
    repo_path: &Path,
    graph: &KnowledgeGraph,
    verbose: bool,
) -> Result<()> {
    let paths = get_storage_paths(repo_path);
    tokio::fs::create_dir_all(&paths.lbug_dir).await?;

    // Serialize nodes and relationships to JSON.
    let nodes: Vec<&crate::types::GraphNode> = graph.nodes().collect();
    let rels: Vec<&crate::types::GraphRelationship> = graph.relationships().collect();

    let nodes_json = serde_json::to_string_pretty(&nodes)?;
    let rels_json = serde_json::to_string_pretty(&rels)?;

    tokio::fs::write(paths.lbug_dir.join("nodes.json"), &nodes_json).await?;
    tokio::fs::write(paths.lbug_dir.join("relationships.json"), &rels_json).await?;

    // Write metadata.
    let meta = serde_json::json!({
        "indexedAt": chrono_now(),
        "nodeCount": nodes.len(),
        "relationshipCount": rels.len(),
    });
    tokio::fs::write(
        &paths.meta_path,
        serde_json::to_string_pretty(&meta)?,
    )
    .await?;

    if verbose {
        eprintln!(
            "  Wrote {} nodes and {} relationships to {}",
            nodes.len(),
            rels.len(),
            paths.lbug_dir.display()
        );
    }

    Ok(())
}

/// Return the current UTC time as an ISO-8601 string.
fn chrono_now() -> String {
    // Use std only — no chrono dependency yet.
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // Minimal ISO-8601 representation (seconds precision).
    let s = secs;
    let mins = s / 60;
    let hours = mins / 60;
    let days_total = hours / 24;
    let hour = hours % 24;
    let min = mins % 60;
    let sec = s % 60;
    // Days since 1970-01-01 → calendar date (Gregorian, no leap-second awareness).
    let (year, month, day) = days_to_date(days_total);
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{min:02}:{sec:02}Z")
}

/// Convert days since Unix epoch to (year, month, day).
fn days_to_date(mut days: u64) -> (u64, u64, u64) {
    let mut year = 1970u64;
    loop {
        let leap = is_leap(year);
        let days_in_year = if leap { 366 } else { 365 };
        if days < days_in_year {
            break;
        }
        days -= days_in_year;
        year += 1;
    }
    let leap = is_leap(year);
    let month_days: [u64; 12] = [31, if leap { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut month = 1u64;
    for &md in &month_days {
        if days < md {
            break;
        }
        days -= md;
        month += 1;
    }
    (year, month, days + 1)
}

fn is_leap(year: u64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}
