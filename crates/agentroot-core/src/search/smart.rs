//! Smart search with natural language query understanding

use crate::db::Database;
use crate::error::Result;
use crate::llm::{LlamaEmbedder, QueryParser};
use crate::search::{hybrid_search, SearchOptions, SearchResult};

/// Smart search that understands natural language queries
///
/// Automatically parses queries like:
/// - "files edited last hour" → applies temporal filter
/// - "rust tutorials by Alice" → applies metadata filter
/// - "recent python code" → semantic search with recency
///
/// Falls back to BM25 search if query parser model is not available.
pub async fn smart_search(
    db: &Database,
    query: &str,
    options: &SearchOptions,
) -> Result<Vec<SearchResult>> {
    // Try to parse the natural language query
    let parser_result = QueryParser::from_default();

    if let Ok(parser) = parser_result {
        // Parser available - use smart parsing
        let parsed = parser.parse(query).await?;

        tracing::info!(
            "Parsed query: '{}' → search_terms='{}', temporal={:?}, metadata_filters={:?}",
            query,
            parsed.search_terms,
            parsed.temporal_filter.as_ref().map(|t| &t.description),
            parsed.metadata_filters.len()
        );

        // Start with base search using extracted terms
        let mut results = match parsed.search_type {
            crate::llm::SearchType::Bm25 => db.search_fts(&parsed.search_terms, options)?,
            crate::llm::SearchType::Vector => {
                // Vector search requires embedder
                match LlamaEmbedder::from_default() {
                    Ok(embedder) => {
                        db.search_vec(&parsed.search_terms, &embedder, options)
                            .await?
                    }
                    Err(_) => {
                        tracing::warn!("Embedder not available, falling back to BM25");
                        db.search_fts(&parsed.search_terms, options)?
                    }
                }
            }
            crate::llm::SearchType::Hybrid => {
                // Hybrid search requires embedder
                match LlamaEmbedder::from_default() {
                    Ok(embedder) => {
                        hybrid_search(db, &parsed.search_terms, options, &embedder, None, None)
                            .await?
                    }
                    Err(_) => {
                        tracing::warn!("Embedder not available, falling back to BM25");
                        db.search_fts(&parsed.search_terms, options)?
                    }
                }
            }
        };

        // Apply temporal filtering if present
        if let Some(temporal) = &parsed.temporal_filter {
            results = apply_temporal_filter(results, temporal)?;
        }

        // Apply metadata filtering if present
        if !parsed.metadata_filters.is_empty() {
            results = apply_metadata_filters(results, &parsed.metadata_filters)?;
        }

        Ok(results)
    } else {
        // Fallback to simple BM25 search if parser not available
        tracing::warn!("Query parser not available, falling back to BM25 search");
        db.search_fts(query, options)
    }
}

/// Apply temporal filter to results
fn apply_temporal_filter(
    mut results: Vec<SearchResult>,
    temporal: &crate::llm::TemporalFilter,
) -> Result<Vec<SearchResult>> {
    use chrono::{DateTime, Utc};

    let start_time = temporal
        .start
        .as_ref()
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&Utc));

    let end_time = temporal
        .end
        .as_ref()
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&Utc));

    results.retain(|result| {
        if let Ok(modified_at) = DateTime::parse_from_rfc3339(&result.modified_at) {
            let modified_utc = modified_at.with_timezone(&Utc);

            let after_start = start_time.map_or(true, |start| modified_utc >= start);
            let before_end = end_time.map_or(true, |end| modified_utc <= end);

            after_start && before_end
        } else {
            // Keep results with unparseable dates
            true
        }
    });

    tracing::info!(
        "Temporal filter '{}' applied: {} results remain",
        temporal.description,
        results.len()
    );

    Ok(results)
}

/// Apply metadata filters to results
fn apply_metadata_filters(
    mut results: Vec<SearchResult>,
    filters: &[crate::llm::MetadataFilterHint],
) -> Result<Vec<SearchResult>> {
    for filter in filters {
        let initial_count = results.len();

        results.retain(|result| {
            if let Some(user_meta) = &result.user_metadata {
                if let Some(value) = user_meta.get(&filter.field) {
                    match filter.operator.as_str() {
                        "eq" => format!("{:?}", value).contains(&filter.value),
                        "contains" => format!("{:?}", value)
                            .to_lowercase()
                            .contains(&filter.value.to_lowercase()),
                        _ => true,
                    }
                } else {
                    false
                }
            } else {
                false
            }
        });

        tracing::info!(
            "Metadata filter {}:{}={} applied: {} → {} results",
            filter.field,
            filter.operator,
            filter.value,
            initial_count,
            results.len()
        );
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::TemporalFilter;
    use chrono::Utc;

    #[test]
    fn test_temporal_filter_last_hour() {
        let now = Utc::now();
        let one_hour_ago = now - chrono::Duration::hours(1);
        let two_hours_ago = now - chrono::Duration::hours(2);

        let temporal = TemporalFilter {
            start: Some(one_hour_ago.to_rfc3339()),
            end: Some(now.to_rfc3339()),
            description: "Last hour".to_string(),
        };

        let results = vec![
            create_test_result(now.to_rfc3339()),           // Should pass
            create_test_result(one_hour_ago.to_rfc3339()),  // Should pass
            create_test_result(two_hours_ago.to_rfc3339()), // Should fail
        ];

        let filtered = apply_temporal_filter(results, &temporal).unwrap();
        assert_eq!(filtered.len(), 2);
    }

    fn create_test_result(modified_at: String) -> SearchResult {
        SearchResult {
            filepath: "test".to_string(),
            display_path: "test".to_string(),
            title: "Test".to_string(),
            hash: "abc123".to_string(),
            collection_name: "test".to_string(),
            modified_at,
            body: None,
            body_length: 0,
            docid: "abc123".to_string(),
            context: None,
            score: 1.0,
            source: crate::search::SearchSource::Bm25,
            chunk_pos: None,
            llm_summary: None,
            llm_title: None,
            llm_keywords: None,
            llm_category: None,
            llm_difficulty: None,
            user_metadata: None,
        }
    }
}
