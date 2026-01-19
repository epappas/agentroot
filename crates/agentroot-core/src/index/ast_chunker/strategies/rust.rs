//! Rust-specific chunking strategy

use tree_sitter::Node;
use super::{ChunkingStrategy, line_numbers, get_breadcrumb};
use crate::index::ast_chunker::types::{SemanticChunk, ChunkType, ChunkMetadata, compute_chunk_hash};
use crate::error::Result;

const RUST_SEMANTIC_NODES: &[&str] = &[
    "function_item",
    "impl_item",
    "struct_item",
    "enum_item",
    "trait_item",
    "mod_item",
    "type_item",
    "const_item",
    "static_item",
    "macro_definition",
];

pub struct RustStrategy;

impl ChunkingStrategy for RustStrategy {
    fn semantic_node_types(&self) -> &[&str] {
        RUST_SEMANTIC_NODES
    }

    fn extract_chunks(&self, source: &str, root: Node) -> Result<Vec<SemanticChunk>> {
        let mut chunks = Vec::new();
        let mut cursor = root.walk();
        extract_rust_chunks(source, &mut cursor, &mut chunks, self);

        if chunks.is_empty() {
            chunks.push(SemanticChunk::new(source.to_string(), ChunkType::Text, 0));
        }

        Ok(chunks)
    }

    fn chunk_type_for_node(&self, node: Node) -> ChunkType {
        match node.kind() {
            "function_item" => ChunkType::Function,
            "impl_item" => {
                if has_child_kind(node, "trait") {
                    ChunkType::Trait
                } else {
                    ChunkType::Method
                }
            }
            "struct_item" => ChunkType::Struct,
            "enum_item" => ChunkType::Enum,
            "trait_item" => ChunkType::Trait,
            "mod_item" => ChunkType::Module,
            _ => ChunkType::Function,
        }
    }
}

fn extract_rust_chunks(
    source: &str,
    cursor: &mut tree_sitter::TreeCursor,
    chunks: &mut Vec<SemanticChunk>,
    strategy: &RustStrategy,
) {
    loop {
        let node = cursor.node();
        let kind = node.kind();

        if RUST_SEMANTIC_NODES.contains(&kind) {
            let leading = strategy.extract_leading_trivia(source, node);
            let trailing = strategy.extract_trailing_trivia(source, node);
            let text = source[node.start_byte()..node.end_byte()].to_string();
            let (start_line, end_line) = line_numbers(source, node.start_byte(), node.end_byte());
            let breadcrumb = get_breadcrumb(source, node);
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
                    language: Some("rust"),
                    start_line,
                    end_line,
                },
            };
            chunks.push(chunk);

            if kind == "impl_item" && cursor.goto_first_child() {
                extract_rust_chunks(source, cursor, chunks, strategy);
                cursor.goto_parent();
            }
        } else if cursor.goto_first_child() {
            extract_rust_chunks(source, cursor, chunks, strategy);
            cursor.goto_parent();
        }

        if !cursor.goto_next_sibling() {
            break;
        }
    }
}

fn has_child_kind(node: Node, kind: &str) -> bool {
    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            if cursor.node().kind() == kind {
                return true;
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
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
/// A test function
fn test_fn() {
    println!("hello");
}
"#;
        let tree = parse(source, Language::Rust).unwrap();
        let strategy = RustStrategy;
        let chunks = strategy.extract_chunks(source, tree.root_node()).unwrap();

        assert!(!chunks.is_empty());
        assert!(chunks.iter().any(|c| c.chunk_type == ChunkType::Function));
    }

    #[test]
    fn test_extract_struct() {
        let source = r#"
/// My struct
struct MyStruct {
    field: i32,
}
"#;
        let tree = parse(source, Language::Rust).unwrap();
        let strategy = RustStrategy;
        let chunks = strategy.extract_chunks(source, tree.root_node()).unwrap();

        assert!(chunks.iter().any(|c| c.chunk_type == ChunkType::Struct));
    }

    #[test]
    fn test_extract_impl() {
        let source = r#"
impl MyStruct {
    fn new() -> Self {
        Self { field: 0 }
    }
}
"#;
        let tree = parse(source, Language::Rust).unwrap();
        let strategy = RustStrategy;
        let chunks = strategy.extract_chunks(source, tree.root_node()).unwrap();

        assert!(!chunks.is_empty());
    }
}
