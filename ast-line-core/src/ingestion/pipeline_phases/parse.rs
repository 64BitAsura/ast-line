use anyhow::Result;
use regex::Regex;
use tree_sitter::{Node, Parser};

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

    if ext == "rs" {
        extract_rust_symbols(graph, &file_id, file_path, content);
        return;
    }

    let patterns: Vec<(NodeKind, Regex)> = match ext {
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
                add_symbol(graph, &file_id, file_path, kind.clone(), name_match.as_str());
            }
        }
    }
}

fn extract_rust_symbols(graph: &mut KnowledgeGraph, file_id: &str, file_path: &str, content: &str) {
    let mut parser = Parser::new();
    if parser
        .set_language(&tree_sitter_rust::LANGUAGE.into())
        .is_err()
    {
        return;
    }

    let Some(tree) = parser.parse(content, None) else {
        return;
    };

    let source = content.as_bytes();
    let mut stack = vec![tree.root_node()];

    while let Some(node) = stack.pop() {
        match node.kind() {
            "function_item" => {
                if let Some(name) = field_text(node, "name", source) {
                    add_symbol(graph, file_id, file_path, NodeKind::Function, &name);
                }
            }
            "struct_item" => {
                if let Some(name) = field_text(node, "name", source) {
                    add_symbol(graph, file_id, file_path, NodeKind::Struct, &name);
                }
            }
            "enum_item" => {
                if let Some(name) = field_text(node, "name", source) {
                    add_symbol(graph, file_id, file_path, NodeKind::Enum, &name);
                }
            }
            "trait_item" => {
                if let Some(name) = field_text(node, "name", source) {
                    add_symbol(graph, file_id, file_path, NodeKind::Trait, &name);
                }
            }
            "impl_item" => {
                if let Some(name) = impl_type_name(node, source) {
                    add_symbol(graph, file_id, file_path, NodeKind::Impl, &name);
                }
            }
            _ => {}
        }

        let mut cursor = node.walk();
        let children: Vec<_> = node.children(&mut cursor).collect();
        stack.extend(children.into_iter().rev());
    }
}

fn field_text(node: Node<'_>, field_name: &str, source: &[u8]) -> Option<String> {
    let field = node.child_by_field_name(field_name)?;
    field.utf8_text(source).ok().map(ToOwned::to_owned)
}

fn impl_type_name(node: Node<'_>, source: &[u8]) -> Option<String> {
    let ty = node.child_by_field_name("type")?;
    if let Some(found) = find_first_type_identifier(ty, source) {
        return Some(found);
    }
    ty.utf8_text(source).ok().map(ToOwned::to_owned)
}

fn find_first_type_identifier(node: Node<'_>, source: &[u8]) -> Option<String> {
    let mut stack = vec![node];
    while let Some(current) = stack.pop() {
        if matches!(
            current.kind(),
            "type_identifier" | "scoped_type_identifier" | "identifier"
        ) {
            return current.utf8_text(source).ok().map(ToOwned::to_owned);
        }

        let mut cursor = current.walk();
        let children: Vec<_> = current.children(&mut cursor).collect();
        stack.extend(children.into_iter().rev());
    }

    None
}

fn add_symbol(
    graph: &mut KnowledgeGraph,
    file_id: &str,
    file_path: &str,
    kind: NodeKind,
    name: &str,
) {
    let node_id = format!("sym:{}:{}:{}", file_path, kind_str(&kind), name);
    let node = GraphNode::new(&node_id, kind, name)
        .with_property("filePath", serde_json::Value::String(file_path.to_string()));
    graph.add_node(node);

    let rel_id = format!("contains:{}:{}", file_id, node_id);
    graph.add_relationship(GraphRelationship::new(
        rel_id,
        RelationshipKind::Contains,
        file_id,
        &node_id,
    ));
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
