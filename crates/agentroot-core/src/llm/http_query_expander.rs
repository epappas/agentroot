//! HTTP-based query expander using external LLM service

use super::{ChatMessage, ExpandedQuery, LLMClient, QueryExpander};
use crate::config::LLMServiceConfig;
use crate::error::Result;
use async_trait::async_trait;
use std::sync::Arc;

/// Query expander using external HTTP LLM service
pub struct HttpQueryExpander {
    client: Arc<dyn LLMClient>,
}

impl HttpQueryExpander {
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
impl QueryExpander for HttpQueryExpander {
    async fn expand(&self, query: &str, context: Option<&str>) -> Result<ExpandedQuery> {
        let prompt = build_expansion_prompt(query, context);

        let messages = vec![
            ChatMessage::system(
                "Expand search queries. Output ONLY JSON: {\"lexical\": [...], \"semantic\": [...], \"hyde\": \"...\" or null}"
            ),
            ChatMessage::user(prompt),
        ];

        let response = self.client.chat_completion(messages).await?;

        parse_expansion_response(&response)
    }

    fn model_name(&self) -> &str {
        self.client.model_name()
    }
}

fn build_expansion_prompt(query: &str, context: Option<&str>) -> String {
    let context_info = context
        .map(|c| format!(" Context: {}", c))
        .unwrap_or_default();

    format!(
        r#"Query: "{}"{} Expand for better search:

- lexical: 2-3 synonyms/related terms
- semantic: 2-3 rephrased questions  
- hyde: short hypothetical answer (50 words) or null

Output JSON only:
{{"lexical": [...], "semantic": [...], "hyde": "..." or null}}"#,
        query, context_info
    )
}

fn parse_expansion_response(response: &str) -> Result<ExpandedQuery> {
    // Extract JSON from response (handle markdown code blocks)
    let json_str = if let Some(start) = response.find('{') {
        if let Some(end) = response.rfind('}') {
            &response[start..=end]
        } else {
            response
        }
    } else {
        // No JSON found, return empty expansion
        return Ok(ExpandedQuery::default());
    };

    // Parse JSON
    let parsed_json: serde_json::Value = match serde_json::from_str(json_str) {
        Ok(json) => json,
        Err(e) => {
            tracing::warn!(
                "Failed to parse expansion JSON: {}, using empty expansion",
                e
            );
            tracing::debug!("Raw LLM response: {}", response);
            // Return empty expansion
            return Ok(ExpandedQuery::default());
        }
    };

    let lexical = if let Some(arr) = parsed_json["lexical"].as_array() {
        arr.iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect()
    } else {
        vec![]
    };

    let semantic = if let Some(arr) = parsed_json["semantic"].as_array() {
        arr.iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect()
    } else {
        vec![]
    };

    let hyde = parsed_json["hyde"]
        .as_str()
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty());

    Ok(ExpandedQuery {
        lexical,
        semantic,
        hyde,
    })
}
