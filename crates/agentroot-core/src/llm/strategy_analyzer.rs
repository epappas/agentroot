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

/// Strategy analysis result from LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyAnalysis {
    /// Recommended search strategy
    pub strategy: SearchStrategy,
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
            confidence: 0.5,
            reasoning: "Fallback to hybrid search".to_string(),
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
    pub async fn analyze(&self, query: &str) -> Result<StrategyAnalysis> {
        let prompt = build_strategy_prompt(query);

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
fn build_strategy_prompt(query: &str) -> String {
    format!(
        r#"Analyze this search query and recommend the optimal search strategy.

Query: "{}"

Available strategies:
- "bm25": Keyword matching (fast, exact terms). Best for: exact code symbols, file names, specific technical terms.
- "vector": Semantic similarity (understands meaning). Best for: natural language questions, concepts, "how to" queries, multilingual.
- "hybrid": Combines both + reranking (highest quality). Best for: mixed queries with technical terms + natural language.

Consider:
1. Language: Is it natural language question or technical terms?
2. Intent: Looking for concepts or exact matches?
3. Multilingual: Is query in non-English language?
4. Specificity: Broad concept or specific code element?

Output ONLY this JSON (no markdown, no explanation):
{{
  "strategy": "bm25" | "vector" | "hybrid",
  "confidence": 0.0-1.0,
  "reasoning": "brief explanation",
  "is_multilingual": true | false
}}"#,
        query
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
            confidence: 1.0,
            reasoning: "No embeddings available".to_string(),
            is_multilingual: false,
        };
    }

    let is_nl = is_natural_language_heuristic(query);
    let has_tech = has_technical_terms_heuristic(query);

    if is_nl && !has_tech {
        StrategyAnalysis {
            strategy: SearchStrategy::Vector,
            confidence: 0.7,
            reasoning: "Natural language query detected (heuristic)".to_string(),
            is_multilingual: false,
        }
    } else {
        StrategyAnalysis {
            strategy: SearchStrategy::Hybrid,
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
fn has_technical_terms_heuristic(query: &str) -> bool {
    // Look for Rust path separators
    if query.contains("::") {
        return true;
    }

    // Look for snake_case (underscores between lowercase letters)
    if query.contains('_') {
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
        let json = r#"{"strategy": "vector", "confidence": 0.9, "reasoning": "Natural language question", "is_multilingual": false}"#;
        let result = parse_strategy_response(json).unwrap();
        assert_eq!(result.strategy, SearchStrategy::Vector);
        assert_eq!(result.confidence, 0.9);
    }

    #[test]
    fn test_parse_strategy_with_markdown() {
        let response = r#"```json
{"strategy": "hybrid", "confidence": 0.8, "reasoning": "Mixed query", "is_multilingual": false}
```"#;
        let result = parse_strategy_response(response).unwrap();
        assert_eq!(result.strategy, SearchStrategy::Hybrid);
    }
}
