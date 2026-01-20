//! Search engine module
//!
//! Provides:
//! - BM25 full-text search via FTS5
//! - Vector similarity search via sqlite-vec
//! - Hybrid search with RRF fusion

mod bm25;
mod hybrid;
mod smart;
mod snippet;
mod vector;

pub use hybrid::*;
pub use smart::smart_search;
pub use snippet::*;

/// Search options
#[derive(Debug, Clone)]
pub struct SearchOptions {
    /// Maximum number of results
    pub limit: usize,
    /// Minimum score threshold (0.0 - 1.0)
    pub min_score: f64,
    /// Filter by collection name
    pub collection: Option<String>,
    /// Filter by provider type (e.g., "file", "github")
    pub provider: Option<String>,
    /// Include full document content
    pub full_content: bool,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            limit: 20,
            min_score: 0.0,
            collection: None,
            provider: None,
            full_content: false,
        }
    }
}

use crate::db::UserMetadata;

/// Search result
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub filepath: String,
    pub display_path: String,
    pub title: String,
    pub hash: String,
    pub collection_name: String,
    pub modified_at: String,
    pub body: Option<String>,
    pub body_length: usize,
    pub docid: String,
    pub context: Option<String>,
    pub score: f64,
    pub source: SearchSource,
    pub chunk_pos: Option<usize>,
    pub llm_summary: Option<String>,
    pub llm_title: Option<String>,
    pub llm_keywords: Option<Vec<String>>,
    pub llm_category: Option<String>,
    pub llm_difficulty: Option<String>,
    pub user_metadata: Option<UserMetadata>,
}

/// Source of search result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchSource {
    Bm25,
    Vector,
    Hybrid,
}
