//! Metadata generation for documents

use crate::error::Result;
use crate::index::ast_chunker::ChunkType;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Metadata generation trait
#[async_trait]
pub trait MetadataGenerator: Send + Sync {
    /// Generate comprehensive metadata for a document
    async fn generate_metadata(
        &self,
        content: &str,
        context: &MetadataContext,
    ) -> Result<DocumentMetadata>;

    /// Get model name
    fn model_name(&self) -> &str;

    /// Get LLM client for additional operations (e.g., chunk metadata)
    fn llm_client(&self) -> Option<&dyn crate::llm::LLMClient>;
}

/// Context information for metadata generation
#[derive(Debug, Clone)]
pub struct MetadataContext {
    /// Provider type (file, github, url, pdf, sql)
    pub source_type: String,
    /// Programming language (if applicable)
    pub language: Option<String>,
    /// File extension (if applicable)
    pub file_extension: Option<String>,
    /// Collection name
    pub collection_name: String,
    /// Provider configuration (JSON)
    pub provider_config: Option<String>,
    /// Document creation timestamp
    pub created_at: String,
    /// Document modification timestamp
    pub modified_at: String,
    /// AST chunk types found in document
    pub existing_structure: Option<Vec<ChunkType>>,
}

impl MetadataContext {
    /// Create new metadata context
    pub fn new(source_type: String, collection_name: String) -> Self {
        Self {
            source_type,
            collection_name,
            language: None,
            file_extension: None,
            provider_config: None,
            created_at: String::new(),
            modified_at: String::new(),
            existing_structure: None,
        }
    }

    /// Set language
    pub fn with_language(mut self, language: String) -> Self {
        self.language = Some(language);
        self
    }

    /// Set file extension
    pub fn with_extension(mut self, extension: String) -> Self {
        self.file_extension = Some(extension);
        self
    }

    /// Set provider config
    pub fn with_provider_config(mut self, config: String) -> Self {
        self.provider_config = Some(config);
        self
    }

    /// Set timestamps
    pub fn with_timestamps(mut self, created_at: String, modified_at: String) -> Self {
        self.created_at = created_at;
        self.modified_at = modified_at;
        self
    }

    /// Set existing structure
    pub fn with_structure(mut self, structure: Vec<ChunkType>) -> Self {
        self.existing_structure = Some(structure);
        self
    }
}

/// Extracted concept for glossary
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExtractedConcept {
    /// Canonical concept term (normalized)
    pub term: String,
    /// Snippet showing usage (~100 chars)
    pub snippet: String,
}

/// Generated metadata result
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DocumentMetadata {
    /// Document summary (100-200 words)
    pub summary: String,
    /// Semantic title (improved from filename)
    pub semantic_title: String,
    /// Keywords for search (5-10 terms)
    pub keywords: Vec<String>,
    /// Document category (tutorial, reference, config, etc.)
    pub category: String,
    /// Purpose/intent description
    pub intent: String,
    /// Related concepts/entities
    pub concepts: Vec<String>,
    /// Difficulty level (beginner, intermediate, advanced)
    pub difficulty: String,
    /// Suggested search queries
    pub suggested_queries: Vec<String>,
    /// Extracted concepts for intelligent glossary
    #[serde(default)]
    pub extracted_concepts: Vec<ExtractedConcept>,
}

impl DocumentMetadata {
    /// Create new empty metadata
    pub fn new() -> Self {
        Self {
            summary: String::new(),
            semantic_title: String::new(),
            keywords: Vec::new(),
            category: String::new(),
            intent: String::new(),
            concepts: Vec::new(),
            difficulty: String::new(),
            suggested_queries: Vec::new(),
            extracted_concepts: Vec::new(),
        }
    }

    /// Create metadata with basic fields
    pub fn basic(title: String, summary: String) -> Self {
        Self {
            summary,
            semantic_title: title,
            keywords: Vec::new(),
            category: "unknown".to_string(),
            intent: String::new(),
            concepts: Vec::new(),
            difficulty: "intermediate".to_string(),
            suggested_queries: Vec::new(),
            extracted_concepts: Vec::new(),
        }
    }

    /// Validate metadata completeness
    pub fn is_complete(&self) -> bool {
        !self.summary.is_empty()
            && !self.semantic_title.is_empty()
            && !self.keywords.is_empty()
            && !self.category.is_empty()
    }

    /// Convert to JSON string
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string(self).map_err(|e| e.into())
    }

    /// Parse from JSON string
    pub fn from_json(json: &str) -> Result<Self> {
        serde_json::from_str(json).map_err(|e| e.into())
    }
}

impl Default for DocumentMetadata {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata_context_builder() {
        let context = MetadataContext::new("file".to_string(), "test-collection".to_string())
            .with_language("rust".to_string())
            .with_extension("rs".to_string())
            .with_timestamps("2024-01-01".to_string(), "2024-01-02".to_string());

        assert_eq!(context.source_type, "file");
        assert_eq!(context.collection_name, "test-collection");
        assert_eq!(context.language, Some("rust".to_string()));
        assert_eq!(context.file_extension, Some("rs".to_string()));
    }

    #[test]
    fn test_document_metadata_basic() {
        let metadata = DocumentMetadata::basic(
            "Test Document".to_string(),
            "This is a test summary.".to_string(),
        );

        assert_eq!(metadata.semantic_title, "Test Document");
        assert_eq!(metadata.summary, "This is a test summary.");
        assert_eq!(metadata.difficulty, "intermediate");
        assert!(!metadata.is_complete());
    }

    #[test]
    fn test_document_metadata_complete() {
        let metadata = DocumentMetadata {
            summary: "A comprehensive test".to_string(),
            semantic_title: "Test".to_string(),
            keywords: vec!["test".to_string()],
            category: "test".to_string(),
            intent: "Testing".to_string(),
            concepts: vec!["testing".to_string()],
            difficulty: "beginner".to_string(),
            suggested_queries: vec!["how to test".to_string()],
            extracted_concepts: Vec::new(),
        };

        assert!(metadata.is_complete());
    }

    #[test]
    fn test_metadata_json_serialization() {
        let metadata = DocumentMetadata {
            summary: "Test summary".to_string(),
            semantic_title: "Test Title".to_string(),
            keywords: vec!["test".to_string(), "rust".to_string()],
            category: "tutorial".to_string(),
            intent: "Learn testing".to_string(),
            concepts: vec!["unit testing".to_string()],
            difficulty: "beginner".to_string(),
            suggested_queries: vec!["rust testing".to_string()],
            extracted_concepts: Vec::new(),
        };

        let json = metadata.to_json().unwrap();
        let parsed = DocumentMetadata::from_json(&json).unwrap();

        assert_eq!(metadata, parsed);
    }

    #[test]
    fn test_metadata_context_with_structure() {
        let structure = vec![ChunkType::Function, ChunkType::Struct];
        let context = MetadataContext::new("file".to_string(), "code".to_string())
            .with_structure(structure.clone());

        assert_eq!(context.existing_structure, Some(structure));
    }
}
