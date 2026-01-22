//! Unified intelligent search - automatically chooses best strategy

use super::{hybrid_search, SearchOptions, SearchResult};
use crate::db::Database;
use crate::error::Result;
use crate::llm::{HttpEmbedder, HttpQueryExpander, HttpQueryParser, HttpReranker};

/// Unified intelligent search that automatically:
/// 1. Parses metadata filters (category:X, difficulty:Y)
/// 2. Parses temporal filters (last week, recently)
/// 3. Chooses optimal search strategy (BM25/vector/hybrid)
/// 4. Applies query expansion when beneficial
/// 5. Uses reranking when available
///
/// This is the ONE search function users should use.
pub async fn unified_search(
    db: &Database,
    query: &str,
    options: &SearchOptions,
) -> Result<Vec<SearchResult>> {
    // Try to parse natural language query (temporal, metadata filters)
    let parsed_query = if let Ok(parser) = HttpQueryParser::from_env() {
        match parser.parse(query).await {
            Ok(parsed) => Some(parsed),
            Err(e) => {
                tracing::debug!("Query parsing failed: {}, using raw query", e);
                None
            }
        }
    } else {
        None
    };

    // Extract search terms and filters
    let search_terms = if let Some(ref pq) = parsed_query {
        &pq.search_terms
    } else {
        query
    };

    // Check if embeddings are available
    let has_embeddings = db.has_vector_index();

    // Automatically choose best search strategy
    let results = if !has_embeddings {
        // No embeddings → BM25 only
        tracing::info!("Strategy: BM25 (no embeddings available)");
        db.search_fts(search_terms, options)?
    } else {
        // Analyze query characteristics to choose strategy
        let is_natural_language = is_natural_language_query(query);
        let has_exact_terms = has_exact_technical_terms(query);

        if is_natural_language && !has_exact_terms {
            // Natural language question → Vector search is best
            tracing::info!("Strategy: Vector (natural language query)");
            let embedder = HttpEmbedder::from_env()?;
            db.search_vec(search_terms, &embedder, options).await?
        } else {
            // Mixed or technical query → Use full hybrid with expansion & reranking
            tracing::info!("Strategy: Hybrid (mixed query)");

            let embedder = HttpEmbedder::from_env()?;
            let expander = HttpQueryExpander::from_env().ok();
            let reranker = HttpReranker::from_env().ok();

            hybrid_search(
                db,
                search_terms,
                options,
                &embedder,
                expander
                    .as_ref()
                    .map(|e| e as &dyn crate::llm::QueryExpander),
                reranker.as_ref().map(|r| r as &dyn crate::llm::Reranker),
            )
            .await?
        }
    };

    // Apply temporal filtering if detected
    let results = if let Some(ref pq) = parsed_query {
        if let Some(ref temporal) = pq.temporal_filter {
            apply_temporal_filter(results, temporal)?
        } else {
            results
        }
    } else {
        results
    };

    // Apply metadata filtering if detected
    let results = if let Some(ref pq) = parsed_query {
        if !pq.metadata_filters.is_empty() {
            apply_metadata_filters(results, &pq.metadata_filters)?
        } else {
            results
        }
    } else {
        results
    };

    Ok(results)
}

/// Detect if query is natural language (vs technical terms)
fn is_natural_language_query(query: &str) -> bool {
    let nl_indicators = [
        "how to",
        "how do",
        "what is",
        "what are",
        "why does",
        "why do",
        "when should",
        "where can",
        "who is",
        "explain",
        "show me",
        "help me",
        "tutorial",
        "guide",
        "example",
    ];

    let lower = query.to_lowercase();
    nl_indicators
        .iter()
        .any(|indicator| lower.contains(indicator))
}

/// Detect if query has exact technical terms (code symbols, trait names, etc.)
fn has_exact_technical_terms(query: &str) -> bool {
    // Look for PascalCase, snake_case, SCREAMING_CASE, or :: (Rust paths)
    query.contains("::")
        || query
            .chars()
            .any(|c| c.is_uppercase() && query.chars().filter(|&c| c == '_').count() == 0)
        || query.contains('_')
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
            let after_start = start_time.is_none_or(|start| modified_utc >= start);
            let before_end = end_time.is_none_or(|end| modified_utc <= end);
            after_start && before_end
        } else {
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
