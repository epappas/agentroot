//! Natural language query parser using LLM
//!
//! Parses user queries like "files edited last hour" into structured search parameters
//!
//! NOTE: LLM-based parsing temporarily disabled during Candle migration.
//! Will be re-implemented using Candle text generation in the future.

use crate::error::{AgentRootError, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Parsed query with extracted intent and filters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedQuery {
    /// The cleaned search terms to use
    pub search_terms: String,

    /// Temporal constraints
    pub temporal_filter: Option<TemporalFilter>,

    /// Metadata filters extracted from query
    pub metadata_filters: Vec<MetadataFilterHint>,

    /// Suggested search type
    pub search_type: SearchType,

    /// Confidence in the parse (0.0 - 1.0)
    pub confidence: f64,
}

/// Temporal filter for time-based queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalFilter {
    /// Start datetime (ISO 8601)
    pub start: Option<String>,

    /// End datetime (ISO 8601)
    pub end: Option<String>,

    /// Human-readable description
    pub description: String,
}

/// Metadata filter hint extracted from query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataFilterHint {
    /// Field name
    pub field: String,

    /// Expected value
    pub value: String,

    /// Operator (eq, contains, gt, lt)
    pub operator: String,
}

/// Search type recommendation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SearchType {
    /// BM25 full-text search
    Bm25,

    /// Vector semantic search
    Vector,

    /// Hybrid (both + reranking)
    Hybrid,
}

/// Query parser (LLM-based parsing temporarily disabled)
pub struct QueryParser {
    #[allow(dead_code)]
    model_path: PathBuf,
}

impl QueryParser {
    /// Create a new query parser with custom model
    pub fn new(model_path: PathBuf) -> Result<Self> {
        if !model_path.exists() {
            return Err(AgentRootError::ModelNotFound(
                model_path.to_string_lossy().to_string(),
            ));
        }
        Ok(Self { model_path })
    }

    /// Create parser with default model (currently disabled)
    pub fn from_default() -> Result<Self> {
        Err(AgentRootError::Config(
            "LLM-based query parsing temporarily disabled during Candle migration. \
             Smart search will use rule-based parsing instead."
                .to_string(),
        ))
    }

    /// Parse natural language query into structured search
    pub async fn parse(&self, _query: &str) -> Result<ParsedQuery> {
        Err(AgentRootError::Config(
            "LLM query parsing disabled (Candle migration in progress)".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_parse_requires_model() {
        let result = QueryParser::from_default();
        assert!(result.is_err());
    }
}
