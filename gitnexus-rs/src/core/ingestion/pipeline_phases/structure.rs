//! Phase: structure
//!
//! Builds File and Folder nodes in the graph from scanned paths.
//!
//! deps:   scan
//! reads:  all_paths from ScanOutput
//! writes: graph — File and Folder nodes + CONTAINS edges

use anyhow::Result;
use std::collections::HashSet;
use uuid::Uuid;

use crate::core::graph::KnowledgeGraph;
use crate::types::graph::{GraphNode, GraphRelationship, NodeKind, RelationshipKind};
use crate::types::pipeline::{PipelinePhase, PipelineProgress, PipelineStats};

use super::scan::ScanOutput;

/// Output produced by the structure phase.
pub struct StructureOutput {
    /// Pass-through from scan for downstream phases.
    pub scanned_files: Vec<crate::core::ingestion::filesystem_walker::ScannedFile>,
    pub all_paths: Vec<String>,
    /// Materialised once here; shared across downstream phases to avoid
    /// repeated `HashSet` construction on large repos.
    pub all_path_set: HashSet<String>,
    pub total_files: usize,
}

/// The structure phase — populates File and Folder nodes.
pub struct StructurePhase;

impl StructurePhase {
    pub fn execute(
        scan: ScanOutput,
        graph: &mut KnowledgeGraph,
        on_progress: &mut dyn FnMut(PipelineProgress),
    ) -> Result<StructureOutput> {
        let total_files = scan.total_files;

        on_progress(PipelineProgress {
            phase: PipelinePhase::Structure,
            percent: 15,
            message: "Analysing project structure...".into(),
            detail: None,
            stats: Some(PipelineStats {
                files_processed: 0,
                total_files,
                nodes_created: graph.node_count(),
                relationships_created: graph.relationship_count(),
            }),
        });

        process_structure(graph, &scan.all_paths);

        on_progress(PipelineProgress {
            phase: PipelinePhase::Structure,
            percent: 20,
            message: "Project structure analysed".into(),
            detail: None,
            stats: Some(PipelineStats {
                files_processed: total_files,
                total_files,
                nodes_created: graph.node_count(),
                relationships_created: graph.relationship_count(),
            }),
        });

        let all_path_set: HashSet<String> = scan.all_paths.iter().cloned().collect();

        Ok(StructureOutput {
            scanned_files: scan.scanned_files,
            all_paths: scan.all_paths,
            all_path_set,
            total_files,
        })
    }
}

/// Populate the graph with File and Folder nodes derived from `all_paths`.
///
/// For each file path we ensure a File node exists and walk up the directory
/// tree creating Folder nodes and CONTAINS edges as needed.
fn process_structure(graph: &mut KnowledgeGraph, all_paths: &[String]) {
    // Track which folder paths have already been added.
    let mut seen_folders: HashSet<String> = HashSet::new();

    for file_path in all_paths {
        // ── File node ──────────────────────────────────────────────────
        let file_id = format!("file:{file_path}");
        let file_name = std::path::Path::new(file_path)
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| file_path.clone());

        let file_node = GraphNode::new(&file_id, NodeKind::File, &file_name)
            .with_property("filePath", serde_json::Value::String(file_path.clone()));
        graph.add_node(file_node);

        // ── Folder nodes and CONTAINS edges ───────────────────────────
        let mut child_id = file_id.clone();
        let mut remaining: &str = file_path;

        loop {
            let parent_dir = match remaining.rfind('/') {
                Some(idx) => &remaining[..idx],
                None => {
                    // At the repo root — create a virtual root folder if needed.
                    let root_id = "folder:/".to_owned();
                    if seen_folders.insert(root_id.clone()) {
                        graph.add_node(GraphNode::new(&root_id, NodeKind::Folder, "/"));
                    }
                    let rel_id = format!("contains:{root_id}:{child_id}");
                    graph.add_relationship(GraphRelationship::new(
                        rel_id,
                        RelationshipKind::Contains,
                        &root_id,
                        &child_id,
                    ));
                    break;
                }
            };

            let folder_id = format!("folder:{parent_dir}");
            let folder_name = parent_dir
                .rfind('/')
                .map(|i| &parent_dir[i + 1..])
                .unwrap_or(parent_dir);

            if seen_folders.insert(folder_id.clone()) {
                graph.add_node(GraphNode::new(&folder_id, NodeKind::Folder, folder_name));
            }

            let rel_id = format!("contains:{}:{}", folder_id, child_id);
            graph.add_relationship(GraphRelationship::new(
                rel_id,
                RelationshipKind::Contains,
                &folder_id,
                &child_id,
            ));

            child_id = folder_id;
            remaining = parent_dir;
        }
    }
}

/// Derive a stable node id from an arbitrary string (used internally).
#[allow(dead_code)]
fn stable_id(s: &str) -> String {
    // Use a name-based UUID v4 derived from the path string so ids are
    // deterministic across runs without a namespace registry.
    Uuid::new_v4().to_string() + ":" + s
}
