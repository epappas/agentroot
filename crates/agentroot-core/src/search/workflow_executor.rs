//! Workflow execution engine - runs planned workflows step-by-step

use super::{hybrid_search, SearchOptions, SearchResult};
use crate::db::Database;
use crate::error::Result;
use crate::llm::{
    HttpEmbedder, HttpQueryExpander, HttpReranker, Workflow, WorkflowContext, WorkflowStep,
};
use chrono::{DateTime, Duration, Utc};
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

    Ok(context.results)
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

            context.results = db.search_fts(query, &opts)?;
            context
                .step_results
                .push(("bm25_search".to_string(), context.results.len()));
        }

        WorkflowStep::VectorSearch { query, limit } => {
            let embedder = HttpEmbedder::from_env()?;
            let mut opts = base_options.clone();
            opts.limit = *limit;

            context.results = db.search_vec(query, &embedder, &opts).await?;
            context
                .step_results
                .push(("vector_search".to_string(), context.results.len()));
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

            context.results = hybrid_search(
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

            context
                .step_results
                .push(("hybrid_search".to_string(), context.results.len()));
        }

        WorkflowStep::FilterMetadata {
            category,
            difficulty,
            tags,
            exclude_category,
            exclude_difficulty,
        } => {
            let initial_count = context.results.len();

            context.results.retain(|result| {
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

            context
                .step_results
                .push(("filter_metadata".to_string(), context.results.len()));

            tracing::debug!(
                "Metadata filter: {} → {} results",
                initial_count,
                context.results.len()
            );
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
                let to_rerank: Vec<SearchResult> =
                    context.results.iter().take(*limit).cloned().collect();
                // Reranking implementation would go here
                // For now, just take top results
                context.results = context.results.into_iter().take(*limit).collect();
            } else {
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

        WorkflowStep::ExpandQuery { .. } | WorkflowStep::Merge { .. } => {
            // These would require more complex state management
            // Not implemented in initial version
            tracing::warn!("Step {:?} not yet implemented", step);
        }
    }

    Ok(context)
}

/// Parse temporal expressions like "3 months ago", "2024-01-01"
fn parse_temporal_expression(expr: Option<&str>) -> Result<Option<DateTime<Utc>>> {
    let Some(expr) = expr else {
        return Ok(None);
    };

    // Try parsing as ISO date first
    if let Ok(dt) = DateTime::parse_from_rfc3339(expr) {
        return Ok(Some(dt.with_timezone(&Utc)));
    }

    // Parse relative expressions
    let expr_lower = expr.to_lowercase();

    if expr_lower.contains("ago") {
        let now = Utc::now();

        if expr_lower.contains("month") {
            if let Some(num) = extract_number(&expr_lower) {
                return Ok(Some(now - Duration::days(num * 30)));
            }
        } else if expr_lower.contains("week") {
            if let Some(num) = extract_number(&expr_lower) {
                return Ok(Some(now - Duration::weeks(num)));
            }
        } else if expr_lower.contains("day") {
            if let Some(num) = extract_number(&expr_lower) {
                return Ok(Some(now - Duration::days(num)));
            }
        } else if expr_lower.contains("year") {
            if let Some(num) = extract_number(&expr_lower) {
                return Ok(Some(now - Duration::days(num * 365)));
            }
        }
    }

    Ok(None)
}

/// Extract first number from string
fn extract_number(s: &str) -> Option<i64> {
    s.split_whitespace()
        .find_map(|word| word.parse::<i64>().ok())
}
