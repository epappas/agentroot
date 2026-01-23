//! Orchestrated search - LLM plans and executes dynamic workflows
//!
//! Uses ReAct pattern where LLM:
//! 1. Reasons about the query
//! 2. Plans a custom workflow
//! 3. Observes intermediate results
//! 4. Adapts as needed

use super::{execute_workflow, parse_metadata_filters, SearchOptions, SearchResult};
use crate::db::Database;
use crate::error::Result;
use crate::llm::{fallback_workflow, WorkflowOrchestrator};

/// Orchestrated search with dynamic workflow planning
///
/// The LLM builds a custom workflow for each query instead of choosing
/// from fixed strategies. Supports complex multi-step queries and adapts
/// to query complexity.
///
/// Examples of dynamic workflows:
/// - "recent tutorials about X" → Vector + filter(category) + filter(temporal) + rerank
/// - "SourceProvider::method" → BM25 (simple exact match)
/// - "compare X vs Y" → Multiple vector searches + merge + rerank
pub async fn orchestrated_search(
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

    // Check if embeddings are available
    let has_embeddings = db.has_vector_index();

    if !has_embeddings {
        // No embeddings → Simple BM25 workflow
        tracing::info!("No embeddings available, using BM25 workflow");
        let workflow = fallback_workflow(&clean_query, has_embeddings);
        return execute_workflow(db, &workflow, &clean_query, &enhanced_options).await;
    }

    // Try LLM-based workflow planning
    match WorkflowOrchestrator::from_env() {
        Ok(orchestrator) => {
            match orchestrator
                .plan_workflow(&clean_query, has_embeddings)
                .await
            {
                Ok(workflow) => {
                    tracing::info!(
                        "LLM Workflow: {} steps (complexity: {}, reasoning: {})",
                        workflow.steps.len(),
                        workflow.complexity,
                        workflow.reasoning
                    );

                    execute_workflow(db, &workflow, &clean_query, &enhanced_options).await
                }
                Err(e) => {
                    tracing::warn!("Workflow planning failed: {}, using fallback", e);
                    let workflow = fallback_workflow(&clean_query, has_embeddings);
                    execute_workflow(db, &workflow, &clean_query, &enhanced_options).await
                }
            }
        }
        Err(e) => {
            // LLM not configured, use fallback workflow
            tracing::debug!("LLM not configured, using fallback workflow: {}", e);
            let workflow = fallback_workflow(&clean_query, has_embeddings);
            execute_workflow(db, &workflow, &clean_query, &enhanced_options).await
        }
    }
}
