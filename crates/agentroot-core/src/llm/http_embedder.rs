//! HTTP-based embedder using external LLM service

use super::{Embedder, LLMClient};
use crate::config::LLMServiceConfig;
use crate::error::Result;
use async_trait::async_trait;
use std::sync::Arc;

/// Embedder that uses external HTTP service (vLLM, OpenAI, etc.)
pub struct HttpEmbedder {
    client: Arc<dyn LLMClient>,
}

impl HttpEmbedder {
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
impl Embedder for HttpEmbedder {
    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        self.client.embed(text).await
    }

    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        self.client.embed_batch(texts).await
    }

    fn dimensions(&self) -> usize {
        self.client.embedding_dimensions()
    }

    fn model_name(&self) -> &str {
        self.client.model_name()
    }
}
