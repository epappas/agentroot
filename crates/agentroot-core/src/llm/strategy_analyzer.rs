//! LLM-based query strategy analyzer - determines optimal search strategy

use super::{ChatMessage, LLMClient};
use crate::error::{AgentRootError, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Search strategy recommendation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchStrategy {
    /// BM25 full-text search (keyword matching)
    Bm25,
    /// Vector similarity search (semantic understanding)
    Vector,
    /// Hybrid search (combines BM25 + vector + reranking)
    Hybrid,
}

/// Search granularity - documents vs chunks
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchGranularity {
    /// Search at document level (whole files)
    Document,
    /// Search at chunk level (functions, sections)
    Chunk,
    /// Search both and merge results
    Both,
}

/// Strategy analysis result from LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyAnalysis {
    /// Recommended search strategy
    pub strategy: SearchStrategy,
    /// Recommended search granularity (document vs chunk)
    pub granularity: SearchGranularity,
    /// Confidence score (0.0 - 1.0)
    pub confidence: f64,
    /// Brief reasoning (for debugging/logging)
    pub reasoning: String,
    /// Whether query is multilingual or non-English
    pub is_multilingual: bool,
}

impl Default for StrategyAnalysis {
    fn default() -> Self {
        Self {
            strategy: SearchStrategy::Hybrid,
            granularity: SearchGranularity::Document,
            confidence: 0.5,
            reasoning: "Fallback to hybrid document search".to_string(),
            is_multilingual: false,
        }
    }
}

/// LLM-based strategy analyzer
pub struct HttpStrategyAnalyzer {
    client: Arc<dyn LLMClient>,
}

impl HttpStrategyAnalyzer {
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

    /// Analyze query and recommend search strategy
    pub async fn analyze(&self, query: &str, language_context: Option<&str>) -> Result<StrategyAnalysis> {
        let prompt = build_strategy_prompt(query, language_context);

        let messages = vec![
            ChatMessage::system(
                "Analyze search queries and recommend optimal search strategy. Output ONLY JSON.",
            ),
            ChatMessage::user(prompt),
        ];

        let response = self.client.chat_completion(messages).await?;

        parse_strategy_response(&response)
    }
}

/// Build LLM prompt for strategy analysis
fn build_strategy_prompt(query: &str, language_context: Option<&str>) -> String {
    let context_info = if let Some(lang) = language_context {
        format!("\nCodebase Language: {}\n", lang)
    } else {
        String::new()
    };

    format!(
        r#"Analyze this search query and recommend the optimal search strategy and granularity.

Query: "{}"{}

Available strategies:
- "bm25": Keyword matching (fast, exact terms). Best for: exact code symbols, file names, specific technical terms, programming keywords.
- "vector": Semantic similarity (understands meaning). Best for: natural language questions, abstract concepts, "how to" queries, multilingual.
- "hybrid": Combines both + reranking (highest quality). Best for: mixed queries with technical terms + natural language.

IMPORTANT FOR CODE SEARCHES:
If the codebase language is provided, treat language-specific keywords as technical terms requiring BM25 or hybrid:
- Rust: trait, impl, struct, enum, fn, mod, use, pub, async, mut, const, static, etc.
- Python: class, def, async, await, import, from, lambda, etc.
- JavaScript/TypeScript: function, class, const, let, async, await, import, export, etc.
- Go: func, struct, interface, type, package, import, etc.

These keywords should use BM25 or hybrid (NOT vector-only), even if they appear as single words.

Available granularities:
- "document": Search whole files/documents. Best for: feature discovery, "does X have Y?", concept questions, broad exploration.
- "chunk": Search code chunks (functions, sections). Best for: finding specific implementations, "show me function X", debugging specific code.
- "both": Search documents then extract relevant chunks. Best for: answering questions with evidence, explaining "why/how".

Consider:
1. Query intent: Feature discovery? Code lookup? Debugging? Concept understanding?
2. Expected answer: Does user want to know "what/why" (documents) or find exact code (chunks)?
3. Language: Natural language question or technical code terms?
4. Specificity: Broad question or specific function/implementation?

IMPORTANT: Distinguish between feature discovery and code lookup:

Feature Discovery (use "document"):
- "does X have Y?" → Need to check documentation/README
- "what is X?" → Need explanation from docs
- "how to use X?" → Need tutorial/guide
- "X feature?" → Asking if feature exists

Code Lookup (use "chunk"):
- "show me X function" → Need specific implementation
- "find X method" → Need exact code
- "X::Y" (with ::) → Looking for specific code path
- "fn X" → Looking for function definition

Examples:
- "does agentroot have mcp?" → document (asking IF feature exists, check README)
- "show me search_fts function" → chunk (want specific implementation)
- "how to use providers?" → both (need tutorial + examples)
- "VLLMClient::embed method" → chunk (specific method implementation)
- "what features does X have?" → document (broad feature list)
- "MCP server implementation" → chunk (specific code)

Output ONLY this JSON (no markdown, no explanation):
{{
  "strategy": "bm25" | "vector" | "hybrid",
  "granularity": "document" | "chunk" | "both",
  "confidence": 0.0-1.0,
  "reasoning": "brief explanation",
  "is_multilingual": true | false
}}"#,
        query,
        context_info
    )
}

