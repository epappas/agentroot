//! Language-specific chunking strategies

mod go;
mod javascript;
mod python;
mod rust;

pub use go::GoStrategy;
pub use javascript::JavaScriptStrategy;
pub use python::PythonStrategy;
pub use rust::RustStrategy;

use super::language::Language;
use super::types::{ChunkType, SemanticChunk};
use crate::error::Result;
use tree_sitter::Node;

/// Trait for language-specific semantic chunking
pub trait ChunkingStrategy: Send + Sync {
    /// Get the node types that represent semantic boundaries
    fn semantic_node_types(&self) -> &[&str];

    /// Extract chunks from the source given the AST root
    fn extract_chunks(&self, source: &str, root: Node) -> Result<Vec<SemanticChunk>>;

    /// Determine chunk type from AST node
    fn chunk_type_for_node(&self, node: Node) -> ChunkType;

    /// Extract leading trivia (comments/docs) for a node
    fn extract_leading_trivia(&self, source: &str, node: Node) -> String {
        extract_leading_comments(source, node)
    }

    /// Extract trailing trivia for a node
    fn extract_trailing_trivia(&self, source: &str, node: Node) -> String {
        extract_trailing_comment(source, node)
    }
}

/// Enum-based strategy dispatch to avoid heap allocation
pub enum LanguageStrategy {
    Rust(RustStrategy),
    Python(PythonStrategy),
    JavaScript(JavaScriptStrategy),
    Go(GoStrategy),
}

impl LanguageStrategy {
    pub fn for_language(language: Language) -> Self {
        match language {
            Language::Rust => Self::Rust(RustStrategy),
            Language::Python => Self::Python(PythonStrategy),
            Language::JavaScript => Self::JavaScript(JavaScriptStrategy::javascript()),
            Language::TypeScript | Language::TypeScriptTsx => {
                Self::JavaScript(JavaScriptStrategy::typescript())
            }
            Language::Go => Self::Go(GoStrategy),
        }
    }

    pub fn extract_chunks(&self, source: &str, root: Node) -> Result<Vec<SemanticChunk>> {
        match self {
            Self::Rust(s) => s.extract_chunks(source, root),
            Self::Python(s) => s.extract_chunks(source, root),
            Self::JavaScript(s) => s.extract_chunks(source, root),
            Self::Go(s) => s.extract_chunks(source, root),
        }
    }
}

/// Extract leading comments/docs above a node
pub fn extract_leading_comments(source: &str, node: Node) -> String {
    let start_byte = node.start_byte();
    if start_byte == 0 {
        return String::new();
    }

    let preceding = &source[..start_byte];
    let lines: Vec<&str> = preceding.lines().rev().collect();
    let mut trivia_lines = Vec::new();

    for line in lines {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if !trivia_lines.is_empty() {
                break;
            }
            continue;
        }
        if is_comment_line(trimmed) {
            trivia_lines.push(line);
        } else {
            break;
        }
    }

    trivia_lines.reverse();
    if trivia_lines.is_empty() {
        String::new()
    } else {
        trivia_lines.join("\n")
    }
}

/// Check if a line is a comment
fn is_comment_line(line: &str) -> bool {
    line.starts_with("//")
        || line.starts_with('#')
        || line.starts_with("/*")
        || line.starts_with('*')
        || line.starts_with("*/")
        || line.starts_with("///")
        || line.starts_with("//!")
        || line.starts_with("\"\"\"")
        || line.starts_with("'''")
}

/// Extract trailing comment on the same line
pub fn extract_trailing_comment(source: &str, node: Node) -> String {
    let end_byte = node.end_byte();
    if end_byte >= source.len() {
        return String::new();
    }

    let following = &source[end_byte..];
    if let Some(line_end) = following.find('\n') {
        let same_line = following[..line_end].trim();
        if same_line.starts_with("//") || same_line.starts_with('#') {
            return same_line.to_string();
        }
    }
    String::new()
}

/// Compute line numbers for a byte range
pub fn line_numbers(source: &str, start_byte: usize, end_byte: usize) -> (usize, usize) {
    let start_line = source[..start_byte].matches('\n').count() + 1;
    let end_line = source[..end_byte].matches('\n').count() + 1;
    (start_line, end_line)
}

/// Get breadcrumb path for a node (e.g., "ClassName::method_name")
pub fn get_breadcrumb(source: &str, node: Node) -> Option<String> {
    let mut parts = Vec::new();
    let mut current = Some(node);

    while let Some(n) = current {
        if let Some(name) = extract_name_from_node(source, n) {
            parts.push(name);
        }
        current = n.parent();
    }

    if parts.is_empty() {
        None
    } else {
        parts.reverse();
        Some(parts.join("::"))
    }
}

/// Extract name identifier from a node
fn extract_name_from_node(source: &str, node: Node) -> Option<String> {
    let kind = node.kind();
    let name_field = match kind {
        "function_item"
        | "function_definition"
        | "function_declaration"
        | "method_definition"
        | "method_declaration" => "name",
        "impl_item" => "type",
        "struct_item" | "class_definition" | "class_declaration" => "name",
        "enum_item" | "type_declaration" => "name",
        "trait_item" | "interface_declaration" => "name",
        _ => return None,
    };

    node.child_by_field_name(name_field)
        .map(|n| source[n.start_byte()..n.end_byte()].to_string())
}
