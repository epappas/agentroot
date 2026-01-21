//! HTTP-based query parser using external LLM service

use super::{ChatMessage, LLMClient, ParsedQuery, SearchType, TemporalFilter};
use crate::config::LLMServiceConfig;
use crate::error::{AgentRootError, Result};
use chrono::{Duration, Utc};
use std::sync::Arc;

/// Query parser using external HTTP LLM service
pub struct HttpQueryParser {
    client: Arc<dyn LLMClient>,
}

impl HttpQueryParser {
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

    /// Parse natural language query
    pub async fn parse(&self, query: &str) -> Result<ParsedQuery> {
        let prompt = build_query_parsing_prompt(query);

        let messages = vec![
            ChatMessage::system(
                "You are a search query parser. Extract structured information from user queries. \
                 Output ONLY valid JSON with these fields: \
                 search_terms (string), temporal_filter (object or null), metadata_filters (array), \
                 search_type (bm25/vector/hybrid), confidence (0.0-1.0)"
            ),
            ChatMessage::user(prompt),
        ];

        let response = self.client.chat_completion(messages).await?;

        parse_query_response(&response, query)
    }
}

fn build_query_parsing_prompt(query: &str) -> String {
    format!(
        r#"Parse this search query and extract structured information:

Query: "{}"

Output JSON with:
- search_terms: main keywords (string)
- temporal_filter: {{"description": "...", "relative_hours": N}} or null
- metadata_filters: [{{"field": "...", "value": "...", "operator": "contains"}}] or []
- search_type: "bm25" | "vector" | "hybrid"
- confidence: 0.0-1.0

Examples:
Input: "files that were edit recently"
Output: {{"search_terms": "files", "temporal_filter": {{"description": "recently", "relative_hours": 24}}, "metadata_filters": [], "search_type": "hybrid", "confidence": 0.9}}

Input: "rust code by Alice from last week"
Output: {{"search_terms": "rust code", "temporal_filter": {{"description": "last week", "relative_hours": 168}}, "metadata_filters": [{{"field": "author", "value": "Alice", "operator": "contains"}}], "search_type": "hybrid", "confidence": 0.95}}

Input: "python functions"
Output: {{"search_terms": "python functions", "temporal_filter": null, "metadata_filters": [], "search_type": "hybrid", "confidence": 0.85}}

Now parse the query above. Output only JSON:"#,
        query
    )
}

fn parse_query_response(response: &str, original_query: &str) -> Result<ParsedQuery> {
    // Extract JSON from response (handle markdown code blocks and extra text)
    let json_str = if let Some(start) = response.find('{') {
        if let Some(end) = response.rfind('}') {
            &response[start..=end]
        } else {
            response
        }
    } else {
        // No JSON found, use fallback
        return Ok(ParsedQuery {
            search_terms: original_query.to_string(),
            temporal_filter: None,
            metadata_filters: vec![],
            search_type: SearchType::Hybrid,
            confidence: 0.5,
        });
    };

    // Parse JSON
    let parsed_json: serde_json::Value = serde_json::from_str(json_str).map_err(|e| {
        tracing::warn!("Failed to parse query JSON: {}, using fallback", e);
        AgentRootError::Llm(format!("JSON parse error: {}", e))
    })?;

    let search_terms = parsed_json["search_terms"]
        .as_str()
        .unwrap_or(original_query)
        .to_string();

    let temporal_filter = if let Some(tf) = parsed_json.get("temporal_filter") {
        if !tf.is_null() {
            let hours = tf["relative_hours"].as_i64().unwrap_or(24);
            let description = tf["description"].as_str().unwrap_or("").to_string();
            let now = Utc::now();
            let start = now - Duration::hours(hours);
            Some(TemporalFilter {
                start: Some(start.to_rfc3339()),
                end: Some(now.to_rfc3339()),
                description,
            })
        } else {
            None
        }
    } else {
        None
    };

    let metadata_filters = if let Some(filters) = parsed_json["metadata_filters"].as_array() {
        filters
            .iter()
            .filter_map(|f| {
                Some(super::MetadataFilterHint {
                    field: f["field"].as_str()?.to_string(),
                    value: f["value"].as_str()?.to_string(),
                    operator: f["operator"].as_str().unwrap_or("contains").to_string(),
                })
            })
            .collect()
    } else {
        vec![]
    };

    let search_type = match parsed_json["search_type"].as_str() {
        Some("bm25") => SearchType::Bm25,
        Some("vector") => SearchType::Vector,
        Some("hybrid") => SearchType::Hybrid,
        _ => SearchType::Hybrid,
    };

    let confidence = parsed_json["confidence"].as_f64().unwrap_or(0.8);

    Ok(ParsedQuery {
        search_terms,
        temporal_filter,
        metadata_filters,
        search_type,
        confidence,
    })
}
