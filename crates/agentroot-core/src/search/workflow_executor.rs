//! Workflow execution engine - runs planned workflows step-by-step

use super::{hybrid_search, SearchOptions, SearchResult, SearchSource};
use crate::db::Database;
use crate::error::Result;
use crate::llm::{
    HttpEmbedder, HttpQueryExpander, HttpReranker, MergeStrategy, QueryExpander, RerankDocument,
    Reranker, Workflow, WorkflowContext, WorkflowStep,
};
use chrono::{DateTime, Duration, Utc};
use rusqlite::params;
use std::collections::HashMap;

/// Execute a planned workflow
pub async fn execute_workflow(
    db: &Database,
    workflow: &Workflow,
    query: &str,
    options: &SearchOptions,
) -> Result<Vec<SearchResult>> {
    let mut context = WorkflowContext::new(query.to_string());

    tracing::info!(
        "Executing workflow: {} steps ({})",
        workflow.steps.len(),
        workflow.reasoning
    );

    for (idx, step) in workflow.steps.iter().enumerate() {
        tracing::debug!("Step {}/{}: {:?}", idx + 1, workflow.steps.len(), step);

        context = execute_step(db, step, context, options).await?;

        tracing::debug!("After step {}: {} results", idx + 1, context.results.len());
    }

    // Log execution summary
    tracing::info!(
        "Workflow completed: {} results (expected {})",
        context.results.len(),
        workflow.expected_results
    );

    for (step_name, count) in &context.step_results {
        tracing::debug!("  {}: {} results", step_name, count);
    }

    // Apply final limit from SearchOptions
    let final_results: Vec<SearchResult> = context
        .results
        .into_iter()
        .take(options.limit)
        .collect();

    Ok(final_results)
}

