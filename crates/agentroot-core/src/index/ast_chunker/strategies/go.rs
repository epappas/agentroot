//! Go-specific chunking strategy

use tree_sitter::Node;
use super::{ChunkingStrategy, line_numbers, get_breadcrumb};
use crate::index::ast_chunker::types::{SemanticChunk, ChunkType, ChunkMetadata, compute_chunk_hash};
use crate::error::Result;

const GO_SEMANTIC_NODES: &[&str] = &[
    "function_declaration",
    "method_declaration",
    "type_declaration",
    "const_declaration",
    "var_declaration",
];

pub struct GoStrategy;

impl ChunkingStrategy for GoStrategy {
    fn semantic_node_types(&self) -> &[&str] {
        GO_SEMANTIC_NODES
    }

    fn extract_chunks(&self, source: &str, root: Node) -> Result<Vec<SemanticChunk>> {
        let mut chunks = Vec::new();
        let mut cursor = root.walk();
        extract_go_chunks(source, &mut cursor, &mut chunks, self);

        if chunks.is_empty() {
            chunks.push(SemanticChunk::new(source.to_string(), ChunkType::Text, 0));
        }

        Ok(chunks)
    }

    fn chunk_type_for_node(&self, node: Node) -> ChunkType {
        match node.kind() {
            "function_declaration" => ChunkType::Function,
            "method_declaration" => ChunkType::Method,
            "type_declaration" => {
                if has_struct_type(node) {
                    ChunkType::Struct
                } else if has_interface_type(node) {
                    ChunkType::Interface
                } else {
                    ChunkType::Struct
                }
            }
            _ => ChunkType::Text,
        }
    }
}

fn extract_go_chunks(
    source: &str,
    cursor: &mut tree_sitter::TreeCursor,
    chunks: &mut Vec<SemanticChunk>,
    strategy: &GoStrategy,
) {
    loop {
        let node = cursor.node();
        let kind = node.kind();

        if GO_SEMANTIC_NODES.contains(&kind) {
            let leading = strategy.extract_leading_trivia(source, node);
            let trailing = strategy.extract_trailing_trivia(source, node);
            let text = source[node.start_byte()..node.end_byte()].to_string();
            let (start_line, end_line) = line_numbers(source, node.start_byte(), node.end_byte());

            let name = get_go_name(source, node);
            let receiver = get_method_receiver(source, node);

            let breadcrumb = match (&receiver, &name) {
                (Some(r), Some(n)) => Some(format!("{}::{}", r, n)),
                (None, Some(n)) => Some(n.clone()),
                _ => get_breadcrumb(source, node),
            };

            let chunk_hash = compute_chunk_hash(&text, &leading, &trailing);

            let chunk = SemanticChunk {
                text,
                chunk_type: strategy.chunk_type_for_node(node),
                chunk_hash,
                position: node.start_byte(),
                token_count: None,
                metadata: ChunkMetadata {
                    leading_trivia: leading,
                    trailing_trivia: trailing,
                    breadcrumb,
                    language: Some("go"),
                    start_line,
                    end_line,
                },
            };
            chunks.push(chunk);
        } else if cursor.goto_first_child() {
            extract_go_chunks(source, cursor, chunks, strategy);
            cursor.goto_parent();
        }

        if !cursor.goto_next_sibling() {
            break;
        }
    }
}

fn get_go_name(source: &str, node: Node) -> Option<String> {
    if let Some(name_node) = node.child_by_field_name("name") {
        return Some(source[name_node.start_byte()..name_node.end_byte()].to_string());
    }

    if node.kind() == "type_declaration" {
        let mut cursor = node.walk();
        if cursor.goto_first_child() {
            loop {
                let child = cursor.node();
                if child.kind() == "type_spec" {
                    if let Some(name) = child.child_by_field_name("name") {
                        return Some(source[name.start_byte()..name.end_byte()].to_string());
                    }
                }
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }
    }

    None
}

fn get_method_receiver(source: &str, node: Node) -> Option<String> {
    if node.kind() != "method_declaration" {
        return None;
    }

    let receiver = node.child_by_field_name("receiver")?;
    let mut cursor = receiver.walk();

    if cursor.goto_first_child() {
        loop {
            let child = cursor.node();
            if child.kind() == "parameter_declaration" {
                if let Some(type_node) = child.child_by_field_name("type") {
                    let type_str = source[type_node.start_byte()..type_node.end_byte()].to_string();
                    return Some(type_str.trim_start_matches('*').to_string());
                }
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }

    None
}

fn has_struct_type(node: Node) -> bool {
    contains_node_kind(node, "struct_type")
}

fn has_interface_type(node: Node) -> bool {
    contains_node_kind(node, "interface_type")
}

fn contains_node_kind(node: Node, target_kind: &str) -> bool {
    let mut cursor = node.walk();
    search_for_kind(&mut cursor, target_kind)
}

fn search_for_kind(cursor: &mut tree_sitter::TreeCursor, target_kind: &str) -> bool {
    if cursor.node().kind() == target_kind {
        return true;
    }
    if cursor.goto_first_child() {
        loop {
            if search_for_kind(cursor, target_kind) {
                cursor.goto_parent();
                return true;
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
        cursor.goto_parent();
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::ast_chunker::parser::parse;
    use crate::index::ast_chunker::language::Language;

    #[test]
    fn test_extract_function() {
        let source = r#"
package main

// Hello says hello
func Hello() {
    fmt.Println("hello")
}
"#;
        let tree = parse(source, Language::Go).unwrap();
        let strategy = GoStrategy;
        let chunks = strategy.extract_chunks(source, tree.root_node()).unwrap();

        assert!(!chunks.is_empty());
        assert!(chunks.iter().any(|c| c.chunk_type == ChunkType::Function));
    }

    #[test]
    fn test_extract_method() {
        let source = r#"
package main

func (s *Server) Start() error {
    return nil
}
"#;
        let tree = parse(source, Language::Go).unwrap();
        let strategy = GoStrategy;
        let chunks = strategy.extract_chunks(source, tree.root_node()).unwrap();

        assert!(chunks.iter().any(|c| c.chunk_type == ChunkType::Method));
        let method = chunks.iter().find(|c| c.chunk_type == ChunkType::Method).unwrap();
        assert!(method.metadata.breadcrumb.as_ref().unwrap().contains("Server"));
    }

    #[test]
    fn test_extract_struct() {
        let source = r#"
package main

type Server struct {
    Port int
}
"#;
        let tree = parse(source, Language::Go).unwrap();
        let strategy = GoStrategy;
        let chunks = strategy.extract_chunks(source, tree.root_node()).unwrap();

        assert!(chunks.iter().any(|c| c.chunk_type == ChunkType::Struct));
    }

    #[test]
    fn test_extract_interface() {
        let source = r#"
package main

type Handler interface {
    Handle() error
}
"#;
        let tree = parse(source, Language::Go).unwrap();
        let strategy = GoStrategy;
        let chunks = strategy.extract_chunks(source, tree.root_node()).unwrap();

        assert!(chunks.iter().any(|c| c.chunk_type == ChunkType::Interface));
    }
}
