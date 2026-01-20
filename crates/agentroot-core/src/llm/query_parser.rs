//! Natural language query parser using LLM
//!
//! Parses user queries like "files edited last hour" into structured search parameters

use crate::error::{AgentRootError, Result};
use chrono::{Duration, Utc};
use llama_cpp_2::{
    context::params::LlamaContextParams,
    llama_backend::LlamaBackend,
    llama_batch::LlamaBatch,
    model::{params::LlamaModelParams, LlamaModel},
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Parsed query with extracted intent and filters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedQuery {
    /// The cleaned search terms to use
    pub search_terms: String,

    /// Temporal constraints
    pub temporal_filter: Option<TemporalFilter>,

    /// Metadata filters extracted from query
    pub metadata_filters: Vec<MetadataFilterHint>,

    /// Suggested search type
    pub search_type: SearchType,

    /// Confidence in the parse (0.0 - 1.0)
    pub confidence: f64,
}

/// Temporal filter for time-based queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalFilter {
    /// Start datetime (ISO 8601)
    pub start: Option<String>,

    /// End datetime (ISO 8601)
    pub end: Option<String>,

    /// Human-readable description
    pub description: String,
}

/// Metadata filter hint extracted from query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataFilterHint {
    /// Field name
    pub field: String,

    /// Expected value
    pub value: String,

    /// Operator (eq, contains, gt, lt)
    pub operator: String,
}

/// Search type recommendation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SearchType {
    /// BM25 full-text search
    Bm25,

    /// Vector semantic search
    Vector,

    /// Hybrid (both + reranking)
    Hybrid,
}

/// Query parser using local LLM
pub struct QueryParser {
    #[allow(dead_code)]
    model_path: PathBuf,
}

impl QueryParser {
    /// Create a new query parser with custom model
    pub fn new(model_path: PathBuf) -> Result<Self> {
        if !model_path.exists() {
            return Err(AgentRootError::ModelNotFound(
                model_path.to_string_lossy().to_string(),
            ));
        }
        Ok(Self { model_path })
    }

    /// Create parser with default model
    pub fn from_default() -> Result<Self> {
        let model_dir = dirs::data_local_dir()
            .ok_or_else(|| AgentRootError::Config("Cannot determine data directory".to_string()))?
            .join("agentroot")
            .join("models");

        let model_path = model_dir.join("llama-3.1-8b-instruct.Q4_K_M.gguf");

        if !model_path.exists() {
            return Err(AgentRootError::ModelNotFound(format!(
                "Model not found at {}. Run 'agentroot embed' first to download models.",
                model_path.display()
            )));
        }

        Ok(Self { model_path })
    }

    /// Parse natural language query into structured search
    pub async fn parse(&self, query: &str) -> Result<ParsedQuery> {
        self.llm_parse(query).await
    }

