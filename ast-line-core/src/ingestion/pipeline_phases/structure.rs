use anyhow::Result;
use std::collections::HashSet;

use crate::graph::KnowledgeGraph;
use crate::ingestion::filesystem_walker::ScannedFile;
use crate::types::graph::{GraphNode, GraphRelationship, NodeKind, RelationshipKind};
use crate::types::pipeline::{PipelinePhase, PipelineProgress, PipelineStats};
use super::scan::ScanOutput;

pub struct StructureOutput {
    pub scanned_files: Vec<ScannedFile>,
    pub all_paths: Vec<String>,
    pub all_path_set: HashSet<String>,
    pub total_files: usize,
}

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
            message: "Analyzing project structure...".into(),
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
            message: "Project structure analyzed".into(),
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

fn process_structure(graph: &mut KnowledgeGraph, all_paths: &[String]) {
    let mut seen_folders: HashSet<String> = HashSet::new();

    for file_path in all_paths {
        let file_id = format!("file:{file_path}");
        let file_name = std::path::Path::new(file_path)
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| file_path.clone());

        let file_node = GraphNode::new(&file_id, NodeKind::File, &file_name)
            .with_property("filePath", serde_json::Value::String(file_path.clone()));
        graph.add_node(file_node);

        let mut child_id = file_id.clone();
        let mut remaining: &str = file_path;

        loop {
            let parent_dir = match remaining.rfind('/') {
                Some(idx) => &remaining[..idx],
                None => {
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
