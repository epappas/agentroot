//! Natural language query parser using LLM
//!
//! Parses user queries like "files edited last hour" into structured search parameters

use crate::error::{AgentRootError, Result};
use chrono::{DateTime, Duration, Utc};
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

/// Query parser using local LLM
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

    /// Create parser with default model
    pub fn from_default() -> Result<Self> {
        let model_dir = dirs::data_local_dir()
            .ok_or_else(|| AgentRootError::Config("Cannot determine data directory".to_string()))?
            .join("agentroot")
            .join("models");

        let model_path = model_dir.join("llama-3.1-8b-instruct.Q4_K_M.gguf");

        if !model_path.exists() {
            return Err(AgentRootError::ModelNotFound(format!(
                "Model not found at {}. Run 'agentroot embed' first to download models.",
                model_path.display()
            )));
        }

        Ok(Self { model_path })
    }

    /// Parse natural language query into structured search
    pub async fn parse(&self, query: &str) -> Result<ParsedQuery> {
        // First try rule-based parsing for common patterns
        if let Some(parsed) = self.try_rule_based_parse(query) {
            return Ok(parsed);
        }

        // Fall back to LLM-based parsing
        self.llm_parse(query).await
    }

    /// Try rule-based parsing for common patterns (fast path)
    fn try_rule_based_parse(&self, query: &str) -> Option<ParsedQuery> {
        let query_lower = query.to_lowercase();

        // Detect temporal queries
        if let Some(temporal) = self.extract_temporal_pattern(&query_lower) {
            let search_terms = self.remove_temporal_keywords(&query_lower);
            return Some(ParsedQuery {
                search_terms: search_terms.trim().to_string(),
                temporal_filter: Some(temporal),
                metadata_filters: vec![],
                search_type: SearchType::Bm25,
                confidence: 0.8,
            });
        }

        // Detect metadata queries
        if let Some((field, value, search_terms)) = self.extract_metadata_pattern(query) {
            return Some(ParsedQuery {
                search_terms,
                temporal_filter: None,
                metadata_filters: vec![MetadataFilterHint {
                    field,
                    value,
                    operator: "contains".to_string(),
                }],
                search_type: SearchType::Hybrid,
                confidence: 0.85,
            });
        }

        None
    }

    /// Extract temporal patterns like "last hour", "yesterday", "today"
    fn extract_temporal_pattern(&self, query: &str) -> Option<TemporalFilter> {
        let now = Utc::now();

        if query.contains("last hour") || query.contains("past hour") {
            let start = now - Duration::hours(1);
            return Some(TemporalFilter {
                start: Some(start.to_rfc3339()),
                end: Some(now.to_rfc3339()),
                description: "Last hour".to_string(),
            });
        }

        if query.contains("last 24 hours") || query.contains("past day") {
            let start = now - Duration::days(1);
            return Some(TemporalFilter {
                start: Some(start.to_rfc3339()),
                end: Some(now.to_rfc3339()),
                description: "Last 24 hours".to_string(),
            });
        }

        if query.contains("yesterday") {
            let yesterday = now - Duration::days(1);
            let start = yesterday.date_naive().and_hms_opt(0, 0, 0)?;
            let end = yesterday.date_naive().and_hms_opt(23, 59, 59)?;
            return Some(TemporalFilter {
                start: Some(DateTime::<Utc>::from_naive_utc_and_offset(start, Utc).to_rfc3339()),
                end: Some(DateTime::<Utc>::from_naive_utc_and_offset(end, Utc).to_rfc3339()),
                description: "Yesterday".to_string(),
            });
        }

        if query.contains("today") {
            let today = now.date_naive().and_hms_opt(0, 0, 0)?;
            return Some(TemporalFilter {
                start: Some(DateTime::<Utc>::from_naive_utc_and_offset(today, Utc).to_rfc3339()),
                end: Some(now.to_rfc3339()),
                description: "Today".to_string(),
            });
        }

        if query.contains("this week") || query.contains("last week") {
            let start = now - Duration::weeks(1);
            return Some(TemporalFilter {
                start: Some(start.to_rfc3339()),
                end: Some(now.to_rfc3339()),
                description: "Last week".to_string(),
            });
        }

        if query.contains("this month") || query.contains("last month") {
            let start = now - Duration::days(30);
            return Some(TemporalFilter {
                start: Some(start.to_rfc3339()),
                end: Some(now.to_rfc3339()),
                description: "Last 30 days".to_string(),
            });
        }

        None
    }

    /// Remove temporal keywords from query
    fn remove_temporal_keywords(&self, query: &str) -> String {
        let keywords = [
            "last hour",
            "past hour",
            "last 24 hours",
            "past day",
            "yesterday",
            "today",
            "this week",
            "last week",
            "this month",
            "last month",
            "edited",
            "modified",
            "created",
            "from",
            "since",
            "in",
            "during",
        ];

        let mut cleaned = query.to_string();
        for keyword in &keywords {
            cleaned = cleaned.replace(keyword, " ");
        }

        cleaned.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    /// Extract metadata patterns like "by Alice", "author:Alice"
    fn extract_metadata_pattern(&self, query: &str) -> Option<(String, String, String)> {
        let query_lower = query.to_lowercase();

        // Pattern: "by <author>"
        if let Some(idx) = query_lower.find(" by ") {
            let after = &query[idx + 4..];
            let author = after.split_whitespace().next()?.trim();
            let search_terms = query[..idx].trim().to_string();
            return Some(("author".to_string(), author.to_string(), search_terms));
        }

        // Pattern: "author:<value>"
        if let Some(idx) = query_lower.find("author:") {
            let after = &query[idx + 7..];
            let author = after.split_whitespace().next()?.trim();
            let search_terms = format!("{} {}", &query[..idx], &after[author.len()..])
                .trim()
                .to_string();
            return Some(("author".to_string(), author.to_string(), search_terms));
        }

        // Pattern: "tagged <tag>" or "tag:<tag>"
        if let Some(idx) = query_lower.find("tagged ") {
            let after = &query[idx + 7..];
            let tag = after.split_whitespace().next()?.trim();
            let search_terms = query[..idx].trim().to_string();
            return Some(("tags".to_string(), tag.to_string(), search_terms));
        }

        None
    }

    /// Parse query using LLM (fallback for complex queries)
    async fn llm_parse(&self, query: &str) -> Result<ParsedQuery> {
        // For now, return a simple parsed query
        // TODO: Implement full LLM parsing with llama-cpp-2

        tracing::debug!("LLM parsing not yet implemented, using fallback");

        // Fallback: treat as semantic search
        Ok(ParsedQuery {
            search_terms: query.to_string(),
            temporal_filter: None,
            metadata_filters: vec![],
            search_type: SearchType::Hybrid,
            confidence: 0.5,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_temporal_last_hour() {
        let parser = QueryParser::from_default().unwrap_or_else(|_| QueryParser {
            model_path: PathBuf::from("dummy"),
        });

        let result = parser.try_rule_based_parse("files edited last hour");
        assert!(result.is_some());

        let parsed = result.unwrap();
        assert!(parsed.temporal_filter.is_some());
        assert_eq!(parsed.temporal_filter.unwrap().description, "Last hour");
        assert_eq!(parsed.search_terms, "files");
    }

    #[test]
    fn test_parse_temporal_yesterday() {
        let parser = QueryParser {
            model_path: PathBuf::from("dummy"),
        };

        let result = parser.try_rule_based_parse("documents from yesterday");
        assert!(result.is_some());

        let parsed = result.unwrap();
        assert!(parsed.temporal_filter.is_some());
        assert_eq!(parsed.temporal_filter.unwrap().description, "Yesterday");
    }

    #[test]
    fn test_parse_metadata_by_author() {
        let parser = QueryParser {
            model_path: PathBuf::from("dummy"),
        };

        let result = parser.try_rule_based_parse("rust tutorials by Alice");
        assert!(result.is_some());

        let parsed = result.unwrap();
        assert_eq!(parsed.search_terms, "rust tutorials");
        assert_eq!(parsed.metadata_filters.len(), 1);
        assert_eq!(parsed.metadata_filters[0].field, "author");
        assert_eq!(parsed.metadata_filters[0].value, "Alice");
    }

    #[test]
    fn test_parse_metadata_author_colon() {
        let parser = QueryParser {
            model_path: PathBuf::from("dummy"),
        };

        let result = parser.try_rule_based_parse("author:Alice rust tutorial");
        assert!(result.is_some());

        let parsed = result.unwrap();
        assert!(parsed.search_terms.contains("rust"));
        assert_eq!(parsed.metadata_filters[0].field, "author");
    }

    #[test]
    fn test_parse_combined_temporal_metadata() {
        let parser = QueryParser {
            model_path: PathBuf::from("dummy"),
        };

        // First detect temporal
        let result = parser.try_rule_based_parse("files by Alice last hour");
        assert!(result.is_some());

        let parsed = result.unwrap();
        assert!(parsed.temporal_filter.is_some());
    }
}