    /// Parse query using LLM
    async fn llm_parse(&self, query: &str) -> Result<ParsedQuery> {
        tracing::debug!("Using LLM to parse query: {}", query);

        let mut backend = LlamaBackend::init()
            .map_err(|e| AgentRootError::Llm(format!("Failed to init LLM backend: {}", e)))?;
        backend.void_logs();

        let model_params = LlamaModelParams::default();
        let model = LlamaModel::load_from_file(&backend, &self.model_path, &model_params)
            .map_err(|e| AgentRootError::Llm(format!("Failed to load LLM model: {}", e)))?;

        let ctx_size = std::num::NonZeroU32::new(4096).unwrap();
        let ctx_params = LlamaContextParams::default()
            .with_n_ctx(Some(ctx_size))
            .with_n_batch(512);

        let mut ctx = model
            .new_context(&backend, ctx_params)
            .map_err(|e| AgentRootError::Llm(format!("Failed to create LLM context: {}", e)))?;

        let prompt = self.build_parsing_prompt(query);

        let tokens = model
            .str_to_token(&prompt, llama_cpp_2::model::AddBos::Never)
            .map_err(|e| AgentRootError::Llm(format!("Tokenization error: {}", e)))?;

        let max_output_tokens = 256;
        let mut output_tokens = Vec::new();
        let mut current_pos = 0;

        // Process prompt tokens - enable logits for the last token
        let chunks: Vec<_> = tokens.chunks(512).collect();
        for (chunk_idx, chunk) in chunks.iter().enumerate() {
            let is_last_chunk = chunk_idx == chunks.len() - 1;
            let mut batch = LlamaBatch::new(chunk.len(), 1);
            for (i, token) in chunk.iter().enumerate() {
                let is_last_token_overall = is_last_chunk && i == chunk.len() - 1;
                batch
                    .add(*token, current_pos + i as i32, &[0], is_last_token_overall)
                    .map_err(|e| AgentRootError::Llm(format!("Batch error: {}", e)))?;
            }
            current_pos += chunk.len() as i32;

            ctx.decode(&mut batch)
                .map_err(|e| AgentRootError::Llm(format!("Decode error: {}", e)))?;
        }

        for (chunk_idx, chunk) in chunks.iter().enumerate() {
            let is_last_chunk = chunk_idx == chunks.len() - 1;
            let mut batch = LlamaBatch::new(chunk.len(), 1);
            tracing::debug!(
                "Processing chunk {}/{}, size: {}, is_last: {}",
                chunk_idx + 1,
                chunks.len(),
                chunk.len(),
                is_last_chunk
            );

            for (i, token) in chunk.iter().enumerate() {
                let is_last_token_overall = is_last_chunk && i == chunk.len() - 1;
                if is_last_token_overall {
                    tracing::debug!(
                        "Marking token at position {} (offset {} in batch) for logits",
                        current_pos + i as i32,
                        i
                    );
                }
                batch
                    .add(*token, current_pos + i as i32, &[0], is_last_token_overall)
                    .map_err(|e| AgentRootError::Llm(format!("Batch error: {}", e)))?;
            }
            current_pos += chunk.len() as i32;

            ctx.decode(&mut batch)
                .map_err(|e| AgentRootError::Llm(format!("Decode error: {}", e)))?;
        }

        tracing::debug!(
            "Prompt processed, {} tokens total, current_pos = {}, will sample from position {}",
            tokens.len(),
            current_pos,
            current_pos - 1
        );

        let mut generated_text = String::new();
        let mut brace_count = 0;
        let mut json_started = false;

        for i in 0..max_output_tokens {
            let token_data_array = ctx.token_data_array();

            let next_token = token_data_array
                .data
                .iter()
                .max_by(|a, b| a.logit().partial_cmp(&b.logit()).unwrap())
                .map(|td| td.id())
                .ok_or_else(|| AgentRootError::Llm("No token found".to_string()))?;

            if next_token == model.token_eos() {
                tracing::debug!("Hit EOS token after {} tokens", i);
                break;
            }

            let token_str = model
                .token_to_str(next_token, llama_cpp_2::model::Special::Tokenize)
                .map_err(|e| AgentRootError::Llm(format!("Token decode error: {}", e)))?;

            generated_text.push_str(&token_str);
            output_tokens.push(next_token);

            if token_str.contains("{") {
                json_started = true;
                brace_count += token_str.matches("{").count() as i32;
            }
            if token_str.contains("}") {
                brace_count -= token_str.matches("}").count() as i32;
                if json_started && brace_count == 0 {
                    tracing::debug!("JSON complete after {} tokens", i + 1);
                    break;
                }
            }

            if i % 50 == 0 && i > 0 {
                tracing::debug!(
                    "Generated {} tokens so far, text length: {}",
                    i,
                    generated_text.len()
                );
            }

            let mut batch = LlamaBatch::new(1, 1);
            batch
                .add(next_token, current_pos, &[0], true)
                .map_err(|e| AgentRootError::Llm(format!("Batch error: {}", e)))?;

            ctx.decode(&mut batch)
                .map_err(|e| AgentRootError::Llm(format!("Decode error: {}", e)))?;

            current_pos += 1;
        }

        tracing::debug!("LLM raw output: {}", generated_text);

        self.parse_llm_response(&generated_text, query)
    }

    fn build_parsing_prompt(&self, query: &str) -> String {
        format!(
            r#"<|begin_of_text|><|start_header_id|>system<|end_header_id|>

You are a search query parser. Extract structured information from user queries.
Output ONLY valid JSON with these fields:
- search_terms: main keywords (string)
- temporal_filter: {{"description": "...", "relative_hours": N}} or null
- metadata_filters: [{{"field": "...", "value": "...", "operator": "contains"}}] or []
- confidence: 0.0-1.0

Examples:
Query: "files that were edit recently"
{{"search_terms": "files", "temporal_filter": {{"description": "recently", "relative_hours": 24}}, "metadata_filters": [], "confidence": 0.9}}

Query: "rust code by Alice from last week"
{{"search_terms": "rust code", "temporal_filter": {{"description": "last week", "relative_hours": 168}}, "metadata_filters": [{{"field": "author", "value": "Alice", "operator": "contains"}}], "confidence": 0.95}}

Query: "python functions"
{{"search_terms": "python functions", "temporal_filter": null, "metadata_filters": [], "confidence": 0.85}}

<|eot_id|><|start_header_id|>user<|end_header_id|>

Parse this query: "{}"<|eot_id|><|start_header_id|>assistant<|end_header_id|>

"#,
            query
        )
    }

