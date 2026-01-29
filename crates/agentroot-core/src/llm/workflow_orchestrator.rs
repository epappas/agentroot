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

    /// BM25 keyword search on chunks (functions, sections)
    Bm25ChunkSearch {
        query: String,
        #[serde(default = "default_limit")]
        limit: usize,
    },

    /// Vector semantic search on chunks
    VectorChunkSearch {
        query: String,
        #[serde(default = "default_limit")]
        limit: usize,
    },
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
1. "bm25_search": Keyword matching on documents (exact terms, fast)
2. "vector_search": Semantic similarity on documents (concepts, meanings)
3. "hybrid_search": Combines BM25 + vector on documents (best quality)
4. "bm25_chunk_search": Keyword matching on code chunks (functions, classes, sections)
5. "vector_chunk_search": Semantic similarity on code chunks
6. "glossary_search": Intelligent concept glossary (USE SPARINGLY - see guidelines)
7. "filter_metadata": Filter by category, difficulty, tags
8. "filter_temporal": Filter by date ranges
9. "filter_collection": Filter by specific collections
10. "expand_query": Generate query variations
11. "rerank": LLM reranking for quality
12. "deduplicate": Remove duplicate results
13. "merge": Combine results from multiple searches
14. "limit": Take top N results

Chunk Search Guidelines (IMPORTANT):
- Use chunk search for TECHNICAL/CODE queries (function names, class names, implementations)
- Use chunk search when query targets specific code constructs (::, ->, impl, fn, class, def)
- Chunk results include: breadcrumb, start/end line, language, purpose, concepts
- Prefer document search for conceptual/natural language queries
- You CAN combine document + chunk search with merge for comprehensive results

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
1. "bm25_search": Keyword matching on documents (only option without embeddings)
2. "bm25_chunk_search": Keyword matching on code chunks (functions, classes, sections)
3. "glossary_search": Intelligent concept glossary (USE SPARINGLY for abstract queries)
4. "filter_metadata": Filter by category, difficulty, tags
5. "filter_temporal": Filter by date ranges
6. "limit": Take top N results

Chunk Search Guidelines:
- Use chunk search for TECHNICAL/CODE queries targeting specific code constructs
- Prefer document search for conceptual/natural language queries

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
- SPECIFIC TERMS (function names, file names, ::, _) → Use BM25 chunk search
- TECHNICAL KEYWORDS (single technical words) → Use BM25
- CODE CONSTRUCTS (impl, fn, class, def, struct) → Use BM25 chunk search
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
    {{"step": "bm25_chunk_search", "query": "SourceProvider list_items", "limit": 20}}
  ],
  "reasoning": "Specific code reference - chunk search finds exact function/method definitions",
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

