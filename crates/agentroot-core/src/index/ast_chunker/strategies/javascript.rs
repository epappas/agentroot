//! JavaScript/TypeScript-specific chunking strategy

use tree_sitter::Node;
use super::{ChunkingStrategy, line_numbers, get_breadcrumb};
use crate::index::ast_chunker::types::{SemanticChunk, ChunkType, ChunkMetadata, compute_chunk_hash};
use crate::error::Result;

const JS_SEMANTIC_NODES: &[&str] = &[
    "function_declaration",
    "class_declaration",
    "method_definition",
    "arrow_function",
    "function_expression",
    "export_statement",
    "interface_declaration",
    "type_alias_declaration",
    "enum_declaration",
];

pub struct JavaScriptStrategy {
    pub is_typescript: bool,
}

impl JavaScriptStrategy {
    pub fn javascript() -> Self {
        Self { is_typescript: false }
    }

    pub fn typescript() -> Self {
        Self { is_typescript: true }
    }
}

impl ChunkingStrategy for JavaScriptStrategy {
    fn semantic_node_types(&self) -> &[&str] {
        JS_SEMANTIC_NODES
    }

    fn extract_chunks(&self, source: &str, root: Node) -> Result<Vec<SemanticChunk>> {
        let mut chunks = Vec::new();
        let mut cursor = root.walk();
        extract_js_chunks(source, &mut cursor, &mut chunks, self, None);

        if chunks.is_empty() {
            chunks.push(SemanticChunk::new(source.to_string(), ChunkType::Text, 0));
        }

        Ok(chunks)
    }

    fn chunk_type_for_node(&self, node: Node) -> ChunkType {
        match node.kind() {
            "function_declaration" | "function_expression" | "arrow_function" => ChunkType::Function,
            "class_declaration" => ChunkType::Class,
            "method_definition" => ChunkType::Method,
            "interface_declaration" => ChunkType::Interface,
            "type_alias_declaration" => ChunkType::Struct,
            "enum_declaration" => ChunkType::Enum,
            "export_statement" => {
                if let Some(decl) = get_exported_declaration(node) {
                    self.chunk_type_for_node(decl)
                } else {
                    ChunkType::Module
                }
            }
            _ => ChunkType::Text,
        }
    }
}

fn extract_js_chunks(
    source: &str,
    cursor: &mut tree_sitter::TreeCursor,
    chunks: &mut Vec<SemanticChunk>,
    strategy: &JavaScriptStrategy,
    parent_class: Option<&str>,
) {
    loop {
        let node = cursor.node();
        let kind = node.kind();

        let is_semantic = JS_SEMANTIC_NODES.contains(&kind);
        let is_var_with_fn = is_variable_with_function(node);

        if is_semantic || is_var_with_fn {
            let actual_node = if kind == "export_statement" {
                get_exported_declaration(node).unwrap_or(node)
            } else {
                node
            };

            let leading = strategy.extract_leading_trivia(source, node);
            let trailing = strategy.extract_trailing_trivia(source, node);
            let text = source[node.start_byte()..node.end_byte()].to_string();
            let (start_line, end_line) = line_numbers(source, node.start_byte(), node.end_byte());

            let name = get_js_name(source, actual_node);
            let breadcrumb = match (parent_class, &name) {
                (Some(cls), Some(n)) => Some(format!("{}::{}", cls, n)),
                (None, Some(n)) => Some(n.clone()),
                _ => get_breadcrumb(source, node),
            };

            let chunk_type = if parent_class.is_some() {
                ChunkType::Method
            } else if is_var_with_fn {
                ChunkType::Function
            } else {
                strategy.chunk_type_for_node(node)
            };

            let chunk_hash = compute_chunk_hash(&text, &leading, &trailing);
            let lang: &'static str = if strategy.is_typescript { "typescript" } else { "javascript" };

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
                    language: Some(lang),
                    start_line,
                    end_line,
                },
            };
            chunks.push(chunk);

            if actual_node.kind() == "class_declaration" || actual_node.kind() == "class" {
                let class_name = name.as_deref();
                if cursor.goto_first_child() {
                    extract_js_chunks(source, cursor, chunks, strategy, class_name);
                    cursor.goto_parent();
                }
            }
        } else if cursor.goto_first_child() {
            extract_js_chunks(source, cursor, chunks, strategy, parent_class);
            cursor.goto_parent();
        }

        if !cursor.goto_next_sibling() {
            break;
        }
    }
}

fn get_exported_declaration(node: Node) -> Option<Node> {
    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            let child = cursor.node();
            let k = child.kind();
            if k != "export" && k != "default" && !k.contains("comment") {
                return Some(child);
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
    None
}

fn get_js_name(source: &str, node: Node) -> Option<String> {
    if let Some(name_node) = node.child_by_field_name("name") {
        return Some(source[name_node.start_byte()..name_node.end_byte()].to_string());
    }

    if node.kind() == "lexical_declaration" || node.kind() == "variable_declaration" {
        let mut cursor = node.walk();
        if cursor.goto_first_child() {
            loop {
                let child = cursor.node();
                if child.kind() == "variable_declarator" {
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

fn is_variable_with_function(node: Node) -> bool {
    let kind = node.kind();
    if kind != "lexical_declaration" && kind != "variable_declaration" {
        return false;
    }

    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            let child = cursor.node();
            if child.kind() == "variable_declarator" {
                if let Some(value) = child.child_by_field_name("value") {
                    let vk = value.kind();
                    if vk == "arrow_function" || vk == "function_expression" || vk == "function" {
                        return true;
                    }
                }
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
function hello() {
    console.log("hello");
}
"#;
        let tree = parse(source, Language::JavaScript).unwrap();
        let strategy = JavaScriptStrategy::javascript();
        let chunks = strategy.extract_chunks(source, tree.root_node()).unwrap();

        assert!(!chunks.is_empty());
        assert!(chunks.iter().any(|c| c.chunk_type == ChunkType::Function));
    }

    #[test]
    fn test_extract_class() {
        let source = r#"
class MyClass {
    constructor() {}
    method() {}
}
"#;
        let tree = parse(source, Language::JavaScript).unwrap();
        let strategy = JavaScriptStrategy::javascript();
        let chunks = strategy.extract_chunks(source, tree.root_node()).unwrap();

        assert!(chunks.iter().any(|c| c.chunk_type == ChunkType::Class));
    }

    #[test]
    fn test_extract_arrow_function() {
        let source = r#"
const myFunc = () => {
    return 42;
};
"#;
        let tree = parse(source, Language::JavaScript).unwrap();
        let strategy = JavaScriptStrategy::javascript();
        let chunks = strategy.extract_chunks(source, tree.root_node()).unwrap();

        assert!(!chunks.is_empty());
    }

    #[test]
    fn test_extract_typescript_interface() {
        let source = r#"
interface User {
    name: string;
    age: number;
}
"#;
        let tree = parse(source, Language::TypeScript).unwrap();
        let strategy = JavaScriptStrategy::typescript();
        let chunks = strategy.extract_chunks(source, tree.root_node()).unwrap();

        assert!(chunks.iter().any(|c| c.chunk_type == ChunkType::Interface));
    }
}
