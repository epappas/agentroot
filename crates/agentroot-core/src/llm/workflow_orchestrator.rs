//! Dynamic workflow orchestration using ReAct pattern
//!
//! Instead of fixed strategies (BM25, Vector, Hybrid), the LLM plans
//! a custom workflow of operations for each query.

use super::{ChatMessage, LLMClient};
use crate::error::{AgentRootError, Result};
use crate::search::SearchResult;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Individual workflow step/operation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "step", rename_all = "snake_case")]
pub enum WorkflowStep {
    /// BM25 keyword search
    Bm25Search {
        query: String,
        #[serde(default = "default_limit")]
        limit: usize,
    },

    /// Vector semantic search
    VectorSearch {
        query: String,
        #[serde(default = "default_limit")]
        limit: usize,
    },

    /// Hybrid search (BM25 + Vector + RRF)
    HybridSearch {
        query: String,
        #[serde(default = "default_limit")]
        limit: usize,
        #[serde(default)]
        use_expansion: bool,
        #[serde(default)]
        use_reranking: bool,
    },

    /// Filter by metadata
    FilterMetadata {
        category: Option<String>,
        difficulty: Option<String>,
        tags: Option<Vec<String>>,
        exclude_category: Option<String>,
        exclude_difficulty: Option<String>,
    },

    /// Filter by temporal criteria
    FilterTemporal {
        after: Option<String>, // "2024-01-01" or "6 months ago"
        before: Option<String>,
    },

    /// Filter by collection
    FilterCollection { collections: Vec<String> },

    /// Expand query with variations
    ExpandQuery { original_query: String },

    /// Search intelligent glossary for semantic concepts
    GlossarySearch {
        query: String,
        #[serde(default = "default_limit")]
        limit: usize,
        #[serde(default = "default_glossary_confidence")]
        min_confidence: f64,
    },

    /// Rerank results
    Rerank {
        #[serde(default = "default_rerank_limit")]
        limit: usize,
        query: String,
    },

    /// Deduplicate results
    Deduplicate,

    /// Merge results from multiple searches
    Merge { strategy: MergeStrategy },

    /// Take top N results
    Limit { count: usize },
}

fn default_limit() -> usize {
    20
}
fn default_rerank_limit() -> usize {
    10
}
fn default_glossary_confidence() -> f64 {
    0.3
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MergeStrategy {
    /// Reciprocal Rank Fusion
    Rrf,
    /// Interleave results
    Interleave,
    /// Append (preserve order)
    Append,
}

/// Complete workflow planned by LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    /// Sequence of steps to execute
    pub steps: Vec<WorkflowStep>,

    /// LLM's reasoning for this workflow
    pub reasoning: String,

    /// Expected result count
    #[serde(default = "default_limit")]
    pub expected_results: usize,

    /// Query complexity (simple, moderate, complex)
    #[serde(default)]
    pub complexity: String,
}

/// Workflow execution context
pub struct WorkflowContext {
    /// Current results being processed
    pub results: Vec<SearchResult>,

    /// Original query
    pub query: String,

    /// Intermediate results from each step (for debugging)
    pub step_results: Vec<(String, usize)>, // (step_name, result_count)
}

impl WorkflowContext {
    pub fn new(query: String) -> Self {
        Self {
            results: Vec::new(),
            query,
            step_results: Vec::new(),
        }
    }
}

/// LLM-based workflow orchestrator
pub struct WorkflowOrchestrator {
    client: Arc<dyn LLMClient>,
}

impl WorkflowOrchestrator {
    /// Create from LLM client
    pub fn new(client: Arc<dyn LLMClient>) -> Self {
        Self { client }
    }

    /// Create from environment variables
    pub fn from_env() -> Result<Self> {
        let client = super::VLLMClient::from_env()?;
        Ok(Self {
            client: Arc::new(client),
        })
    }

    /// Plan workflow for query using LLM
    pub async fn plan_workflow(&self, query: &str, has_embeddings: bool) -> Result<Workflow> {
        let prompt = build_workflow_prompt(query, has_embeddings);

        let messages = vec![
            ChatMessage::system(
                "You are a search workflow planner. Design optimal multi-step search workflows. Output ONLY JSON."
            ),
            ChatMessage::user(prompt),
        ];

        let response = self.client.chat_completion(messages).await?;

        parse_workflow_response(&response)
    }
}