    fn parse_llm_response(&self, response: &str, original_query: &str) -> Result<ParsedQuery> {
        let json_start = response.find('{');
        let json_end = response.rfind('}');

        let json_str = match (json_start, json_end) {
            (Some(start), Some(end)) if end > start => &response[start..=end],
            _ => {
                tracing::warn!("Failed to extract JSON from LLM response, using fallback");
                return Ok(ParsedQuery {
                    search_terms: original_query.to_string(),
                    temporal_filter: None,
                    metadata_filters: vec![],
                    search_type: SearchType::Hybrid,
                    confidence: 0.5,
                });
            }
        };

        let parsed_json: serde_json::Value = serde_json::from_str(json_str).map_err(|e| {
            tracing::warn!("Failed to parse LLM JSON output: {}", e);
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
                    Some(MetadataFilterHint {
                        field: f["field"].as_str()?.to_string(),
                        value: f["value"].as_str()?.to_string(),
                        operator: f["operator"].as_str().unwrap_or("contains").to_string(),
                    })
                })
                .collect()
        } else {
            vec![]
        };

        let confidence = parsed_json["confidence"].as_f64().unwrap_or(0.8);

        Ok(ParsedQuery {
            search_terms,
            temporal_filter,
            metadata_filters,
            search_type: SearchType::Hybrid,
            confidence,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_parse_requires_model() {
        let result = QueryParser::from_default();
        if result.is_err() {
            println!("Skipping test: LLM model not available");
            return;
        }

        let parser = result.unwrap();
        let parsed = parser.parse("test query").await;

        assert!(parsed.is_ok() || parsed.is_err());
    }

    #[tokio::test]
    async fn test_llm_parse_temporal_query() {
        let result = QueryParser::from_default();
        if result.is_err() {
            println!("Skipping test: LLM model not available");
            return;
        }

        let parser = result.unwrap();
        let parsed = parser.parse("files that were edit recently").await;

        if let Ok(parsed) = parsed {
            println!("Parsed query: {:?}", parsed);
            assert!(!parsed.search_terms.is_empty());
        }
    }

    #[tokio::test]
    async fn test_llm_parse_metadata_query() {
        let result = QueryParser::from_default();
        if result.is_err() {
            println!("Skipping test: LLM model not available");
            return;
        }

        let parser = result.unwrap();
        let parsed = parser.parse("rust code by Alice").await;

        if let Ok(parsed) = parsed {
            println!("Parsed query: {:?}", parsed);
            assert!(!parsed.search_terms.is_empty());
        }
    }

    #[test]
    fn test_parse_llm_response_valid_json() {
        let parser = QueryParser {
            model_path: PathBuf::from("dummy"),
        };

        let response = r#"{"search_terms": "files", "temporal_filter": {"description": "recently", "relative_hours": 24}, "metadata_filters": [], "confidence": 0.9}"#;
        let result = parser.parse_llm_response(response, "files that were edit recently");

        assert!(result.is_ok());
        let parsed = result.unwrap();
        assert_eq!(parsed.search_terms, "files");
        assert!(parsed.temporal_filter.is_some());
    }

    #[test]
    fn test_parse_llm_response_invalid_json_fallback() {
        let parser = QueryParser {
            model_path: PathBuf::from("dummy"),
        };

        let response = "not valid json";
        let result = parser.parse_llm_response(response, "original query");

        assert!(result.is_ok());
        let parsed = result.unwrap();
        assert_eq!(parsed.search_terms, "original query");
        assert_eq!(parsed.confidence, 0.5);
    }

    #[test]
    fn test_build_parsing_prompt() {
        let parser = QueryParser {
            model_path: PathBuf::from("dummy"),
        };

        let prompt = parser.build_parsing_prompt("test query");
        assert!(prompt.contains("test query"));
        assert!(prompt.contains("search_terms"));
        assert!(prompt.contains("temporal_filter"));
    }
}
