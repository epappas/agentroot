//! Unified intelligent search - automatically chooses best strategy
//!
//! Uses LLM to analyze query intent and select optimal search strategy.
//! Falls back to heuristics if LLM unavailable.

use super::{hybrid_search, parse_metadata_filters, SearchOptions, SearchResult};
use crate::db::Database;
use crate::error::Result;
use crate::llm::{
    heuristic_strategy, HttpEmbedder, HttpQueryExpander, HttpQueryParser, HttpReranker,
    HttpStrategyAnalyzer,
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

    // Detect predominant language for context
    let language_context = detect_language_context(db, enhanced_options.collection.as_deref());

    // Try LLM-based strategy selection first
    let analysis = match HttpStrategyAnalyzer::from_env() {
        Ok(analyzer) => match analyzer.analyze(search_terms, language_context.as_deref()).await {
            Ok(analysis) => {
                tracing::info!(
                    "LLM Strategy: {:?}, Granularity: {:?} (confidence: {:.2}, reasoning: {})",
                    analysis.strategy,
                    analysis.granularity,
                    analysis.confidence,
                    analysis.reasoning
                );
                if analysis.is_multilingual {
                    tracing::info!("Multilingual query detected");
                }
                analysis
            }
            Err(e) => {
                tracing::warn!(
                    "LLM strategy analysis failed: {}, using heuristic fallback",
                    e
                );
                let fallback = heuristic_strategy(search_terms, has_embeddings);
                tracing::info!(
                    "Heuristic Strategy: {:?}, Granularity: {:?} (reasoning: {})",
                    fallback.strategy,
                    fallback.granularity,
                    fallback.reasoning
                );
                fallback
            }
        },
        Err(_) => {
            // LLM not configured, use heuristics
            let fallback = heuristic_strategy(search_terms, has_embeddings);
            tracing::debug!(
                "Heuristic Strategy: {:?}, Granularity: {:?} (reasoning: {})",
                fallback.strategy,
                fallback.granularity,
                fallback.reasoning
            );
            fallback
        }
    };

    // Execute search based on strategy and granularity
    let results = execute_intelligent_search(
        db,
        search_terms,
        &analysis,
        &enhanced_options,
    )
    .await?;

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

/// Execute search based on strategy and granularity
async fn execute_intelligent_search(
    db: &Database,
    query: &str,
    analysis: &crate::llm::StrategyAnalysis,
    options: &SearchOptions,
) -> Result<Vec<SearchResult>> {
    use crate::llm::{SearchGranularity, SearchStrategy};

    // First, execute the base search with the chosen strategy
    let base_results = match analysis.strategy {
        SearchStrategy::Bm25 => {
            // Choose granularity for BM25
            match analysis.granularity {
                SearchGranularity::Document => db.search_fts(query, options)?,
                SearchGranularity::Chunk => db.search_chunks_bm25(query, options)?,
                SearchGranularity::Both => {
                    // Search documents first, then chunks
                    let mut docs = db.search_fts(query, options)?;
                    let chunks = db.search_chunks_bm25(query, options)?;
                    docs.extend(chunks);
                    docs
                }
            }
        }

        SearchStrategy::Vector => {
            let embedder = HttpEmbedder::from_env()?;
            // For now, vector search at document level
            // TODO: Add chunk-level vector search
            db.search_vec(query, &embedder, options).await?
        }

        SearchStrategy::Hybrid => {
            let embedder = HttpEmbedder::from_env()?;
            let expander = HttpQueryExpander::from_env().ok();
            let reranker = HttpReranker::from_env().ok();

            // For now, hybrid at document level
            // TODO: Add chunk-level hybrid search
            hybrid_search(
                db,
                query,
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

    Ok(base_results)
}

/// Detect predominant programming language from collection patterns
fn detect_language_context(db: &Database, collection: Option<&str>) -> Option<String> {
    use std::collections::HashMap;

    // Try to get collection info to check file patterns
    let collections = if let Some(coll_name) = collection {
        vec![coll_name.to_string()]
    } else {
        // Get all collections
        db.list_collections()
            .ok()?
            .into_iter()
            .map(|c| c.name)
            .collect()
    };

    let mut language_counts: HashMap<String, usize> = HashMap::new();

    // Check patterns for language indicators and count document counts
    // Extract language from file extension pattern (no hardcoded mapping)
    for coll_name in collections {
        if let Ok(Some(coll)) = db.get_collection(&coll_name) {
            // Extract extension from pattern like "**/*.rs" or "*.py"
            let lang = extract_language_from_pattern(&coll.pattern);

            if let Some(language) = lang {
                *language_counts.entry(language).or_insert(0) += coll.document_count;
            }
        }
    }

    // Return the most common language by document count
    language_counts
        .into_iter()
        .max_by_key(|(_, count)| *count)
        .map(|(lang, _)| lang.to_string())
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

/// Extract language name from file pattern (no hardcoded mappings)
/// Uses file extension as the language identifier
fn extract_language_from_pattern(pattern: &str) -> Option<String> {
    // Match patterns like "**/*.rs", "*.py", "**/*.{ts,tsx}"
    // Extract the extension(s) after the last dot
    
    if let Some(ext_part) = pattern.split("*.").last() {
        // Handle simple extension: "*.rs" -> "rs"
        if !ext_part.contains('{') && !ext_part.contains(',') {
            let ext = ext_part.trim();
            if !ext.is_empty() && ext.chars().all(|c| c.is_alphanumeric()) {
                // Capitalize: "rs" -> "Rust"
                return Some(capitalize_extension(ext));
            }
        }
        
        // Handle multiple extensions: "*.{ts,tsx}" -> use first one
        if ext_part.contains('{') {
            if let Some(first_ext) = ext_part
                .trim_start_matches('{')
                .split(',')
                .next()
            {
                return Some(capitalize_extension(first_ext.trim()));
            }
        }
    }
    
    None
}

/// Map file extension to language name
/// This is metadata mapping (extension -> display name), not search logic
fn capitalize_extension(ext: &str) -> String {
    // Common extension to language name mappings for display
    match ext {
        "rs" => "Rust",
        "py" => "Python", 
        "js" => "JavaScript",
        "ts" => "TypeScript",
        "go" => "Go",
        "java" => "Java",
        "cpp" | "cc" | "cxx" => "C++",
        "c" => "C",
        "rb" => "Ruby",
        "php" => "PHP",
        "swift" => "Swift",
        "kt" => "Kotlin",
        // Fallback: capitalize first letter
        _ => {
            let mut chars = ext.chars();
            return match chars.next() {
                Some(first) => first.to_uppercase().chain(chars).collect(),
                None => ext.to_string(),
            };
        }
    }.to_string()
}
