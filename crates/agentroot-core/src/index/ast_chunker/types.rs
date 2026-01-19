//! Core types for AST-aware semantic chunking

use serde::{Deserialize, Serialize};

/// Type of semantic chunk extracted from source code or text
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChunkType {
    Function,
    Method,
    Class,
    Struct,
    Enum,
    Trait,
    Interface,
    Module,
    Import,
    Text,
}

impl ChunkType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Function => "function",
            Self::Method => "method",
            Self::Class => "class",
            Self::Struct => "struct",
            Self::Enum => "enum",
            Self::Trait => "trait",
            Self::Interface => "interface",
            Self::Module => "module",
            Self::Import => "import",
            Self::Text => "text",
        }
    }
}

/// Metadata associated with a chunk
#[derive(Debug, Clone, Default)]
pub struct ChunkMetadata {
    /// Comments/docs above the chunk
    pub leading_trivia: String,
    /// Comments after the chunk
    pub trailing_trivia: String,
    /// Hierarchical path (e.g., "MyClass::my_method")
    pub breadcrumb: Option<String>,
    /// Source language (static string to avoid heap allocation per chunk)
    pub language: Option<&'static str>,
    /// Starting line number (1-indexed)
    pub start_line: usize,
    /// Ending line number (1-indexed)
    pub end_line: usize,
}

/// A semantic chunk of source code or text
#[derive(Debug, Clone)]
pub struct SemanticChunk {
    /// The chunk text content
    pub text: String,
    /// Type of semantic unit
    pub chunk_type: ChunkType,
    /// blake3 hash for cache invalidation
    pub chunk_hash: String,
    /// Byte position in source
    pub position: usize,
    /// Token count (if computed)
    pub token_count: Option<usize>,
    /// Additional metadata
    pub metadata: ChunkMetadata,
}

impl SemanticChunk {
    pub fn new(text: String, chunk_type: ChunkType, position: usize) -> Self {
        let chunk_hash = compute_chunk_hash(&text, "", "");
        Self {
            text,
            chunk_type,
            chunk_hash,
            position,
            token_count: None,
            metadata: ChunkMetadata::default(),
        }
    }

    pub fn with_context(
        text: String,
        chunk_type: ChunkType,
        position: usize,
        leading: &str,
        trailing: &str,
    ) -> Self {
        let chunk_hash = compute_chunk_hash(&text, leading, trailing);
        Self {
            text,
            chunk_type,
            chunk_hash,
            position,
            token_count: None,
            metadata: ChunkMetadata {
                leading_trivia: leading.to_string(),
                trailing_trivia: trailing.to_string(),
                ..Default::default()
            },
        }
    }

    pub fn with_metadata(mut self, metadata: ChunkMetadata) -> Self {
        self.chunk_hash = compute_chunk_hash(
            &self.text,
            &metadata.leading_trivia,
            &metadata.trailing_trivia,
        );
        self.metadata = metadata;
        self
    }
}

/// Compute blake3 hash for a chunk including its context
pub fn compute_chunk_hash(text: &str, leading: &str, trailing: &str) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(leading.as_bytes());
    hasher.update(text.as_bytes());
    hasher.update(trailing.as_bytes());
    let hash = hasher.finalize();
    hash.to_hex()[..32].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_hash_stability() {
        let hash1 = compute_chunk_hash("fn foo() {}", "", "");
        let hash2 = compute_chunk_hash("fn foo() {}", "", "");
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_chunk_hash_context_matters() {
        let hash1 = compute_chunk_hash("fn foo() {}", "// doc", "");
        let hash2 = compute_chunk_hash("fn foo() {}", "", "");
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_chunk_hash_length() {
        let hash = compute_chunk_hash("test", "", "");
        assert_eq!(hash.len(), 32);
    }

    #[test]
    fn test_semantic_chunk_creation() {
        let chunk = SemanticChunk::new("fn test() {}".to_string(), ChunkType::Function, 0);
        assert_eq!(chunk.chunk_type, ChunkType::Function);
        assert_eq!(chunk.position, 0);
        assert_eq!(chunk.chunk_hash.len(), 32);
    }
}
