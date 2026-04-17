use anyhow::Result;
use crate::graph::KnowledgeGraph;
use crate::types::graph::{RelationshipKind, NodeKind, GraphRelationship};
use crate::types::pipeline::{PipelinePhase, PipelineProgress, PipelineStats};
use super::parse::ParseOutput;

pub struct CrossFilePhase;

impl CrossFilePhase {
    pub fn execute(
        parse: ParseOutput,
        graph: &mut KnowledgeGraph,
        on_progress: &mut dyn FnMut(PipelineProgress),
    ) -> Result<()> {
        on_progress(PipelineProgress {
            phase: PipelinePhase::CrossFile,
            percent: 70,
            message: "Resolving cross-file references...".into(),
            detail: None,
            stats: Some(PipelineStats {
                files_processed: parse.total_files,
                total_files: parse.total_files,
                nodes_created: graph.node_count(),
                relationships_created: graph.relationship_count(),
            }),
        });

        // Collect import nodes
        let import_nodes: Vec<(String, String)> = graph
            .nodes_of_kind(&NodeKind::Import)
            .filter_map(|n| {
                let file_path = n.properties.get("filePath")?.as_str()?.to_string();
                Some((n.id.clone(), file_path))
            })
            .collect();

        // Try to resolve each import to a file node
        let mut new_rels: Vec<GraphRelationship> = Vec::new();
        for (import_id, file_path) in &import_nodes {
            let file_node_id = format!("file:{file_path}");
            if graph.get_node(&file_node_id).is_some() {
                let rel_id = format!("imports:{}:{}", file_path, import_id);
                new_rels.push(GraphRelationship::new(
                    rel_id,
                    RelationshipKind::Imports,
                    &file_node_id,
                    import_id.as_str(),
                ));
            }
        }

        for rel in new_rels {
            graph.add_relationship(rel);
        }

        on_progress(PipelineProgress {
            phase: PipelinePhase::CrossFile,
            percent: 80,
            message: "Cross-file references resolved".into(),
            detail: None,
            stats: Some(PipelineStats {
                files_processed: parse.total_files,
                total_files: parse.total_files,
                nodes_created: graph.node_count(),
                relationships_created: graph.relationship_count(),
            }),
        });

        Ok(())
    }
}
