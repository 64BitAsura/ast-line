use anyhow::Result;
use std::path::Path;
use std::time::Instant;

use crate::graph::KnowledgeGraph;
use crate::ingestion::pipeline_phases::{
    CommunitiesPhase, CrossFilePhase, ParsePhase, ScanPhase, StructurePhase,
};
use crate::storage::repo_manager::get_storage_paths;
use crate::types::pipeline::{PipelinePhase, PipelineProgress, PipelineStats};

#[derive(Debug, Default, Clone)]
pub struct PipelineOptions {
    pub force: bool,
    pub verbose: bool,
}

pub async fn run_pipeline_from_repo(
    repo_path: &Path,
    options: PipelineOptions,
    mut on_progress: impl FnMut(PipelineProgress),
) -> Result<()> {
    let start = Instant::now();
    let mut graph = KnowledgeGraph::new();

    let scan_output = ScanPhase::execute(repo_path, &graph, &mut on_progress)?;
    let total_files = scan_output.total_files;

    let structure_output = StructurePhase::execute(scan_output, &mut graph, &mut on_progress)?;

    let parse_output = ParsePhase::execute(repo_path, structure_output, &mut graph, &mut on_progress)?;

    CrossFilePhase::execute(parse_output, &mut graph, &mut on_progress)?;

    CommunitiesPhase::execute(&mut graph, &mut on_progress)?;

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

async fn persist_graph(repo_path: &Path, graph: &KnowledgeGraph, verbose: bool) -> Result<()> {
    let paths = get_storage_paths(repo_path);
    tokio::fs::create_dir_all(&paths.lbug_dir).await?;

    let nodes: Vec<&crate::types::GraphNode> = graph.nodes().collect();
    let rels: Vec<&crate::types::GraphRelationship> = graph.relationships().collect();

    let nodes_json = serde_json::to_string_pretty(&nodes)?;
    let rels_json = serde_json::to_string_pretty(&rels)?;

    tokio::fs::write(paths.lbug_dir.join("nodes.json"), &nodes_json).await?;
    tokio::fs::write(paths.lbug_dir.join("relationships.json"), &rels_json).await?;

    let meta = serde_json::json!({
        "indexedAt": chrono_now(),
        "nodeCount": nodes.len(),
        "relationshipCount": rels.len(),
    });
    tokio::fs::write(&paths.meta_path, serde_json::to_string_pretty(&meta)?).await?;

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

fn chrono_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let mins = secs / 60;
    let hours = mins / 60;
    let days_total = hours / 24;
    let hour = hours % 24;
    let min = mins % 60;
    let sec = secs % 60;
    let (year, month, day) = days_to_date(days_total);
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{min:02}:{sec:02}Z")
}

fn days_to_date(mut days: u64) -> (u64, u64, u64) {
    let mut year = 1970u64;
    loop {
        let leap = is_leap(year);
        let days_in_year = if leap { 366 } else { 365 };
        if days < days_in_year { break; }
        days -= days_in_year;
        year += 1;
    }
    let leap = is_leap(year);
    let month_days: [u64; 12] = [31, if leap { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut month = 1u64;
    for &md in &month_days {
        if days < md { break; }
        days -= md;
        month += 1;
    }
    (year, month, days + 1)
}

fn is_leap(year: u64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}