/// Build LLM prompt for workflow planning
fn build_workflow_prompt(query: &str, has_embeddings: bool) -> String {
    let available_ops = if has_embeddings {
        r#"Available operations:
1. "bm25_search": Keyword matching (exact terms, fast)
2. "vector_search": Semantic similarity (concepts, meanings)
3. "hybrid_search": Combines BM25 + vector (best quality)
4. "glossary_search": Intelligent concept glossary (USE SPARINGLY - see guidelines)
5. "filter_metadata": Filter by category, difficulty, tags
6. "filter_temporal": Filter by date ranges
7. "filter_collection": Filter by specific collections
8. "expand_query": Generate query variations
9. "rerank": LLM reranking for quality
10. "deduplicate": Remove duplicate results
11. "merge": Combine results from multiple searches
12. "limit": Take top N results

GlossarySearch Guidelines (IMPORTANT):
- USE SPARINGLY - glossary is a SUPPLEMENTARY aid, NOT a primary search mechanism
- Use ONLY for abstract/exploratory queries (e.g., "distributed systems", "orchestration")
- DO NOT use for:
  * Specific technical terms (function/class names)
  * Exact code references
  * Simple keyword lookups
- Glossary finds semantically related content through concept relationships
- Example: query "orchestrator" → finds docs about "kubernetes", "container management"
- Typical placement: AFTER primary search to expand with related concepts"#
    } else {
        r#"Available operations:
1. "bm25_search": Keyword matching (only option without embeddings)
2. "glossary_search": Intelligent concept glossary (USE SPARINGLY for abstract queries)
3. "filter_metadata": Filter by category, difficulty, tags
4. "filter_temporal": Filter by date ranges
5. "limit": Take top N results

GlossarySearch Guidelines:
- USE SPARINGLY - for abstract/exploratory queries only
- DO NOT use for specific technical terms or exact matches"#
    };

    format!(
        r#"Plan an optimal search workflow for this query:

Query: "{}"

{}

Design a workflow (sequence of operations) that best answers this query.

Guidelines:
- Start with appropriate search operation(s)
- Add filters if query mentions categories, difficulty, or dates
- Use reranking for quality if available
- Keep workflows simple (2-5 steps usually sufficient)
- Consider query complexity and user intent

CRITICAL: Choose search strategy based on query type:
- ACRONYMS (MCP, API, CLI, etc.) → Use BM25 for exact matching
- SPECIFIC TERMS (function names, file names, ::, _) → Use BM25
- TECHNICAL KEYWORDS (single technical words) → Use BM25
- CONCEPTUAL/NATURAL LANGUAGE → Use vector or hybrid
- "does X have Y?" where Y is specific → Use BM25 to find Y

Examples:

Query: "recent tutorials about providers"
Workflow:
{{
  "steps": [
    {{"step": "vector_search", "query": "providers tutorials", "limit": 30}},
    {{"step": "filter_metadata", "category": "tutorial"}},
    {{"step": "filter_temporal", "after": "3 months ago"}},
    {{"step": "rerank", "limit": 10, "query": "providers tutorials"}}
  ],
  "reasoning": "Semantic search for concepts, filter by metadata and recency, rerank for quality",
  "expected_results": 10,
  "complexity": "moderate"
}}

Query: "SourceProvider::list_items"
Workflow:
{{
  "steps": [
    {{"step": "bm25_search", "query": "SourceProvider::list_items", "limit": 20}}
  ],
  "reasoning": "Exact technical term - BM25 keyword matching is optimal",
  "expected_results": 20,
  "complexity": "simple"
}}

Query: "does agentroot have mcp?"
Workflow:
{{
  "steps": [
    {{"step": "bm25_search", "query": "mcp", "limit": 20}}
  ],
  "reasoning": "Query asks about specific acronym 'MCP' - BM25 will find exact keyword matches in titles/content",
  "expected_results": 20,
  "complexity": "simple"
}}

Query: "MCP server setup"
Workflow:
{{
  "steps": [
    {{"step": "bm25_search", "query": "MCP server setup", "limit": 20}}
  ],
  "reasoning": "Specific technical term 'MCP' with keywords - BM25 for exact matching",
  "expected_results": 20,
  "complexity": "simple"
}}

Query: "how to implement custom providers but not beginner level"
Workflow:
{{
  "steps": [
    {{"step": "vector_search", "query": "implement custom providers", "limit": 40}},
    {{"step": "filter_metadata", "category": "tutorial", "exclude_difficulty": "beginner"}},
    {{"step": "rerank", "limit": 10, "query": "implement custom providers"}}
  ],
  "reasoning": "Natural language with constraints - semantic search + metadata filtering + reranking",
  "expected_results": 10,
  "complexity": "moderate"
}}

Query: "distributed systems architecture patterns"
Workflow:
{{
  "steps": [
    {{"step": "hybrid_search", "query": "distributed systems architecture patterns", "limit": 30}},
    {{"step": "glossary_search", "query": "distributed systems", "limit": 20}},
    {{"step": "merge", "strategy": "rrf"}},
    {{"step": "rerank", "limit": 15, "query": "distributed systems architecture"}}
  ],
  "reasoning": "Abstract exploratory query - hybrid search + glossary for concept relationships + merge + rerank",
  "expected_results": 15,
  "complexity": "moderate"
}}

Output ONLY JSON (no markdown, no explanation):
{{
  "steps": [...],
  "reasoning": "...",
  "expected_results": N,
  "complexity": "simple" | "moderate" | "complex"
}}"#,
        query, available_ops
    )
}

