//! HTTP-based reranker using external LLM service

use super::{ChatMessage, LLMClient, RerankDocument, RerankResult, Reranker};
use crate::config::LLMServiceConfig;
use crate::error::Result;
use async_trait::async_trait;
use std::sync::Arc;

/// Reranker using external HTTP LLM service
pub struct HttpReranker {
    client: Arc<dyn LLMClient>,
}

impl HttpReranker {
    /// Create from LLM client
    pub fn new(client: Arc<dyn LLMClient>) -> Self {
        Self { client }
    }

    /// Create from configuration
    pub fn from_config(config: LLMServiceConfig) -> Result<Self> {
        let client = super::VLLMClient::new(config)?;
        Ok(Self {
            client: Arc::new(client),
        })
    }

    /// Create from environment variables
    pub fn from_env() -> Result<Self> {
        let client = super::VLLMClient::from_env()?;
        Ok(Self {
            client: Arc::new(client),
        })
    }
}

#[async_trait]
impl Reranker for HttpReranker {
    async fn rerank(&self, query: &str, documents: &[RerankDocument]) -> Result<Vec<RerankResult>> {
        // If no documents, return empty
        if documents.is_empty() {
            return Ok(vec![]);
        }

        // Limit to prevent token overflow
        let max_docs = 40;
        let docs_to_rerank = if documents.len() > max_docs {
            &documents[..max_docs]
        } else {
            documents
        };

        let prompt = build_reranking_prompt(query, docs_to_rerank);

        let messages = vec![
            ChatMessage::system(
                "You are a document relevance scorer. Score each document's relevance to the query from 0.0 to 1.0. \
                 Output ONLY valid JSON with a 'scores' array of objects with 'id' and 'score' fields."
            ),
            ChatMessage::user(prompt),
        ];

        let response = self.client.chat_completion(messages).await?;

        parse_reranking_response(&response, docs_to_rerank)
    }

    fn model_name(&self) -> &str {
        self.client.model_name()
    }
}

fn build_reranking_prompt(query: &str, documents: &[RerankDocument]) -> String {
    let mut prompt = format!(
        r#"Score these documents for relevance to the query. Rate from 0.0 (not relevant) to 1.0 (highly relevant).

Query: "{}"

Documents:
"#,
        query
    );

    for (idx, doc) in documents.iter().enumerate() {
        // Truncate document text to prevent token overflow
        let text = if doc.text.len() > 500 {
            format!("{}...", &doc.text[..500])
        } else {
            doc.text.clone()
        };

        prompt.push_str(&format!("\n[{}] ID: {}\nText: {}\n", idx, doc.id, text));
    }

    prompt.push_str(
        r#"
Output JSON with:
- scores: array of {{"id": "...", "score": 0.0-1.0}}

Example:
{{"scores": [{{"id": "abc", "score": 0.95}}, {{"id": "def", "score": 0.72}}, {{"id": "ghi", "score": 0.45}}]}}

Score all documents. Output only JSON:"#,
    );

    prompt
}

fn parse_reranking_response(
    response: &str,
    documents: &[RerankDocument],
) -> Result<Vec<RerankResult>> {
    // Extract JSON from response
    let json_str = if let Some(start) = response.find('{') {
        if let Some(end) = response.rfind('}') {
            &response[start..=end]
        } else {
            response
        }
    } else {
        // No JSON found, return default scores
        return Ok(documents
            .iter()
            .map(|doc| RerankResult {
                id: doc.id.clone(),
                score: 0.5,
            })
            .collect());
    };

    // Parse JSON
    let parsed_json: serde_json::Value = match serde_json::from_str(json_str) {
        Ok(json) => json,
        Err(e) => {
            tracing::warn!(
                "Failed to parse reranking JSON: {}, using fallback scores",
                e
            );
            tracing::debug!("Raw LLM response: {}", response);
            // Return fallback scores
            return Ok(documents
                .iter()
                .map(|doc| RerankResult {
                    id: doc.id.clone(),
                    score: 0.5,
                })
                .collect());
        }
    };

    let scores = if let Some(arr) = parsed_json["scores"].as_array() {
        arr.iter()
            .filter_map(|item| {
                let id = item["id"].as_str()?.to_string();
                let score = item["score"].as_f64()?;
                Some(RerankResult { id, score })
            })
            .collect()
    } else {
        // Fallback to default scores
        documents
            .iter()
            .map(|doc| RerankResult {
                id: doc.id.clone(),
                score: 0.5,
            })
            .collect()
    };

    Ok(scores)
}
