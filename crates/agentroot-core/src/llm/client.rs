//! HTTP client for external LLM services (vLLM, OpenAI, etc.)

use crate::config::LLMServiceConfig;
use crate::error::{AgentRootError, Result};
use crate::llm::{DocumentMetadata, MetadataContext};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Trait for LLM service clients
#[async_trait]
pub trait LLMClient: Send + Sync {
    /// Generate chat completion
    async fn chat_completion(&self, messages: Vec<ChatMessage>) -> Result<String>;

    /// Generate embeddings for text
    async fn embed(&self, text: &str) -> Result<Vec<f32>>;

    /// Generate embeddings for multiple texts
    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>>;

    /// Get embedding dimensions
    fn embedding_dimensions(&self) -> usize;

    /// Get model name
    fn model_name(&self) -> &str;
}

/// Chat message for completion requests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

impl ChatMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: "system".to_string(),
            content: content.into(),
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".to_string(),
            content: content.into(),
        }
    }
}

/// vLLM/OpenAI-compatible client
pub struct VLLMClient {
    http_client: reqwest::Client,
    config: LLMServiceConfig,
    embedding_dimensions: usize,
}

impl VLLMClient {
    /// Create new vLLM client from configuration
    pub fn new(config: LLMServiceConfig) -> Result<Self> {
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .build()
            .map_err(|e| AgentRootError::Http(e))?;

        // Use configured dimensions or default to 384
        let embedding_dimensions = config.embedding_dimensions.unwrap_or(384);

        Ok(Self {
            http_client,
            config,
            embedding_dimensions,
        })
    }

    /// Create from environment variables
    pub fn from_env() -> Result<Self> {
        let config = LLMServiceConfig::default();
        Self::new(config)
    }
}

#[async_trait]
impl LLMClient for VLLMClient {
    async fn chat_completion(&self, messages: Vec<ChatMessage>) -> Result<String> {
        #[derive(Serialize)]
        struct ChatRequest {
            model: String,
            messages: Vec<ChatMessage>,
            temperature: f32,
            max_tokens: u32,
        }

        #[derive(Deserialize)]
        struct ChatResponse {
            choices: Vec<ChatChoice>,
        }

        #[derive(Deserialize)]
        struct ChatChoice {
            message: ChatMessage,
        }

        let request = ChatRequest {
            model: self.config.model.clone(),
            messages,
            temperature: 0.7,
            max_tokens: 512,
        };

        let url = format!("{}/v1/chat/completions", self.config.url);

        let mut req = self.http_client.post(&url).json(&request);

        if let Some(ref api_key) = self.config.api_key {
            req = req.header("Authorization", format!("Bearer {}", api_key));
        }

        let response = req.send().await.map_err(|e| AgentRootError::Http(e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AgentRootError::ExternalError(format!(
                "LLM service error (HTTP {}): {}",
                status, body
            )));
        }

        let chat_response: ChatResponse =
            response.json().await.map_err(|e| AgentRootError::Http(e))?;

        let content = chat_response
            .choices
            .first()
            .ok_or_else(|| AgentRootError::Llm("No response from LLM".to_string()))?
            .message
            .content
            .clone();

        Ok(content)
    }

    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let results = self.embed_batch(&[text.to_string()]).await?;
        results
            .into_iter()
            .next()
            .ok_or_else(|| AgentRootError::Llm("No embedding returned".to_string()))
    }

    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        #[derive(Serialize)]
        struct EmbedRequest {
            model: String,
            input: Vec<String>,
        }

        #[derive(Deserialize)]
        struct EmbedResponse {
            data: Vec<EmbedData>,
        }

        #[derive(Deserialize)]
        struct EmbedData {
            embedding: Vec<f32>,
        }

        let request = EmbedRequest {
            model: self.config.embedding_model.clone(),
            input: texts.to_vec(),
        };

        let url = format!("{}/v1/embeddings", self.config.embeddings_url());

        let mut req = self.http_client.post(&url).json(&request);

        if let Some(ref api_key) = self.config.api_key {
            req = req.header("Authorization", format!("Bearer {}", api_key));
        }

        let response = req.send().await.map_err(|e| AgentRootError::Http(e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AgentRootError::ExternalError(format!(
                "Embedding service error (HTTP {}): {}",
                status, body
            )));
        }

        let embed_response: EmbedResponse =
            response.json().await.map_err(|e| AgentRootError::Http(e))?;

        Ok(embed_response
            .data
            .into_iter()
            .map(|d| d.embedding)
            .collect())
    }

    fn embedding_dimensions(&self) -> usize {
        self.embedding_dimensions
    }

    fn model_name(&self) -> &str {
        &self.config.model
    }
}

/// Helper to generate metadata using LLM client
pub async fn generate_metadata_with_llm(
    client: &dyn LLMClient,
    content: &str,
    context: &MetadataContext,
) -> Result<DocumentMetadata> {
    let prompt = build_metadata_prompt(content, context);

    let messages = vec![
        ChatMessage::system(
            "You are a metadata generator. Extract structured metadata from documents. \
             Respond ONLY with valid JSON matching the schema.",
        ),
        ChatMessage::user(prompt),
    ];

    let response = client.chat_completion(messages).await?;

    // Parse JSON response
    parse_metadata_response(&response)
}

fn build_metadata_prompt(content: &str, context: &MetadataContext) -> String {
    // Truncate content if too long (max ~2000 tokens ~8000 chars)
    let truncated = if content.len() > 8000 {
        &content[..8000]
    } else {
        content
    };

    format!(
        r#"Generate metadata for this document:

Source type: {}
Language: {}
Collection: {}

Content:
{}

Output JSON with these fields:
- summary: 100-200 word summary
- semantic_title: improved title
- keywords: 5-10 keywords (array)
- category: document type
- intent: purpose description
- concepts: related concepts (array)
- difficulty: beginner/intermediate/advanced
- suggested_queries: search queries (array)

JSON:"#,
        context.source_type,
        context.language.as_deref().unwrap_or("unknown"),
        context.collection_name,
        truncated
    )
}

fn parse_metadata_response(response: &str) -> Result<DocumentMetadata> {
    // Extract JSON from response (handle markdown code blocks)
    let json_str = if let Some(start) = response.find('{') {
        if let Some(end) = response.rfind('}') {
            &response[start..=end]
        } else {
            response
        }
    } else {
        response
    };

    serde_json::from_str(json_str)
        .map_err(|e| AgentRootError::Llm(format!("Failed to parse metadata JSON: {}", e)))
}
