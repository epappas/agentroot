//! AST-aware semantic chunking
//!
//! This module provides semantic chunking of source code files using tree-sitter
//! for AST parsing. It extracts functions, classes, methods, and other semantic
//! units while preserving context like docstrings and comments.

pub mod language;
pub mod oversized;
pub mod parser;
pub mod strategies;
pub mod types;

pub use language::{is_supported, Language};
pub use oversized::{split_oversized_chunk, split_oversized_chunks};
pub use strategies::{
    ChunkingStrategy, GoStrategy, JavaScriptStrategy, LanguageStrategy, PythonStrategy,
    RustStrategy,
};
pub use types::{compute_chunk_hash, ChunkMetadata, ChunkType, SemanticChunk};

use super::chunker::{chunk_by_chars, Chunk, CHUNK_OVERLAP_CHARS, CHUNK_SIZE_CHARS};
use crate::error::Result;
use std::path::Path;
use tracing::debug;

const MIN_CHUNK_CHARS: usize = 1;

/// Main semantic chunker that delegates to language-specific strategies
pub struct SemanticChunker {
    max_chunk_chars: usize,
}

impl Default for SemanticChunker {
    fn default() -> Self {
        Self::new()
    }
}

impl SemanticChunker {
    pub fn new() -> Self {
        Self {
            max_chunk_chars: CHUNK_SIZE_CHARS,
        }
    }

    pub fn with_max_chunk_chars(self, max: usize) -> Self {
        let max = if max < MIN_CHUNK_CHARS {
            MIN_CHUNK_CHARS
        } else {
            max
        };
        Self {
            max_chunk_chars: max,
        }
    }

    /// Chunk content semantically based on file path
    ///
    /// For supported languages, uses AST-based chunking.
    /// For unsupported languages, falls back to character-based chunking.
    pub fn chunk(&self, content: &str, path: &Path) -> Result<Vec<SemanticChunk>> {
        let language = match Language::from_path(path) {
            Some(lang) => lang,
            None => return self.fallback_chunk(content),
        };

        let tree = match parser::parse(content, language) {
            Ok(tree) => tree,
            Err(e) => {
                debug!(
                    error = %e,
                    path = %path.display(),
                    language = %language.as_str(),
                    "AST parse failed, falling back to character-based chunking"
                );
                return self.fallback_chunk(content);
            }
        };

        let strategy = LanguageStrategy::for_language(language);
        let chunks = strategy.extract_chunks(content, tree.root_node())?;
        let chunks = split_oversized_chunks(chunks, self.max_chunk_chars);

        Ok(chunks)
    }

    /// Fallback to character-based chunking for unsupported files
    fn fallback_chunk(&self, content: &str) -> Result<Vec<SemanticChunk>> {
        let char_chunks = chunk_by_chars(content, CHUNK_SIZE_CHARS, CHUNK_OVERLAP_CHARS);

        let semantic_chunks = char_chunks
            .into_iter()
            .map(|c| {
                let hash = compute_chunk_hash(&c.text, "", "");
                SemanticChunk {
                    text: c.text,
                    chunk_type: ChunkType::Text,
                    chunk_hash: hash,
                    position: c.position,
                    token_count: c.token_count,
                    metadata: ChunkMetadata::default(),
                }
            })
            .collect();

        Ok(semantic_chunks)
    }
}

/// Convenience function for semantic chunking
pub fn chunk_semantic(content: &str, path: &Path) -> Result<Vec<SemanticChunk>> {
    SemanticChunker::new().chunk(content, path)
}

/// Convert a SemanticChunk to a basic Chunk (for backwards compatibility)
impl From<SemanticChunk> for Chunk {
    fn from(sc: SemanticChunk) -> Self {
        Chunk {
            text: sc.text,
            position: sc.position,
            token_count: sc.token_count,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rust_file_chunking() {
        let content = r#"
/// A greeting function
fn hello() {
    println!("Hello, world!");
}

struct Point {
    x: i32,
    y: i32,
}
"#;
        let path = Path::new("test.rs");
        let chunks = chunk_semantic(content, path).unwrap();

        assert!(chunks.len() >= 2);
        assert!(chunks.iter().any(|c| c.chunk_type == ChunkType::Function));
        assert!(chunks.iter().any(|c| c.chunk_type == ChunkType::Struct));
    }

    #[test]
    fn test_python_file_chunking() {
        let content = r#"
def greet(name):
    """Greet someone."""
    print(f"Hello, {name}!")

class Greeter:
    def __init__(self):
        pass
"#;
        let path = Path::new("test.py");
        let chunks = chunk_semantic(content, path).unwrap();

        assert!(!chunks.is_empty());
    }

    #[test]
    fn test_markdown_fallback() {
        let content = "# Hello\n\nThis is markdown content.";
        let path = Path::new("test.md");
        let chunks = chunk_semantic(content, path).unwrap();

        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].chunk_type, ChunkType::Text);
    }

    #[test]
    fn test_chunk_hash_in_semantic_chunks() {
        let content = "fn test() {}";
        let path = Path::new("test.rs");
        let chunks = chunk_semantic(content, path).unwrap();

        for chunk in &chunks {
            assert_eq!(chunk.chunk_hash.len(), 32);
        }
    }

    #[test]
    fn test_semantic_to_basic_chunk_conversion() {
        let semantic = SemanticChunk::new("test".to_string(), ChunkType::Function, 0);
        let basic: Chunk = semantic.into();

        assert_eq!(basic.text, "test");
        assert_eq!(basic.position, 0);
    }

    #[test]
    fn test_with_max_chunk_chars_validation() {
        let chunker = SemanticChunker::new().with_max_chunk_chars(0);
        assert_eq!(chunker.max_chunk_chars, MIN_CHUNK_CHARS);

        let chunker = SemanticChunker::new().with_max_chunk_chars(500);
        assert_eq!(chunker.max_chunk_chars, 500);
    }
}