/// Execute a single workflow step
async fn execute_step(
    db: &Database,
    step: &WorkflowStep,
    mut context: WorkflowContext,
    base_options: &SearchOptions,
) -> Result<WorkflowContext> {
    match step {
        WorkflowStep::Bm25Search { query, limit } => {
            let mut opts = base_options.clone();
            opts.limit = *limit;

            let mut new_results = db.search_fts(query, &opts)?;
            let count = new_results.len();
            context.results.append(&mut new_results);
            context
                .step_results
                .push(("bm25_search".to_string(), count));
        }

        WorkflowStep::VectorSearch { query, limit } => {
            let embedder = HttpEmbedder::from_env()?;
            let mut opts = base_options.clone();
            opts.limit = *limit;

            let mut new_results = db.search_vec(query, &embedder, &opts).await?;
            let count = new_results.len();
            context.results.append(&mut new_results);
            context
                .step_results
                .push(("vector_search".to_string(), count));
        }

        WorkflowStep::HybridSearch {
            query,
            limit,
            use_expansion,
            use_reranking,
        } => {
            let embedder = HttpEmbedder::from_env()?;
            let mut opts = base_options.clone();
            opts.limit = *limit;

            let expander = if *use_expansion {
                HttpQueryExpander::from_env().ok()
            } else {
                None
            };

            let reranker = if *use_reranking {
                HttpReranker::from_env().ok()
            } else {
                None
            };

            let mut new_results = hybrid_search(
                db,
                query,
                &opts,
                &embedder,
                expander
                    .as_ref()
                    .map(|e| e as &dyn crate::llm::QueryExpander),
                reranker.as_ref().map(|r| r as &dyn crate::llm::Reranker),
            )
            .await?;

            let count = new_results.len();
            context.results.append(&mut new_results);
            context
                .step_results
                .push(("hybrid_search".to_string(), count));
        }

        // Filter by metadata (category, difficulty, tags)
        // Safety: If filter removes >90% of results, skip it to prevent
        // overly restrictive LLM-planned filters from eliminating good results
        WorkflowStep::FilterMetadata {
            category,
            difficulty,
            tags,
            exclude_category,
            exclude_difficulty,
        } => {
            let initial_count = context.results.len();
            let mut filtered_results = context.results.clone();

            filtered_results.retain(|result| {
                // Category filter
                if let Some(ref cat) = category {
                    if let Some(ref result_cat) = result.llm_category {
                        if !result_cat.eq_ignore_ascii_case(cat) {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }

                // Exclude category
                if let Some(ref ex_cat) = exclude_category {
                    if let Some(ref result_cat) = result.llm_category {
                        if result_cat.eq_ignore_ascii_case(ex_cat) {
                            return false;
                        }
                    }
                }

                // Difficulty filter
                if let Some(ref diff) = difficulty {
                    if let Some(ref result_diff) = result.llm_difficulty {
                        if !result_diff.eq_ignore_ascii_case(diff) {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }

                // Exclude difficulty
                if let Some(ref ex_diff) = exclude_difficulty {
                    if let Some(ref result_diff) = result.llm_difficulty {
                        if result_diff.eq_ignore_ascii_case(ex_diff) {
                            return false;
                        }
                    }
                }

                // Tags filter
                if let Some(ref filter_tags) = tags {
                    if let Some(ref result_keywords) = result.llm_keywords {
                        let has_any_tag = filter_tags.iter().any(|tag| {
                            result_keywords
                                .iter()
                                .any(|kw| kw.eq_ignore_ascii_case(tag))
                        });
                        if !has_any_tag {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }

                true
            });

            let filtered_count = filtered_results.len();
            let removal_rate = if initial_count > 0 {
                1.0 - (filtered_count as f64 / initial_count as f64)
            } else {
                0.0
            };

            if removal_rate > 0.9 && initial_count >= 5 {
                tracing::warn!(
                    "FilterMetadata removed {:.0}% of results ({} → {}), skipping overly restrictive filter",
                    removal_rate * 100.0,
                    initial_count,
                    filtered_count
                );
                eprintln!(
                    "[WARN] Metadata filter too aggressive ({} → {}), keeping unfiltered results",
                    initial_count, filtered_count
                );
            } else {
                context.results = filtered_results;
                tracing::debug!(
                    "Metadata filter: {} → {} results ({:.0}% kept)",
                    initial_count,
                    context.results.len(),
                    (1.0 - removal_rate) * 100.0
                );
            }

            context
                .step_results
                .push(("filter_metadata".to_string(), context.results.len()));
        }

        WorkflowStep::FilterTemporal { after, before } => {
            let initial_count = context.results.len();

            let after_time = parse_temporal_expression(after.as_deref())?;
            let before_time = parse_temporal_expression(before.as_deref())?;

            context.results.retain(|result| {
                if let Ok(modified_at) = DateTime::parse_from_rfc3339(&result.modified_at) {
                    let modified_utc = modified_at.with_timezone(&Utc);

                    if let Some(after_utc) = after_time {
                        if modified_utc < after_utc {
                            return false;
                        }
                    }

                    if let Some(before_utc) = before_time {
                        if modified_utc > before_utc {
                            return false;
                        }
                    }
                }
                true
            });

            context
                .step_results
                .push(("filter_temporal".to_string(), context.results.len()));

            tracing::debug!(
                "Temporal filter: {} → {} results",
                initial_count,
                context.results.len()
            );
        }

        WorkflowStep::FilterCollection { collections } => {
            let initial_count = context.results.len();

            context
                .results
                .retain(|result| collections.contains(&result.collection_name));

            context
                .step_results
                .push(("filter_collection".to_string(), context.results.len()));

            tracing::debug!(
                "Collection filter: {} → {} results",
                initial_count,
                context.results.len()
            );
        }

        WorkflowStep::Rerank { limit, query } => {
            if let Ok(reranker) = HttpReranker::from_env() {
                // Prepare documents for reranking
                let to_rerank: Vec<(usize, SearchResult)> = context
                    .results
                    .iter()
                    .enumerate()
                    .take(*limit)
                    .map(|(idx, result)| (idx, result.clone()))
                    .collect();

                if !to_rerank.is_empty() {
                    // Convert to RerankDocument
                    let rerank_docs: Vec<RerankDocument> = to_rerank
                        .iter()
                        .map(|(idx, result)| RerankDocument {
                            id: idx.to_string(),
                            text: format!(
                                "{} {}\n{}",
                                result.title,
                                result.llm_summary.as_deref().unwrap_or(""),
                                result
                                    .body
                                    .as_deref()
                                    .unwrap_or("")
                                    .chars()
                                    .take(500)
                                    .collect::<String>()
                            ),
                        })
                        .collect();

                    // Rerank
                    match reranker.rerank(query, &rerank_docs).await {
                        Ok(reranked) => {
                            // Map back to SearchResults in new order
                            let mut reranked_results = Vec::new();
                            for rr in reranked {
                                if let Ok(idx) = rr.id.parse::<usize>() {
                                    if let Some((_, mut result)) = to_rerank.get(idx).cloned() {
                                        // Update score with reranker score
                                        result.score = rr.score;
                                        reranked_results.push(result);
                                    }
                                }
                            }

                            // Replace with reranked results
                            context.results = reranked_results;

                            tracing::debug!(
                                "Reranked {} results using {}",
                                context.results.len(),
                                reranker.model_name()
                            );
                        }
                        Err(e) => {
                            tracing::warn!("Reranking failed: {}, keeping original order", e);
                            context.results = context.results.into_iter().take(*limit).collect();
                        }
                    }
                } else {
                    context.results = context.results.into_iter().take(*limit).collect();
                }
            } else {
                // No reranker available, just limit results
                context.results = context.results.into_iter().take(*limit).collect();
            }

            context
                .step_results
                .push(("rerank".to_string(), context.results.len()));
        }

        WorkflowStep::Deduplicate => {
            let initial_count = context.results.len();
            let mut seen = HashMap::new();
            let mut deduped = Vec::new();

            for result in context.results {
                if !seen.contains_key(&result.hash) {
                    seen.insert(result.hash.clone(), true);
                    deduped.push(result);
                }
            }

            context.results = deduped;
            context
                .step_results
                .push(("deduplicate".to_string(), context.results.len()));

            tracing::debug!(
                "Deduplication: {} → {} results",
                initial_count,
                context.results.len()
            );
        }

        WorkflowStep::Limit { count } => {
            context.results = context.results.into_iter().take(*count).collect();
            context
                .step_results
                .push(("limit".to_string(), context.results.len()));
        }

        WorkflowStep::ExpandQuery { original_query } => {
            // Expand query and perform searches for each variant
            if let Ok(expander) = HttpQueryExpander::from_env() {
                match expander.expand(original_query, None).await {
                    Ok(expanded) => {
                        let mut all_results = Vec::new();

                        // Search using lexical variations (BM25)
                        for variant in &expanded.lexical {
                            if variant != original_query {
                                let opts = base_options.clone();
                                match db.search_fts(variant, &opts) {
                                    Ok(mut results) => {
                                        all_results.append(&mut results);
                                    }
                                    Err(e) => {
                                        tracing::warn!(
                                            "ExpandQuery: Failed to search variant '{}': {}",
                                            variant,
                                            e
                                        );
                                    }
                                }
                            }
                        }

                        // Search using semantic variations (vector search)
                        if let Ok(embedder) = HttpEmbedder::from_env() {
                            for variant in &expanded.semantic {
                                if variant != original_query {
                                    let opts = base_options.clone();
                                    match db.search_vec(variant, &embedder, &opts).await {
                                        Ok(mut results) => {
                                            all_results.append(&mut results);
                                        }
                                        Err(e) => {
                                            tracing::warn!(
                                                "ExpandQuery: Failed to vector search variant '{}': {}",
                                                variant,
                                                e
                                            );
                                        }
                                    }
                                }
                            }
                        }

                        // Merge with existing results using RRF
                        context.results.append(&mut all_results);
                        context.results = merge_results_rrf(&context.results);

                        tracing::debug!(
                            "ExpandQuery: Expanded to {} lexical + {} semantic variants, merged {} results",
                            expanded.lexical.len(),
                            expanded.semantic.len(),
                            context.results.len()
                        );
                    }
                    Err(e) => {
                        tracing::warn!("ExpandQuery: Query expansion failed: {}", e);
                    }
                }
            } else {
                tracing::debug!("ExpandQuery: QueryExpander not available, skipping");
            }

            context
                .step_results
                .push(("expand_query".to_string(), context.results.len()));
        }

        WorkflowStep::GlossarySearch {
            query,
            limit,
            min_confidence,
        } => {
            // Search concepts using FTS
            let concepts = db.search_concepts(query, *limit)?;

            eprintln!("[DEBUG] GlossarySearch: query='{}', found {} concepts", query, concepts.len());
            
            if concepts.is_empty() {
                tracing::debug!("GlossarySearch: No concepts found for query '{}'", query);
            } else {
                tracing::debug!(
                    "GlossarySearch: Found {} concepts for query '{}'",
                    concepts.len(),
                    query
                );
                for c in &concepts {
                    eprintln!("[DEBUG]   - Concept: '{}' (id={})", c.term, c.id);
                }
            }

            let mut glossary_results = Vec::new();

            for concept in concepts {
                // Get chunks for each concept
                let chunk_infos = db.get_chunks_for_concept(concept.id)?;
                eprintln!("[DEBUG]   Concept '{}' has {} chunks", concept.term, chunk_infos.len());

                for chunk_info in chunk_infos {
                    eprintln!("[DEBUG]     Querying doc with hash: {}", &chunk_info.document_hash[..16]);
                    // Query document metadata directly from database
                    let doc_query = db.conn.query_row(
                        "SELECT d.collection, d.modified_at, d.llm_summary, d.llm_title, d.llm_keywords, d.llm_category, d.llm_difficulty
                         FROM documents d
                         WHERE d.hash = ?1 AND d.active = 1",
                        params![&chunk_info.document_hash],
                        |row| {
                            Ok((
                                row.get::<_, String>(0)?,  // collection
                                row.get::<_, String>(1)?,  // modified_at
                                row.get::<_, Option<String>>(2)?,  // llm_summary
                                row.get::<_, Option<String>>(3)?,  // llm_title
                                row.get::<_, Option<String>>(4)?,  // llm_keywords
                                row.get::<_, Option<String>>(5)?,  // llm_category
                                row.get::<_, Option<String>>(6)?,  // llm_difficulty
                            ))
                        },
                    );

                    match doc_query {
                        Ok((collection_name, modified_at, llm_summary, llm_title, llm_keywords_json, llm_category, llm_difficulty)) => {
                            eprintln!("[DEBUG]       Document found! path={}", chunk_info.document_path);
                        // Parse keywords JSON if present
                        let llm_keywords = llm_keywords_json
                            .and_then(|json| serde_json::from_str::<Vec<String>>(&json).ok());

                        let result = SearchResult {
                            filepath: chunk_info.document_path.clone(),
                            display_path: chunk_info.document_path.clone(),
                            title: chunk_info.document_title.clone(),
                            hash: chunk_info.document_hash.clone(),
                            collection_name,
                            modified_at,
                            body: Some(chunk_info.snippet.clone()),
                            body_length: chunk_info.snippet.len(),
                            docid: format!("#chunk-{}", &chunk_info.chunk_hash[..8]),
                            context: Some(format!("Found via concept: {}", concept.term)),
                            score: *min_confidence,
                            source: SearchSource::Glossary,
                            chunk_pos: None,
                            llm_summary,
                            llm_title,
                            llm_keywords,
                            llm_category,
                            llm_difficulty,
                            user_metadata: None,
                            // Chunk fields (glossary already provides chunk info)
                            is_chunk: true,
                            chunk_hash: Some(chunk_info.chunk_hash.clone()),
                            chunk_type: None,
                            chunk_breadcrumb: None,
                            chunk_start_line: None,
                            chunk_end_line: None,
                            chunk_language: None,
                            chunk_summary: None,
                            chunk_purpose: None,
                            chunk_concepts: Vec::new(),
                            chunk_labels: std::collections::HashMap::new(),
                        };
                        glossary_results.push(result);
                        }
                        Err(e) => {
                            tracing::warn!(
                                "Failed to query document metadata for hash {}: {}",
                                &chunk_info.document_hash,
                                e
                            );
                        }
                    }
                }
            }

            // Deduplicate by document hash
            let mut seen = std::collections::HashSet::new();
            let mut new_results: Vec<SearchResult> = glossary_results
                .into_iter()
                .filter(|r| seen.insert(r.hash.clone()))
                .take(*limit)
                .collect();

            let count = new_results.len();
            context.results.append(&mut new_results);
            context
                .step_results
                .push(("glossary_search".to_string(), count));

            tracing::debug!(
                "GlossarySearch: Returned {} unique documents",
                context.results.len()
            );
        }

        WorkflowStep::Merge { strategy } => {
            // Merge duplicate results using the specified strategy
            let initial_count = context.results.len();

            context.results = match strategy {
                MergeStrategy::Rrf => merge_results_rrf(&context.results),
                MergeStrategy::Interleave => merge_results_interleave(&context.results),
                MergeStrategy::Append => merge_results_append(&context.results),
            };

            context
                .step_results
                .push(("merge".to_string(), context.results.len()));

            tracing::debug!(
                "Merge ({:?}): {} → {} results",
                strategy,
                initial_count,
                context.results.len()
            );
        }
    }

    Ok(context)
}

/// Parse temporal expressions like "3 months ago", "2024-01-01"
fn parse_temporal_expression(expr: Option<&str>) -> Result<Option<DateTime<Utc>>> {
    let Some(expr) = expr else {
        return Ok(None);
    };

    let expr_trimmed = expr.trim();
    if expr_trimmed.is_empty() {
        return Ok(None);
    }

    // Try parsing as ISO date first
    if let Ok(dt) = DateTime::parse_from_rfc3339(expr_trimmed) {
        return Ok(Some(dt.with_timezone(&Utc)));
    }

    // Parse relative expressions
    let expr_lower = expr_trimmed.to_lowercase();

    if expr_lower.contains("ago") {
        let now = Utc::now();
        let now_naive = now.naive_utc();

        if expr_lower.contains("month") {
            if let Some(num) = extract_number(&expr_lower) {
                // Use proper month arithmetic with chrono
                let target_date = now_naive
                    .date()
                    .checked_sub_months(chrono::Months::new(num as u32))
                    .ok_or_else(|| {
                        crate::error::AgentRootError::Search(format!(
                            "Invalid month calculation: {} months ago",
                            num
                        ))
                    })?;
                let target_datetime = target_date.and_time(now_naive.time());
                return Ok(Some(DateTime::from_naive_utc_and_offset(
                    target_datetime,
                    Utc,
                )));
            } else {
                tracing::warn!("Failed to parse number from temporal expression: '{}'", expr);
                return Ok(None);
            }
        } else if expr_lower.contains("week") {
            if let Some(num) = extract_number(&expr_lower) {
                return Ok(Some(now - Duration::weeks(num)));
            } else {
                tracing::warn!("Failed to parse number from temporal expression: '{}'", expr);
                return Ok(None);
            }
        } else if expr_lower.contains("day") {
            if let Some(num) = extract_number(&expr_lower) {
                return Ok(Some(now - Duration::days(num)));
            } else {
                tracing::warn!("Failed to parse number from temporal expression: '{}'", expr);
                return Ok(None);
            }
        } else if expr_lower.contains("year") {
            if let Some(num) = extract_number(&expr_lower) {
                // Use proper year arithmetic with chrono
                let years = if num % 4 == 0 {
                    // Account for leap years (366 days)
                    num * 365 + (num / 4)
                } else {
                    num * 365 + ((num + 3) / 4)
                };
                return Ok(Some(now - Duration::days(years)));
            } else {
                tracing::warn!("Failed to parse number from temporal expression: '{}'", expr);
                return Ok(None);
            }
        } else {
            tracing::warn!(
                "Temporal expression '{}' contains 'ago' but no recognized time unit (day/week/month/year)",
                expr
            );
            return Ok(None);
        }
    }

    tracing::warn!("Unable to parse temporal expression: '{}'", expr);
    Ok(None)
}

/// Extract first number from string
fn extract_number(s: &str) -> Option<i64> {
    s.split_whitespace()
        .find_map(|word| word.parse::<i64>().ok())
}

/// Merge results using Reciprocal Rank Fusion (RRF)
fn merge_results_rrf(results: &[SearchResult]) -> Vec<SearchResult> {
    const RRF_K: f64 = 60.0;

    // Group results by hash
    let mut score_map: HashMap<String, (f64, SearchResult)> = HashMap::new();

    for (rank, result) in results.iter().enumerate() {
        let rrf_score = 1.0 / (rank as f64 + RRF_K);

        score_map
            .entry(result.hash.clone())
            .and_modify(|(score, _)| *score += rrf_score)
            .or_insert((rrf_score, result.clone()));
    }

    // Sort by combined RRF score
    let mut merged: Vec<(f64, SearchResult)> = score_map.into_values().collect();
    merged.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    merged
        .into_iter()
        .map(|(score, mut result)| {
            result.score = score;
            result
        })
        .collect()
}

/// Merge results by interleaving (round-robin)
fn merge_results_interleave(results: &[SearchResult]) -> Vec<SearchResult> {
    // Group by hash to deduplicate
    let mut seen = HashMap::new();
    let mut deduped = Vec::new();

    for result in results {
        if !seen.contains_key(&result.hash) {
            seen.insert(result.hash.clone(), true);
            deduped.push(result.clone());
        }
    }

    deduped
}

/// Merge results by appending (preserve order, deduplicate)
fn merge_results_append(results: &[SearchResult]) -> Vec<SearchResult> {
    let mut seen = HashMap::new();
    let mut merged = Vec::new();

    for result in results {
        if !seen.contains_key(&result.hash) {
            seen.insert(result.hash.clone(), true);
            merged.push(result.clone());
        }
    }

    merged
}
