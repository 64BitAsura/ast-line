use anyhow::Result;
use std::collections::{HashMap, VecDeque};
use uuid::Uuid;

use crate::graph::KnowledgeGraph;
use crate::types::graph::{GraphNode, GraphRelationship, NodeKind, RelationshipKind};
use crate::types::pipeline::{PipelinePhase, PipelineProgress, PipelineStats};

pub struct CommunitiesPhase;

impl CommunitiesPhase {
    pub fn execute(
        graph: &mut KnowledgeGraph,
        on_progress: &mut dyn FnMut(PipelineProgress),
    ) -> Result<()> {
        on_progress(PipelineProgress {
            phase: PipelinePhase::Communities,
            percent: 80,
            message: "Detecting communities...".into(),
            detail: None,
            stats: Some(PipelineStats {
                files_processed: 0,
                total_files: 0,
                nodes_created: graph.node_count(),
                relationships_created: graph.relationship_count(),
            }),
        });

        // Build adjacency for connected-components (undirected)
        let node_ids: Vec<String> = graph.nodes().map(|n| n.id.clone()).collect();
        let mut adj: HashMap<String, Vec<String>> = HashMap::new();
        for id in &node_ids {
            adj.entry(id.clone()).or_default();
        }
        for rel in graph.relationships() {
            adj.entry(rel.source_id.clone()).or_default().push(rel.target_id.clone());
            adj.entry(rel.target_id.clone()).or_default().push(rel.source_id.clone());
        }

        let mut visited: HashMap<String, usize> = HashMap::new();
        let mut component_id = 0usize;

        for id in &node_ids {
            if visited.contains_key(id) {
                continue;
            }
            // BFS
            let mut queue = VecDeque::new();
            queue.push_back(id.clone());
            visited.insert(id.clone(), component_id);
            while let Some(curr) = queue.pop_front() {
                if let Some(neighbors) = adj.get(&curr) {
                    for nb in neighbors {
                        if !visited.contains_key(nb) {
                            visited.insert(nb.clone(), component_id);
                            queue.push_back(nb.clone());
                        }
                    }
                }
            }
            component_id += 1;
        }

        // Group nodes by component
        let mut components: HashMap<usize, Vec<String>> = HashMap::new();
        for (node_id, comp) in &visited {
            components.entry(*comp).or_default().push(node_id.clone());
        }

        // Create community nodes and edges
        let mut community_nodes: Vec<GraphNode> = Vec::new();
        let mut community_rels: Vec<GraphRelationship> = Vec::new();

        for (comp_idx, members) in &components {
            if members.len() < 2 {
                continue;
            }
            let community_id = format!("community:{}", Uuid::new_v4());
            let community_node = GraphNode::new(
                &community_id,
                NodeKind::Community,
                &format!("Community {}", comp_idx),
            );
            community_nodes.push(community_node);

            for member_id in members {
                let rel_id = format!("partof:{}:{}", community_id, member_id);
                community_rels.push(GraphRelationship::new(
                    rel_id,
                    RelationshipKind::PartOfCommunity,
                    member_id.as_str(),
                    &community_id,
                ));
            }
        }

        for node in community_nodes {
            graph.add_node(node);
        }
        for rel in community_rels {
            graph.add_relationship(rel);
        }

        on_progress(PipelineProgress {
            phase: PipelinePhase::Communities,
            percent: 90,
            message: "Communities detected".into(),
            detail: None,
            stats: Some(PipelineStats {
                files_processed: 0,
                total_files: 0,
                nodes_created: graph.node_count(),
                relationships_created: graph.relationship_count(),
            }),
        });

        Ok(())
    }
}