/// Parse LLM response into StrategyAnalysis
fn parse_strategy_response(response: &str) -> Result<StrategyAnalysis> {
    // Try to extract JSON from response (handle markdown code blocks)
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
        tracing::warn!("Failed to parse strategy response: {}", e);
        tracing::debug!("Response was: {}", response);
        AgentRootError::Llm(format!("Invalid strategy analysis JSON: {}", e))
    })
}

/// Heuristic fallback for strategy selection (when LLM unavailable)
pub fn heuristic_strategy(query: &str, has_embeddings: bool) -> StrategyAnalysis {
    if !has_embeddings {
        return StrategyAnalysis {
            strategy: SearchStrategy::Bm25,
            granularity: SearchGranularity::Document,
            confidence: 1.0,
            reasoning: "No embeddings available".to_string(),
            is_multilingual: false,
        };
    }

    let is_nl = is_natural_language_heuristic(query);
    let has_tech = has_technical_terms_heuristic(query);
    
    // Decide granularity based on query pattern
    let granularity = if has_tech || query.contains("fn ") || query.contains("impl ") || query.contains("struct ") {
        // Technical query likely looking for specific code
        SearchGranularity::Chunk
    } else if is_nl {
        // Natural language likely asking about features/concepts
        SearchGranularity::Document
    } else {
        // Mixed or unclear - search both
        SearchGranularity::Both
    };

    if is_nl && !has_tech {
        StrategyAnalysis {
            strategy: SearchStrategy::Vector,
            granularity,
            confidence: 0.7,
            reasoning: "Natural language query detected (heuristic)".to_string(),
            is_multilingual: false,
        }
    } else {
        StrategyAnalysis {
            strategy: SearchStrategy::Hybrid,
            granularity,
            confidence: 0.6,
            reasoning: "Mixed or technical query (heuristic)".to_string(),
            is_multilingual: false,
        }
    }
}

/// Improved natural language detection (heuristic fallback)
fn is_natural_language_heuristic(query: &str) -> bool {
    let nl_indicators = [
        "how to",
        "how do",
        "how can",
        "how should",
        "how would",
        "how does",
        "what is",
        "what are",
        "what does",
        "what's",
        "why does",
        "why do",
        "why is",
        "why are",
        "when should",
        "when do",
        "when is",
        "where can",
        "where do",
        "where is",
        "who is",
        "who are",
        "explain",
        "show me",
        "help me",
        "tell me",
        "tutorial",
        "guide",
        "example",
        "learn",
    ];

    let lower = query.to_lowercase();
    nl_indicators
        .iter()
        .any(|indicator| lower.contains(indicator))
}

/// Improved technical terms detection (heuristic fallback)
/// Uses language-agnostic patterns instead of hardcoded keywords
fn has_technical_terms_heuristic(query: &str) -> bool {
    // Look for Rust/C++ path separators
    if query.contains("::") {
        return true;
    }

    // Look for snake_case (underscores between lowercase letters)
    if query.contains('_') {
        return true;
    }

    // Look for type annotations/arrows
    if query.contains("->") || query.contains("=>") {
        return true;
    }

    // Look for generic syntax
    if query.contains('<') && query.contains('>') {
        return true;
    }

    // Look for PascalCase (capital letter followed by lowercase, then capital)
    // But exclude: normal sentences, acronyms like "I", single words like "Error"
    let words: Vec<&str> = query.split_whitespace().collect();
    for word in &words {
        // Skip short words and common English words
        if word.len() <= 2 {
            continue;
        }

        // Count uppercase letters
        let uppercase_count = word.chars().filter(|c| c.is_uppercase()).count();

        // PascalCase: Multiple capitals but not all capitals (e.g., SourceProvider, HttpClient)
        if uppercase_count >= 2 && uppercase_count < word.len() {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heuristic_natural_language() {
        assert!(is_natural_language_heuristic("how to add custom providers"));
        assert!(is_natural_language_heuristic("how can I use agentroot?"));
        assert!(is_natural_language_heuristic("what is a provider"));
        assert!(is_natural_language_heuristic("explain semantic chunking"));
        assert!(!is_natural_language_heuristic("SourceProvider trait"));
        assert!(!is_natural_language_heuristic("search_fts method"));
    }

    #[test]
    fn test_heuristic_technical_terms() {
        assert!(has_technical_terms_heuristic("SourceProvider trait"));
        assert!(has_technical_terms_heuristic("HttpEmbedder::from_env"));
        assert!(has_technical_terms_heuristic("search_fts method"));
        assert!(has_technical_terms_heuristic("FileProvider implementation"));
        assert!(!has_technical_terms_heuristic("how can I use agentroot?"));
        assert!(!has_technical_terms_heuristic("I want to search documents"));
    }

    #[test]
    fn test_parse_strategy_response() {
        let json = r#"{"strategy": "vector", "granularity": "document", "confidence": 0.9, "reasoning": "Natural language question", "is_multilingual": false}"#;
        let result = parse_strategy_response(json).unwrap();
        assert_eq!(result.strategy, SearchStrategy::Vector);
        assert_eq!(result.granularity, SearchGranularity::Document);
        assert_eq!(result.confidence, 0.9);
    }

    #[test]
    fn test_parse_strategy_with_markdown() {
        let response = r#"```json
{"strategy": "hybrid", "granularity": "chunk", "confidence": 0.8, "reasoning": "Mixed query", "is_multilingual": false}
```"#;
        let result = parse_strategy_response(response).unwrap();
        assert_eq!(result.strategy, SearchStrategy::Hybrid);
        assert_eq!(result.granularity, SearchGranularity::Chunk);
    }
}
