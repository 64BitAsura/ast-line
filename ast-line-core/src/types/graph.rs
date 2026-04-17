use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NodeKind {
    File,
    Folder,
    Function,
    Class,
    Interface,
    TypeAlias,
    Enum,
    Variable,
    Import,
    Export,
    Route,
    Tool,
    Process,
    Community,
    Markdown,
    CobolProgram,
    CobolParagraph,
    CobolSection,
    CobolCopybook,
    Struct,
    Trait,
    Impl,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RelationshipKind {
    Contains,
    Imports,
    Calls,
    Extends,
    Implements,
    Uses,
    Exports,
    HasRoute,
    HasTool,
    PartOfProcess,
    PartOfCommunity,
    MethodOverrides,
    DependsOn,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: String,
    pub kind: NodeKind,
    pub name: String,
    pub properties: HashMap<String, serde_json::Value>,
}

impl GraphNode {
    pub fn new(id: impl Into<String>, kind: NodeKind, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            kind,
            name: name.into(),
            properties: HashMap::new(),
        }
    }

    pub fn with_property(mut self, key: impl Into<String>, value: impl Into<serde_json::Value>) -> Self {
        self.properties.insert(key.into(), value.into());
        self
    }

    pub fn file_path(&self) -> Option<&str> {
        self.properties.get("filePath")?.as_str()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphRelationship {
    pub id: String,
    pub kind: RelationshipKind,
    pub source_id: String,
    pub target_id: String,
    pub properties: HashMap<String, serde_json::Value>,
}

impl GraphRelationship {
    pub fn new(
        id: impl Into<String>,
        kind: RelationshipKind,
        source_id: impl Into<String>,
        target_id: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            kind,
            source_id: source_id.into(),
            target_id: target_id.into(),
            properties: HashMap::new(),
        }
    }
}
