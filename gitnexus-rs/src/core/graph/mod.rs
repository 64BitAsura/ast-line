use std::collections::HashMap;

use crate::types::{GraphNode, GraphRelationship, NodeKind, RelationshipKind};

/// In-memory knowledge graph.
///
/// Stores nodes and directed relationships. Mirrors the `KnowledgeGraph`
/// interface from the TypeScript implementation.
#[derive(Debug, Default)]
pub struct KnowledgeGraph {
    nodes: HashMap<String, GraphNode>,
    relationships: HashMap<String, GraphRelationship>,
}

impl KnowledgeGraph {
    pub fn new() -> Self {
        Self::default()
    }

    // ── Mutation ──────────────────────────────────────────────────────────

    /// Add a node if a node with that id does not already exist.
    pub fn add_node(&mut self, node: GraphNode) {
        self.nodes.entry(node.id.clone()).or_insert(node);
    }

    /// Add a relationship if a relationship with that id does not already exist.
    pub fn add_relationship(&mut self, rel: GraphRelationship) {
        self.relationships.entry(rel.id.clone()).or_insert(rel);
    }

    /// Remove a node and all relationships that reference it.
    pub fn remove_node(&mut self, node_id: &str) -> bool {
        if !self.nodes.contains_key(node_id) {
            return false;
        }
        self.nodes.remove(node_id);
        self.relationships
            .retain(|_, r| r.source_id != node_id && r.target_id != node_id);
        true
    }

    /// Remove all nodes whose `filePath` property matches `file_path`,
    /// along with their relationships. Returns the number of nodes removed.
    pub fn remove_nodes_by_file(&mut self, file_path: &str) -> usize {
        let ids: Vec<String> = self
            .nodes
            .values()
            .filter(|n| n.file_path() == Some(file_path))
            .map(|n| n.id.clone())
            .collect();

        let count = ids.len();
        for id in &ids {
            self.remove_node(id);
        }
        count
    }

    // ── Query ─────────────────────────────────────────────────────────────

    pub fn get_node(&self, id: &str) -> Option<&GraphNode> {
        self.nodes.get(id)
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn relationship_count(&self) -> usize {
        self.relationships.len()
    }

    pub fn nodes(&self) -> impl Iterator<Item = &GraphNode> {
        self.nodes.values()
    }

    pub fn relationships(&self) -> impl Iterator<Item = &GraphRelationship> {
        self.relationships.values()
    }

    /// All nodes of the given kind.
    pub fn nodes_of_kind(&self, kind: &NodeKind) -> impl Iterator<Item = &GraphNode> {
        self.nodes.values().filter(move |n| &n.kind == kind)
    }

    /// All outgoing relationships of kind `kind` from `source_id`.
    pub fn outgoing(
        &self,
        source_id: &str,
        kind: &RelationshipKind,
    ) -> impl Iterator<Item = &GraphRelationship> {
        let source_id = source_id.to_owned();
        let kind = kind.clone();
        self.relationships
            .values()
            .filter(move |r| r.source_id == source_id && r.kind == kind)
    }
}
