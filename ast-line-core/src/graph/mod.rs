use std::collections::HashMap;
use crate::types::{GraphNode, GraphRelationship, NodeKind, RelationshipKind};

#[derive(Debug, Default)]
pub struct KnowledgeGraph {
    nodes: HashMap<String, GraphNode>,
    relationships: HashMap<String, GraphRelationship>,
}

impl KnowledgeGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node(&mut self, node: GraphNode) {
        self.nodes.entry(node.id.clone()).or_insert(node);
    }

    pub fn add_relationship(&mut self, rel: GraphRelationship) {
        self.relationships.entry(rel.id.clone()).or_insert(rel);
    }

    pub fn remove_node(&mut self, node_id: &str) -> bool {
        if !self.nodes.contains_key(node_id) {
            return false;
        }
        self.nodes.remove(node_id);
        self.relationships
            .retain(|_, r| r.source_id != node_id && r.target_id != node_id);
        true
    }

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

    pub fn nodes_of_kind<'a>(&'a self, kind: &'a NodeKind) -> impl Iterator<Item = &'a GraphNode> {
        self.nodes.values().filter(move |n| &n.kind == kind)
    }

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

    pub fn serialize_to_value(&self) -> serde_json::Value {
        serde_json::json!({
            "nodes": self.all_nodes_json(),
            "relationships": self.all_relationships_json(),
        })
    }

    pub fn all_nodes_json(&self) -> Vec<serde_json::Value> {
        self.nodes.values().map(|n| serde_json::to_value(n).unwrap_or_default()).collect()
    }

    pub fn all_relationships_json(&self) -> Vec<serde_json::Value> {
        self.relationships.values().map(|r| serde_json::to_value(r).unwrap_or_default()).collect()
    }
}
