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

        // Limit to prevent token overflow (reduced to 10 for LLM reliability)
        let max_docs = 10;
        let docs_to_rerank = if documents.len() > max_docs {
            &documents[..max_docs]
        } else {
            documents
        };

        let prompt = build_reranking_prompt(query, docs_to_rerank);

        let messages = vec![
            ChatMessage::system(
                "Score document relevance to query. Output ONLY JSON: {\"scores\": [{\"id\": \"...\", \"score\": 0.0-1.0}, ...]}"
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
        r#"Q: "{}"
Docs:
"#,
        query
    );

    for (idx, doc) in documents.iter().enumerate() {
        // Truncate document text very aggressively to prevent token overflow
        let text = if doc.text.len() > 100 {
            &doc.text[..100]
        } else {
            &doc.text
        };

        prompt.push_str(&format!("[{}] {}\n", idx, text));
    }

    prompt.push_str(
        r#"
Score 0-1 JSON:
{"scores":[0.0,...]}
"#,
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

    // Handle simplified format: {"scores": [0.9, 0.7, ...]} by index
    let scores = if let Some(arr) = parsed_json["scores"].as_array() {
        documents
            .iter()
            .enumerate()
            .map(|(idx, doc)| {
                let score = arr.get(idx).and_then(|v| v.as_f64()).unwrap_or(0.5);
                RerankResult {
                    id: doc.id.clone(),
                    score,
                }
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
