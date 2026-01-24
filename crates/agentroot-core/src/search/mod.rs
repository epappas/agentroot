//! Search engine module
//!
//! Provides:
//! - BM25 full-text search via FTS5
//! - Vector similarity search via sqlite-vec
//! - Hybrid search with RRF fusion

mod bm25;
mod hybrid;
mod orchestrated;
mod smart;
mod snippet;
mod unified;
mod vector;
mod workflow_executor;

pub use hybrid::*;
pub use orchestrated::orchestrated_search;
pub use smart::smart_search;
pub use snippet::*;
pub use unified::unified_search;
pub use workflow_executor::execute_workflow;

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
    /// Metadata filters (field, value) e.g., ("category", "tutorial")
    pub metadata_filters: Vec<(String, String)>,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            limit: 20,
            min_score: 0.0,
            collection: None,
            provider: None,
            full_content: false,
            metadata_filters: Vec::new(),
        }
    }
}

use crate::db::UserMetadata;

/// Search result (can represent document or chunk)
#[derive(Debug, Clone)]
pub struct SearchResult {
    // Document-level fields
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
    
    // Chunk-level fields (when result is a chunk)
    pub is_chunk: bool,
    pub chunk_hash: Option<String>,
    pub chunk_type: Option<String>,
    pub chunk_breadcrumb: Option<String>,
    pub chunk_start_line: Option<i32>,
    pub chunk_end_line: Option<i32>,
    pub chunk_language: Option<String>,
    pub chunk_summary: Option<String>,
    pub chunk_purpose: Option<String>,
    pub chunk_concepts: Vec<String>,
    pub chunk_labels: std::collections::HashMap<String, String>,
}

/// Source of search result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchSource {
    Bm25,
    Vector,
    Hybrid,
    Glossary,
}

/// Common English stop words to remove from natural language queries
const STOP_WORDS: &[&str] = &[
    "a", "an", "and", "are", "as", "at", "be", "by", "for", "from",
    "has", "have", "he", "in", "is", "it", "its", "of", "on", "that",
    "the", "to", "was", "will", "with", "does", "do", "did", "can",
    "could", "should", "would", "what", "where", "when", "why", "how",
    "who", "which", "this", "these", "those", "there", "here",
];

/// Sanitize query for FTS5 to prevent syntax errors
/// Removes stop words and problematic FTS5 operator characters
pub fn sanitize_fts5_query(query: &str) -> String {
    if query.trim().is_empty() {
        return query.to_string();
    }
    
    // First, remove FTS5 special operator characters
    let cleaned = query
        .replace('?', "")       // Remove question marks
        .replace('!', "")       // Remove exclamation
        .replace('^', "")        // Remove caret
        .replace('(', "")        // Remove unbalanced parens
        .replace(')', "")
        .replace('[', "")        // Remove brackets
        .replace(']', "")
        .replace('{', "")        // Remove braces
        .replace('}', "");
    
    // Split into words and filter out stop words
    let words: Vec<&str> = cleaned
        .split_whitespace()
        .filter(|word| {
            let lower = word.to_lowercase();
            // Keep word if it's not a stop word or if it's part of a field filter (contains :)
            !STOP_WORDS.contains(&lower.as_str()) || word.contains(':')
        })
        .collect();
    
    if words.is_empty() {
        return String::new();
    }
    
    // Keep AND logic (default) for better precision
    // Natural language queries like "does agentroot have mcp?" become "agentroot mcp"
    words.join(" ")
}

/// Parse metadata filters from query string
/// Supports syntax: "category:tutorial difficulty:beginner search terms"
/// Returns: (clean_query, filters)
pub fn parse_metadata_filters(query: &str) -> (String, Vec<(String, String)>) {
    let mut filters = Vec::new();
    let mut remaining_terms = Vec::new();

    for term in query.split_whitespace() {
        if let Some(colon_pos) = term.find(':') {
            let field = term[..colon_pos].to_lowercase();
            let value = term[colon_pos + 1..].to_string();

            // Only parse known metadata fields as filters
            if matches!(
                field.as_str(),
                "category" | "difficulty" | "tag" | "keyword"
            ) {
                filters.push((field, value));
                continue;
            }
        }
        remaining_terms.push(term);
    }

    let clean_query = remaining_terms.join(" ");
    let sanitized_query = sanitize_fts5_query(&clean_query);
    (sanitized_query, filters)
}