Query: "fn search_chunks"
Workflow:
{{
  "steps": [
    {{"step": "bm25_chunk_search", "query": "search_chunks", "limit": 20}}
  ],
  "reasoning": "Looking for a specific function - chunk search targets function-level code",
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

Query: "database connection implementation"
Workflow:
{{
  "steps": [
    {{"step": "bm25_chunk_search", "query": "database connection", "limit": 20}},
    {{"step": "bm25_search", "query": "database connection implementation", "limit": 20}},
    {{"step": "merge", "strategy": "rrf"}},
    {{"step": "deduplicate"}},
    {{"step": "limit", "count": 20}}
  ],
  "reasoning": "Technical implementation query - chunk search for code + document search for docs, merged",
  "expected_results": 20,
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
    let query_lower = query.to_lowercase();
    let is_nl = query_lower.contains("how")
        || query_lower.contains("what")
        || query_lower.contains("why");

    let is_code_query = query.contains("::")
        || query.contains("->")
        || query.contains("fn ")
        || query.contains("impl ")
        || query.contains("class ")
        || query.contains("def ")
        || query.contains("struct ")
        || is_pascal_or_snake_case(query);

    let steps = if !has_embeddings && is_code_query {
        // Technical query without embeddings: chunk BM25
        vec![WorkflowStep::Bm25ChunkSearch {
            query: query.to_string(),
            limit: 20,
        }]
    } else if !has_embeddings {
        vec![WorkflowStep::Bm25Search {
            query: query.to_string(),
            limit: 20,
        }]
    } else if is_code_query {
        // Technical query with embeddings: chunk BM25 + document BM25, merged
        vec![
            WorkflowStep::Bm25ChunkSearch {
                query: query.to_string(),
                limit: 20,
            },
            WorkflowStep::Bm25Search {
                query: query.to_string(),
                limit: 20,
            },
            WorkflowStep::Merge {
                strategy: MergeStrategy::Rrf,
            },
            WorkflowStep::Deduplicate,
            WorkflowStep::Limit { count: 20 },
        ]
    } else if is_nl {
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
        complexity: if is_code_query { "moderate" } else { "simple" }.to_string(),
    }
}

/// Check if the query contains PascalCase or snake_case identifiers
fn is_pascal_or_snake_case(query: &str) -> bool {
    query.split_whitespace().any(|word| {
        // snake_case: contains underscore with alphanumeric
        if word.contains('_') && word.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return true;
        }
        // PascalCase: starts with uppercase, has at least one lowercase after
        let chars: Vec<char> = word.chars().collect();
        if chars.len() >= 2
            && chars[0].is_uppercase()
            && chars.iter().skip(1).any(|c| c.is_lowercase())
            && chars.iter().skip(1).any(|c| c.is_uppercase())
        {
            return true;
        }
        false
    })
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

    #[test]
    fn test_parse_bm25_chunk_search_step() {
        let json = r#"{
            "steps": [
                {"step": "bm25_chunk_search", "query": "search_chunks", "limit": 15}
            ],
            "reasoning": "Code function lookup via chunk search",
            "expected_results": 15,
            "complexity": "simple"
        }"#;

        let workflow = parse_workflow_response(json).unwrap();
        assert_eq!(workflow.steps.len(), 1);
        match &workflow.steps[0] {
            WorkflowStep::Bm25ChunkSearch { query, limit } => {
                assert_eq!(query, "search_chunks");
                assert_eq!(*limit, 15);
            }
            other => panic!("Expected Bm25ChunkSearch, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_vector_chunk_search_step() {
        let json = r#"{
            "steps": [
                {"step": "vector_chunk_search", "query": "database connection", "limit": 10}
            ],
            "reasoning": "Semantic chunk search",
            "expected_results": 10,
            "complexity": "simple"
        }"#;

        let workflow = parse_workflow_response(json).unwrap();
        assert_eq!(workflow.steps.len(), 1);
        match &workflow.steps[0] {
            WorkflowStep::VectorChunkSearch { query, limit } => {
                assert_eq!(query, "database connection");
                assert_eq!(*limit, 10);
            }
            other => panic!("Expected VectorChunkSearch, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_mixed_doc_and_chunk_workflow() {
        let json = r#"{
            "steps": [
                {"step": "bm25_chunk_search", "query": "database", "limit": 20},
                {"step": "bm25_search", "query": "database implementation", "limit": 20},
                {"step": "merge", "strategy": "rrf"},
                {"step": "deduplicate"},
                {"step": "limit", "count": 20}
            ],
            "reasoning": "Combined chunk + document search for comprehensive results",
            "expected_results": 20,
            "complexity": "moderate"
        }"#;

        let workflow = parse_workflow_response(json).unwrap();
        assert_eq!(workflow.steps.len(), 5);
        assert!(matches!(&workflow.steps[0], WorkflowStep::Bm25ChunkSearch { .. }));
        assert!(matches!(&workflow.steps[1], WorkflowStep::Bm25Search { .. }));
        assert!(matches!(&workflow.steps[2], WorkflowStep::Merge { .. }));
        assert!(matches!(&workflow.steps[3], WorkflowStep::Deduplicate));
        assert!(matches!(&workflow.steps[4], WorkflowStep::Limit { count: 20 }));
    }

    #[test]
    fn test_fallback_workflow_code_query_no_embeddings() {
        let workflow = fallback_workflow("SourceProvider::list_items", false);
        assert_eq!(workflow.steps.len(), 1);
        assert!(matches!(&workflow.steps[0], WorkflowStep::Bm25ChunkSearch { .. }));
    }

    #[test]
    fn test_fallback_workflow_code_query_with_embeddings() {
        let workflow = fallback_workflow("fn search_chunks", true);
        // Should produce chunk + doc + merge + dedup + limit
        assert!(workflow.steps.len() >= 3);
        assert!(matches!(&workflow.steps[0], WorkflowStep::Bm25ChunkSearch { .. }));
        assert!(matches!(&workflow.steps[1], WorkflowStep::Bm25Search { .. }));
    }

    #[test]
    fn test_fallback_workflow_snake_case_query() {
        let workflow = fallback_workflow("search_chunks_bm25", false);
        assert!(matches!(&workflow.steps[0], WorkflowStep::Bm25ChunkSearch { .. }));
    }

    #[test]
    fn test_fallback_workflow_pascal_case_query() {
        let workflow = fallback_workflow("WorkflowStep", false);
        assert!(matches!(&workflow.steps[0], WorkflowStep::Bm25ChunkSearch { .. }));
    }

    #[test]
    fn test_fallback_workflow_natural_language() {
        let workflow = fallback_workflow("how to implement search", true);
        assert!(matches!(&workflow.steps[0], WorkflowStep::VectorSearch { .. }));
    }

    #[test]
    fn test_fallback_workflow_generic_query() {
        let workflow = fallback_workflow("search providers", true);
        assert!(matches!(&workflow.steps[0], WorkflowStep::HybridSearch { .. }));
    }

    #[test]
    fn test_is_pascal_or_snake_case() {
        assert!(is_pascal_or_snake_case("search_chunks"));
        assert!(is_pascal_or_snake_case("WorkflowStep"));
        assert!(is_pascal_or_snake_case("HttpReranker"));
        assert!(!is_pascal_or_snake_case("search"));
        assert!(!is_pascal_or_snake_case("how to search"));
        assert!(!is_pascal_or_snake_case("MCP")); // all caps is not PascalCase
    }

    #[test]
    fn test_chunk_search_step_default_limit() {
        let json = r#"{
            "steps": [
                {"step": "bm25_chunk_search", "query": "test"}
            ],
            "reasoning": "test",
            "expected_results": 20,
            "complexity": "simple"
        }"#;

        let workflow = parse_workflow_response(json).unwrap();
        match &workflow.steps[0] {
            WorkflowStep::Bm25ChunkSearch { limit, .. } => {
                assert_eq!(*limit, 20); // default_limit
            }
            other => panic!("Expected Bm25ChunkSearch, got {:?}", other),
        }
    }
}
