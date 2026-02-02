//! LLM-based memory extraction from session histories

use crate::db::sessions::SessionQuery;
use crate::error::Result;
use crate::llm::client::{ChatMessage, LLMClient, VLLMClient};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExtractedMemory {
    pub category: String,
    pub content: String,
    pub confidence: f64,
}

pub struct MemoryExtractor {
    client: Arc<dyn LLMClient>,
}

const EXTRACTION_PROMPT: &str = r#"Analyze this agent session history and extract long-term memories worth retaining.

Categories:
- "preference": User preferences, style choices, tool preferences
- "entity": Named entities, project names, repo names, tech stack
- "pattern": Recurring patterns, common workflows, frequent requests
- "fact": Technical facts, project details, architecture decisions

Return a JSON array of memories. Each object must have:
- "category": one of the four categories above
- "content": concise memory statement (one sentence)
- "confidence": 0.0 to 1.0 indicating how confident this is a real memory

Return [] if no meaningful memories found.
Only return the JSON array, nothing else."#;

impl MemoryExtractor {
    pub fn new(client: Arc<dyn LLMClient>) -> Self {
        Self { client }
    }

    pub fn from_env() -> Result<Self> {
        let client = VLLMClient::from_env()?;
        Ok(Self {
            client: Arc::new(client),
        })
    }

    /// Extract memories from session query history and context.
    /// Returns empty vec on LLM failure (graceful degradation).
    pub async fn extract_memories(
        &self,
        queries: &[SessionQuery],
        context: &HashMap<String, String>,
    ) -> Vec<ExtractedMemory> {
        let session_summary = build_session_summary(queries, context);

        let messages = vec![
            ChatMessage::system(EXTRACTION_PROMPT),
            ChatMessage::user(session_summary),
        ];

        let response = match self.client.chat_completion(messages).await {
            Ok(r) => r,
            Err(e) => {
                tracing::debug!("Memory extraction LLM call failed: {}", e);
                return vec![];
            }
        };

        parse_extraction_response(&response)
    }
}

fn build_session_summary(queries: &[SessionQuery], context: &HashMap<String, String>) -> String {
    let mut parts = Vec::new();

    if !context.is_empty() {
        parts.push("Session context:".to_string());
        for (k, v) in context {
            parts.push(format!("  {}: {}", k, v));
        }
    }

    parts.push(format!("\nQueries ({}):", queries.len()));
    for q in queries.iter().take(50) {
        parts.push(format!("  - \"{}\" ({} results)", q.query, q.result_count));
    }

    parts.join("\n")
}

fn parse_extraction_response(response: &str) -> Vec<ExtractedMemory> {
    // Try to find JSON array in the response
    let trimmed = response.trim();
    let json_str = if let Some(start) = trimmed.find('[') {
        if let Some(end) = trimmed.rfind(']') {
            &trimmed[start..=end]
        } else {
            return vec![];
        }
    } else {
        return vec![];
    };

    let parsed: Vec<ExtractedMemory> = match serde_json::from_str(json_str) {
        Ok(v) => v,
        Err(e) => {
            tracing::debug!("Failed to parse memory extraction JSON: {}", e);
            return vec![];
        }
    };

    // Validate categories and confidence
    parsed
        .into_iter()
        .filter(|m| {
            matches!(
                m.category.as_str(),
                "preference" | "entity" | "pattern" | "fact"
            ) && (0.0..=1.0).contains(&m.confidence)
                && !m.content.is_empty()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_extraction_response_valid() {
        let response = r#"[
            {"category": "fact", "content": "Project uses Rust with SQLite", "confidence": 0.9},
            {"category": "preference", "content": "User prefers modular code", "confidence": 0.8}
        ]"#;

        let memories = parse_extraction_response(response);
        assert_eq!(memories.len(), 2);
        assert_eq!(memories[0].category, "fact");
        assert_eq!(memories[1].category, "preference");
        assert!((memories[0].confidence - 0.9).abs() < 0.001);
    }

    #[test]
    fn test_parse_extraction_response_invalid() {
        // Malformed JSON
        assert!(parse_extraction_response("not json").is_empty());
        // Missing array
        assert!(parse_extraction_response("{}").is_empty());
        // Invalid category
        let bad_cat = r#"[{"category": "invalid", "content": "test", "confidence": 0.5}]"#;
        assert!(parse_extraction_response(bad_cat).is_empty());
        // Confidence out of range
        let bad_conf = r#"[{"category": "fact", "content": "test", "confidence": 1.5}]"#;
        assert!(parse_extraction_response(bad_conf).is_empty());
        // Empty content
        let empty = r#"[{"category": "fact", "content": "", "confidence": 0.5}]"#;
        assert!(parse_extraction_response(empty).is_empty());
    }

    #[test]
    fn test_parse_extraction_response_with_surrounding_text() {
        let response = "Here are the memories:\n[{\"category\": \"fact\", \"content\": \"Uses Rust\", \"confidence\": 0.7}]\nDone.";
        let memories = parse_extraction_response(response);
        assert_eq!(memories.len(), 1);
    }

    #[test]
    fn test_build_session_summary() {
        let queries = vec![SessionQuery {
            query: "test query".to_string(),
            result_count: 5,
            top_results: vec![],
            created_at: "2024-01-01".to_string(),
        }];
        let mut ctx = HashMap::new();
        ctx.insert("project".to_string(), "agentroot".to_string());

        let summary = build_session_summary(&queries, &ctx);
        assert!(summary.contains("project: agentroot"));
        assert!(summary.contains("test query"));
    }
}
