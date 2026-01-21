//! Natural language query parser types
//!
//! Data structures for parsed queries extracted from natural language.
//! Query parsing is performed by external LLM services via HttpQueryParser.

use serde::{Deserialize, Serialize};

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
