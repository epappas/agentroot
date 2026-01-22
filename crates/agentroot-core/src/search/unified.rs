//! Unified intelligent search - automatically chooses best strategy
//!
//! Uses LLM to analyze query intent and select optimal search strategy.
//! Falls back to heuristics if LLM unavailable.

use super::{hybrid_search, parse_metadata_filters, SearchOptions, SearchResult};
use crate::db::Database;
use crate::error::Result;
use crate::llm::{
    heuristic_strategy, HttpEmbedder, HttpQueryExpander, HttpQueryParser, HttpReranker,
    HttpStrategyAnalyzer, SearchStrategy,
};

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
    // Parse metadata filters from query (category:X, difficulty:Y, etc.)
    let (clean_query, metadata_filters) = parse_metadata_filters(query);

    // Create enhanced options with parsed metadata filters
    let mut enhanced_options = options.clone();
    enhanced_options.metadata_filters.extend(metadata_filters);

    tracing::debug!("Parsed filters: {:?}", enhanced_options.metadata_filters);

    // Try to parse natural language query (temporal filters)
    let parsed_query = if let Ok(parser) = HttpQueryParser::from_env() {
        match parser.parse(&clean_query).await {
            Ok(parsed) => Some(parsed),
            Err(e) => {
                tracing::debug!("Query parsing failed: {}, using raw query", e);
                None
            }
        }
    } else {
        None
    };

    // Extract search terms
    let search_terms = if let Some(ref pq) = parsed_query {
        &pq.search_terms
    } else {
        &clean_query
    };

    // Check if embeddings are available
    let has_embeddings = db.has_vector_index();

    if !has_embeddings {
        // No embeddings → BM25 only
        tracing::info!("Strategy: BM25 (no embeddings available)");
        return db.search_fts(search_terms, &enhanced_options);
    }

    // Try LLM-based strategy selection first
    let strategy = match HttpStrategyAnalyzer::from_env() {
        Ok(analyzer) => match analyzer.analyze(search_terms).await {
            Ok(analysis) => {
                tracing::info!(
                    "LLM Strategy: {:?} (confidence: {:.2}, reasoning: {})",
                    analysis.strategy,
                    analysis.confidence,
                    analysis.reasoning
                );
                if analysis.is_multilingual {
                    tracing::info!("Multilingual query detected");
                }
                analysis.strategy
            }
            Err(e) => {
                tracing::warn!(
                    "LLM strategy analysis failed: {}, using heuristic fallback",
                    e
                );
                let fallback = heuristic_strategy(search_terms, has_embeddings);
                tracing::info!(
                    "Heuristic Strategy: {:?} (reasoning: {})",
                    fallback.strategy,
                    fallback.reasoning
                );
                fallback.strategy
            }
        },
        Err(_) => {
            // LLM not configured, use heuristics
            let fallback = heuristic_strategy(search_terms, has_embeddings);
            tracing::debug!(
                "Heuristic Strategy: {:?} (reasoning: {})",
                fallback.strategy,
                fallback.reasoning
            );
            fallback.strategy
        }
    };

    // Execute search based on strategy
    let results = match strategy {
        SearchStrategy::Bm25 => db.search_fts(search_terms, &enhanced_options)?,

        SearchStrategy::Vector => {
            let embedder = HttpEmbedder::from_env()?;
            db.search_vec(search_terms, &embedder, &enhanced_options)
                .await?
        }

        SearchStrategy::Hybrid => {
            let embedder = HttpEmbedder::from_env()?;
            let expander = HttpQueryExpander::from_env().ok();
            let reranker = HttpReranker::from_env().ok();

            hybrid_search(
                db,
                search_terms,
                &enhanced_options,
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