/// Parse LLM response into Workflow
fn parse_workflow_response(response: &str) -> Result<Workflow> {
    // Extract JSON from response (handle markdown code blocks)
    let json_str = if response.contains("```json") {
        response
            .split("```json")
            .nth(1)
            .and_then(|s| s.split("```").next())
            .unwrap_or(response)
    } else if response.contains("```") {
        response
            .split("```")
            .nth(1)
            .and_then(|s| s.split("```").next())
            .unwrap_or(response)
    } else {
        response
    }
    .trim();

    serde_json::from_str(json_str).map_err(|e| {
        tracing::warn!("Failed to parse workflow response: {}", e);
        tracing::debug!("Response was: {}", response);
        AgentRootError::Llm(format!("Invalid workflow JSON: {}", e))
    })
}

/// Fallback workflow when LLM unavailable
pub fn fallback_workflow(query: &str, has_embeddings: bool) -> Workflow {
    // Simple heuristic-based workflow
    let is_nl = query.to_lowercase().contains("how")
        || query.to_lowercase().contains("what")
        || query.to_lowercase().contains("why");

    let has_tech = query.contains("::") || query.contains('_');

    let steps = if !has_embeddings {
        vec![WorkflowStep::Bm25Search {
            query: query.to_string(),
            limit: 20,
        }]
    } else if is_nl && !has_tech {
        vec![WorkflowStep::VectorSearch {
            query: query.to_string(),
            limit: 20,
        }]
    } else {
        vec![WorkflowStep::HybridSearch {
            query: query.to_string(),
            limit: 20,
            use_expansion: false,
            use_reranking: false,
        }]
    };

    Workflow {
        steps,
        reasoning: "Fallback workflow (LLM unavailable)".to_string(),
        expected_results: 20,
        complexity: "simple".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_workflow_simple() {
        let json = r#"{
            "steps": [
                {"step": "bm25_search", "query": "test", "limit": 10}
            ],
            "reasoning": "Simple keyword search",
            "expected_results": 10,
            "complexity": "simple"
        }"#;

        let workflow = parse_workflow_response(json).unwrap();
        assert_eq!(workflow.steps.len(), 1);
        assert_eq!(workflow.expected_results, 10);
    }

    #[test]
    fn test_parse_workflow_complex() {
        let json = r#"{
            "steps": [
                {"step": "vector_search", "query": "providers", "limit": 30},
                {"step": "filter_metadata", "category": "tutorial"},
                {"step": "rerank", "limit": 10, "query": "providers"}
            ],
            "reasoning": "Multi-step with filtering",
            "expected_results": 10,
            "complexity": "moderate"
        }"#;

        let workflow = parse_workflow_response(json).unwrap();
        assert_eq!(workflow.steps.len(), 3);
    }
}
