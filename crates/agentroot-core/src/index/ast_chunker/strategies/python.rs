//! Python-specific chunking strategy

use tree_sitter::Node;
use super::{ChunkingStrategy, line_numbers, get_breadcrumb};
use crate::index::ast_chunker::types::{SemanticChunk, ChunkType, ChunkMetadata, compute_chunk_hash};
use crate::error::Result;

const PYTHON_SEMANTIC_NODES: &[&str] = &[
    "function_definition",
    "class_definition",
    "decorated_definition",
];

pub struct PythonStrategy;

impl ChunkingStrategy for PythonStrategy {
    fn semantic_node_types(&self) -> &[&str] {
        PYTHON_SEMANTIC_NODES
    }

    fn extract_chunks(&self, source: &str, root: Node) -> Result<Vec<SemanticChunk>> {
        let mut chunks = Vec::new();
        let mut cursor = root.walk();
        extract_python_chunks(source, &mut cursor, &mut chunks, self, None);

        if chunks.is_empty() {
            chunks.push(SemanticChunk::new(source.to_string(), ChunkType::Text, 0));
        }

        Ok(chunks)
    }

    fn chunk_type_for_node(&self, node: Node) -> ChunkType {
        match node.kind() {
            "function_definition" => ChunkType::Function,
            "class_definition" => ChunkType::Class,
            "decorated_definition" => {
                if let Some(inner) = get_decorated_inner(node) {
                    match inner.kind() {
                        "function_definition" => ChunkType::Function,
                        "class_definition" => ChunkType::Class,
                        _ => ChunkType::Function,
                    }
                } else {
                    ChunkType::Function
                }
            }
            _ => ChunkType::Text,
        }
    }

    fn extract_leading_trivia(&self, source: &str, node: Node) -> String {
        let mut trivia = super::extract_leading_comments(source, node);

        if let Some(docstring) = extract_docstring(source, node) {
            if !trivia.is_empty() {
                trivia.push('\n');
            }
            trivia.push_str(&docstring);
        }

        trivia
    }
}

fn extract_python_chunks(
    source: &str,
    cursor: &mut tree_sitter::TreeCursor,
    chunks: &mut Vec<SemanticChunk>,
    strategy: &PythonStrategy,
    parent_class: Option<&str>,
) {
    loop {
        let node = cursor.node();
        let kind = node.kind();

        if PYTHON_SEMANTIC_NODES.contains(&kind) {
            let actual_node = if kind == "decorated_definition" {
                get_decorated_inner(node).unwrap_or(node)
            } else {
                node
            };

            let leading = strategy.extract_leading_trivia(source, node);
            let trailing = strategy.extract_trailing_trivia(source, node);
            let text = source[node.start_byte()..node.end_byte()].to_string();
            let (start_line, end_line) = line_numbers(source, node.start_byte(), node.end_byte());

            let name = actual_node.child_by_field_name("name")
                .map(|n| source[n.start_byte()..n.end_byte()].to_string());

            let breadcrumb = match (parent_class, &name) {
                (Some(cls), Some(n)) => Some(format!("{}::{}", cls, n)),
                (None, Some(n)) => Some(n.clone()),
                _ => get_breadcrumb(source, node),
            };

            let chunk_type = if parent_class.is_some() && actual_node.kind() == "function_definition" {
                ChunkType::Method
            } else {
                strategy.chunk_type_for_node(node)
            };

            let chunk_hash = compute_chunk_hash(&text, &leading, &trailing);

            let chunk = SemanticChunk {
                text,
                chunk_type,
                chunk_hash,
                position: node.start_byte(),
                token_count: None,
                metadata: ChunkMetadata {
                    leading_trivia: leading,
                    trailing_trivia: trailing,
                    breadcrumb,
                    language: Some("python"),
                    start_line,
                    end_line,
                },
            };
            chunks.push(chunk);

            if actual_node.kind() == "class_definition" {
                let class_name = name.as_deref();
                if cursor.goto_first_child() {
                    extract_python_chunks(source, cursor, chunks, strategy, class_name);
                    cursor.goto_parent();
                }
            }
        } else if cursor.goto_first_child() {
            extract_python_chunks(source, cursor, chunks, strategy, parent_class);
            cursor.goto_parent();
        }

        if !cursor.goto_next_sibling() {
            break;
        }
    }
}

fn get_decorated_inner(node: Node) -> Option<Node> {
    node.child_by_field_name("definition")
}

fn extract_docstring(source: &str, node: Node) -> Option<String> {
    let body = match node.kind() {
        "function_definition" | "class_definition" => node.child_by_field_name("body"),
        "decorated_definition" => {
            get_decorated_inner(node).and_then(|n| n.child_by_field_name("body"))
        }
        _ => None,
    }?;

    let mut cursor = body.walk();
    if cursor.goto_first_child() {
        let first = cursor.node();
        if first.kind() == "expression_statement" {
            if let Some(string) = first.child(0) {
                if string.kind() == "string" {
                    return Some(source[string.start_byte()..string.end_byte()].to_string());
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::ast_chunker::parser::parse;
    use crate::index::ast_chunker::language::Language;

    #[test]
    fn test_extract_function() {
        let source = r#"
def hello():
    """Say hello."""
    print("hello")
"#;
        let tree = parse(source, Language::Python).unwrap();
        let strategy = PythonStrategy;
        let chunks = strategy.extract_chunks(source, tree.root_node()).unwrap();

        assert!(!chunks.is_empty());
        assert!(chunks.iter().any(|c| c.chunk_type == ChunkType::Function));
    }

    #[test]
    fn test_extract_class() {
        let source = r#"
class MyClass:
    """A class."""

    def method(self):
        pass
"#;
        let tree = parse(source, Language::Python).unwrap();
        let strategy = PythonStrategy;
        let chunks = strategy.extract_chunks(source, tree.root_node()).unwrap();

        assert!(chunks.iter().any(|c| c.chunk_type == ChunkType::Class));
        assert!(chunks.iter().any(|c| c.chunk_type == ChunkType::Method));
    }

    #[test]
    fn test_extract_decorated() {
        let source = r#"
@decorator
def decorated_fn():
    pass
"#;
        let tree = parse(source, Language::Python).unwrap();
        let strategy = PythonStrategy;
        let chunks = strategy.extract_chunks(source, tree.root_node()).unwrap();

        assert!(!chunks.is_empty());
    }
}
