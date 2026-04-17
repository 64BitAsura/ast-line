use anyhow::Result;
use regex::Regex;

use crate::graph::KnowledgeGraph;
use crate::types::graph::{GraphNode, GraphRelationship, NodeKind, RelationshipKind};
use crate::types::pipeline::{PipelinePhase, PipelineProgress, PipelineStats};
use super::structure::StructureOutput;

pub struct ParseOutput {
    pub scanned_files: Vec<crate::ingestion::filesystem_walker::ScannedFile>,
    pub all_paths: Vec<String>,
    pub total_files: usize,
}

pub struct ParsePhase;

impl ParsePhase {
    pub fn execute(
        repo_path: &std::path::Path,
        structure: StructureOutput,
        graph: &mut KnowledgeGraph,
        on_progress: &mut dyn FnMut(PipelineProgress),
    ) -> Result<ParseOutput> {
        let total_files = structure.total_files;

        on_progress(PipelineProgress {
            phase: PipelinePhase::Parse,
            percent: 20,
            message: "Parsing symbols...".into(),
            detail: None,
            stats: Some(PipelineStats {
                files_processed: 0,
                total_files,
                nodes_created: graph.node_count(),
                relationships_created: graph.relationship_count(),
            }),
        });

        let parseable: Vec<_> = structure.scanned_files.iter()
            .filter(|f| is_parseable(&f.path))
            .cloned()
            .collect();

        let total_parseable = parseable.len().max(1);

        for (i, file) in parseable.iter().enumerate() {
            let full_path = repo_path.join(&file.path);
            if let Ok(content) = std::fs::read_to_string(&full_path) {
                extract_symbols(graph, &file.path, &content);
            }
            let pct = 20 + ((i + 1) as f64 / total_parseable as f64 * 50.0) as u8;
            on_progress(PipelineProgress {
                phase: PipelinePhase::Parse,
                percent: pct,
                message: "Parsing symbols...".into(),
                detail: Some(file.path.clone()),
                stats: Some(PipelineStats {
                    files_processed: i + 1,
                    total_files,
                    nodes_created: graph.node_count(),
                    relationships_created: graph.relationship_count(),
                }),
            });
        }

        on_progress(PipelineProgress {
            phase: PipelinePhase::Parse,
            percent: 70,
            message: "Symbol parsing complete".into(),
            detail: None,
            stats: Some(PipelineStats {
                files_processed: total_files,
                total_files,
                nodes_created: graph.node_count(),
                relationships_created: graph.relationship_count(),
            }),
        });

        Ok(ParseOutput {
            scanned_files: structure.scanned_files,
            all_paths: structure.all_paths,
            total_files,
        })
    }
}

fn is_parseable(path: &str) -> bool {
    matches!(
        path.rsplit('.').next().unwrap_or(""),
        "rs" | "ts" | "tsx" | "js" | "jsx" | "py"
    )
}

fn extract_symbols(graph: &mut KnowledgeGraph, file_path: &str, content: &str) {
    let file_id = format!("file:{file_path}");
    let ext = file_path.rsplit('.').next().unwrap_or("");

    let patterns: Vec<(NodeKind, Regex)> = match ext {
        "rs" => vec![
            (NodeKind::Function, Regex::new(r"(?m)^\s*(?:pub\s+)?(?:async\s+)?fn\s+(\w+)").unwrap()),
            (NodeKind::Struct, Regex::new(r"(?m)^\s*(?:pub\s+)?struct\s+(\w+)").unwrap()),
            (NodeKind::Enum, Regex::new(r"(?m)^\s*(?:pub\s+)?enum\s+(\w+)").unwrap()),
            (NodeKind::Trait, Regex::new(r"(?m)^\s*(?:pub\s+)?trait\s+(\w+)").unwrap()),
            (NodeKind::Impl, Regex::new(r"(?m)^\s*impl(?:<[^>]*>)?\s+(\w+)").unwrap()),
        ],
        "ts" | "tsx" => vec![
            (NodeKind::Function, Regex::new(r"(?m)^\s*(?:export\s+)?(?:async\s+)?function\s+(\w+)").unwrap()),
            (NodeKind::Class, Regex::new(r"(?m)^\s*(?:export\s+)?class\s+(\w+)").unwrap()),
            (NodeKind::Interface, Regex::new(r"(?m)^\s*(?:export\s+)?interface\s+(\w+)").unwrap()),
            (NodeKind::TypeAlias, Regex::new(r"(?m)^\s*(?:export\s+)?type\s+(\w+)\s*=").unwrap()),
            (NodeKind::Enum, Regex::new(r"(?m)^\s*(?:export\s+)?enum\s+(\w+)").unwrap()),
        ],
        "js" | "jsx" => vec![
            (NodeKind::Function, Regex::new(r"(?m)^\s*(?:export\s+)?(?:async\s+)?function\s+(\w+)").unwrap()),
            (NodeKind::Class, Regex::new(r"(?m)^\s*(?:export\s+)?class\s+(\w+)").unwrap()),
        ],
        "py" => vec![
            (NodeKind::Function, Regex::new(r"(?m)^\s*def\s+(\w+)").unwrap()),
            (NodeKind::Class, Regex::new(r"(?m)^\s*class\s+(\w+)").unwrap()),
        ],
        _ => vec![],
    };

    for (kind, re) in &patterns {
        for cap in re.captures_iter(content) {
            if let Some(name_match) = cap.get(1) {
                let name = name_match.as_str().to_string();
                let node_id = format!("sym:{}:{}:{}", file_path, kind_str(kind), name);
                let node = GraphNode::new(&node_id, kind.clone(), &name)
                    .with_property("filePath", serde_json::Value::String(file_path.to_string()));
                graph.add_node(node);

                let rel_id = format!("contains:{}:{}", file_id, node_id);
                graph.add_relationship(GraphRelationship::new(
                    rel_id,
                    RelationshipKind::Contains,
                    &file_id,
                    &node_id,
                ));
            }
        }
    }
}

fn kind_str(kind: &NodeKind) -> &'static str {
    match kind {
        NodeKind::Function => "fn",
        NodeKind::Class => "class",
        NodeKind::Struct => "struct",
        NodeKind::Enum => "enum",
        NodeKind::Trait => "trait",
        NodeKind::Impl => "impl",
        NodeKind::Interface => "interface",
        NodeKind::TypeAlias => "type",
        _ => "sym",
    }
}
