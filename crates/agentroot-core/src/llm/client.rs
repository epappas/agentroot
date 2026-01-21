//! HTTP client for external LLM services (vLLM, OpenAI, etc.)

use crate::config::LLMServiceConfig;
use crate::error::{AgentRootError, Result};
use crate::llm::{DocumentMetadata, MetadataContext};
use async_trait::async_trait;
use futures::stream;
use serde::{Deserialize, Serialize};
use std::sync::{atomic::AtomicU64, Arc};
use std::time::{Duration, Instant};

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

/// API metrics for monitoring
#[derive(Debug, Default)]
pub struct APIMetrics {
    pub total_requests: AtomicU64,
    pub total_errors: AtomicU64,
    pub cache_hits: AtomicU64,
    pub cache_misses: AtomicU64,
    pub total_latency_ms: AtomicU64,
}

/// vLLM/OpenAI-compatible client
pub struct VLLMClient {
    http_client: reqwest::Client,
    config: LLMServiceConfig,
    embedding_dimensions: usize,
    cache: Arc<super::cache::LLMCache>,
    metrics: Arc<APIMetrics>,
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

        // Enable caching by default (1 hour TTL)
        let cache = Arc::new(super::cache::LLMCache::new());

        // Initialize metrics
        let metrics = Arc::new(APIMetrics::default());

        Ok(Self {
            http_client,
            config,
            embedding_dimensions,
            cache,
            metrics,
        })
    }

    /// Create from environment variables
    pub fn from_env() -> Result<Self> {
        let config = LLMServiceConfig::default();
        Self::new(config)
    }

    /// Get current API metrics
    pub fn metrics(&self) -> MetricsSnapshot {
        use std::sync::atomic::Ordering;

        let total = self.metrics.total_requests.load(Ordering::Relaxed);
        let hits = self.metrics.cache_hits.load(Ordering::Relaxed);
        let misses = self.metrics.cache_misses.load(Ordering::Relaxed);

        MetricsSnapshot {
            total_requests: total,
            total_errors: self.metrics.total_errors.load(Ordering::Relaxed),
            cache_hits: hits,
            cache_misses: misses,
            cache_hit_rate: if total > 0 {
                hits as f64 / total as f64 * 100.0
            } else {
                0.0
            },
            avg_latency_ms: if total > 0 {
                self.metrics.total_latency_ms.load(Ordering::Relaxed) as f64 / total as f64
            } else {
                0.0
            },
        }
    }

    /// Embed texts with optimized batching
    ///
    /// Splits large batches into optimal chunks for better throughput
    /// and parallel processing. Returns progress updates via callback.
    pub async fn embed_batch_optimized<F>(
        &self,
        texts: &[String],
        batch_size: usize,
        progress_callback: Option<F>,
    ) -> Result<Vec<Vec<f32>>>
    where
        F: Fn(usize, usize) + Send + Sync,
    {
        const DEFAULT_BATCH_SIZE: usize = 32;
        let chunk_size = if batch_size > 0 {
            batch_size
        } else {
            DEFAULT_BATCH_SIZE
        };

        let total = texts.len();
        let mut all_results = Vec::with_capacity(total);

        for (chunk_idx, chunk) in texts.chunks(chunk_size).enumerate() {
            let chunk_results = self.embed_batch(chunk).await?;
            all_results.extend(chunk_results);

            if let Some(ref callback) = progress_callback {
                callback((chunk_idx + 1) * chunk_size.min(total), total);
            }
        }

        Ok(all_results)
    }

    /// Embed texts in parallel with multiple concurrent batches
    ///
    /// Uses tokio to process multiple batches concurrently for maximum throughput.
    /// Useful for embedding large document collections.
    pub async fn embed_batch_parallel(
        &self,
        texts: &[String],
        batch_size: usize,
        max_concurrent: usize,
    ) -> Result<Vec<Vec<f32>>> {
        use futures::stream::StreamExt;

        const DEFAULT_BATCH_SIZE: usize = 32;
        const DEFAULT_CONCURRENT: usize = 4;

        let chunk_size = if batch_size > 0 {
            batch_size
        } else {
            DEFAULT_BATCH_SIZE
        };
        let concurrent = if max_concurrent > 0 {
            max_concurrent
        } else {
            DEFAULT_CONCURRENT
        };

        let chunks: Vec<_> = texts.chunks(chunk_size).collect();
        let total_chunks = chunks.len();

        tracing::info!(
            "Embedding {} texts in {} batches ({} concurrent)",
            texts.len(),
            total_chunks,
            concurrent
        );

        let results: Vec<_> = stream::iter(chunks)
            .enumerate()
            .map(|(idx, chunk)| async move {
                tracing::debug!("Processing batch {}/{}", idx + 1, total_chunks);
                let result = self.embed_batch(chunk).await;
                (idx, result)
            })
            .buffer_unordered(concurrent)
            .collect()
            .await;

        // Sort results by original order
        let mut sorted_results: Vec<_> = results;
        sorted_results.sort_by_key(|(idx, _)| *idx);

        // Flatten results
        let mut all_embeddings = Vec::with_capacity(texts.len());
        for (_, result) in sorted_results {
            all_embeddings.extend(result?);
        }

        Ok(all_embeddings)
    }
}

/// Snapshot of API metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    pub total_requests: u64,
    pub total_errors: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub cache_hit_rate: f64,
    pub avg_latency_ms: f64,
}

#[async_trait]
impl LLMClient for VLLMClient {
    async fn chat_completion(&self, messages: Vec<ChatMessage>) -> Result<String> {
        use std::sync::atomic::Ordering;

        let start = Instant::now();
        self.metrics.total_requests.fetch_add(1, Ordering::Relaxed);

        // Check cache first
        let messages_json = serde_json::to_string(&messages).unwrap_or_default();
        let cache_key = super::cache::chat_cache_key(&self.config.model, &messages_json);

        if let Some(cached) = self.cache.get(&cache_key) {
            tracing::debug!("Cache hit for chat completion");
            self.metrics.cache_hits.fetch_add(1, Ordering::Relaxed);
            return Ok(cached);
        }

        self.metrics.cache_misses.fetch_add(1, Ordering::Relaxed);

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

        let response = req.send().await.map_err(|e| {
            self.metrics.total_errors.fetch_add(1, Ordering::Relaxed);
            AgentRootError::Http(e)
        })?;

        if !response.status().is_success() {
            self.metrics.total_errors.fetch_add(1, Ordering::Relaxed);
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AgentRootError::ExternalError(format!(
                "LLM service error (HTTP {}): {}",
                status, body
            )));
        }

        let chat_response: ChatResponse = response.json().await.map_err(|e| {
            self.metrics.total_errors.fetch_add(1, Ordering::Relaxed);
            AgentRootError::Http(e)
        })?;

        let content = chat_response
            .choices
            .first()
            .ok_or_else(|| {
                self.metrics.total_errors.fetch_add(1, Ordering::Relaxed);
                AgentRootError::Llm("No response from LLM".to_string())
            })?
            .message
            .content
            .clone();

        // Cache the response
        let _ = self.cache.set(cache_key, content.clone());

        // Track latency
        let elapsed = start.elapsed().as_millis() as u64;
        self.metrics
            .total_latency_ms
            .fetch_add(elapsed, Ordering::Relaxed);

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
        use std::sync::atomic::Ordering;

        let start = Instant::now();
        self.metrics.total_requests.fetch_add(1, Ordering::Relaxed);

        // Check cache for each text
        let mut results = Vec::with_capacity(texts.len());
        let mut uncached_texts = Vec::new();
        let mut uncached_indices = Vec::new();

        for (i, text) in texts.iter().enumerate() {
            let cache_key = super::cache::embedding_cache_key(&self.config.embedding_model, text);
            if let Some(cached) = self.cache.get(&cache_key) {
                // Parse cached embedding
                if let Ok(embedding) = serde_json::from_str::<Vec<f32>>(&cached) {
                    results.push(Some(embedding));
                    self.metrics.cache_hits.fetch_add(1, Ordering::Relaxed);
                    continue;
                }
            }
            self.metrics.cache_misses.fetch_add(1, Ordering::Relaxed);
            results.push(None);
            uncached_texts.push(text.clone());
            uncached_indices.push(i);
        }

        // If all cached, return early
        if uncached_texts.is_empty() {
            tracing::debug!("All {} embeddings from cache", texts.len());
            return Ok(results.into_iter().map(|r| r.unwrap()).collect());
        }

        tracing::debug!(
            "Embedding batch: {} cached, {} to fetch",
            texts.len() - uncached_texts.len(),
            uncached_texts.len()
        );

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
            input: uncached_texts.clone(),
        };

        let url = format!("{}/v1/embeddings", self.config.embeddings_url());

        let mut req = self.http_client.post(&url).json(&request);

        if let Some(ref api_key) = self.config.api_key {
            req = req.header("Authorization", format!("Bearer {}", api_key));
        }

        let response = req.send().await.map_err(|e| {
            self.metrics.total_errors.fetch_add(1, Ordering::Relaxed);
            AgentRootError::Http(e)
        })?;

        if !response.status().is_success() {
            self.metrics.total_errors.fetch_add(1, Ordering::Relaxed);
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AgentRootError::ExternalError(format!(
                "Embedding service error (HTTP {}): {}",
                status, body
            )));
        }

        let embed_response: EmbedResponse = response.json().await.map_err(|e| {
            self.metrics.total_errors.fetch_add(1, Ordering::Relaxed);
            AgentRootError::Http(e)
        })?;

        // Fill in uncached results and cache them
        for (i, embedding) in embed_response.data.into_iter().enumerate() {
            let original_idx = uncached_indices[i];
            results[original_idx] = Some(embedding.embedding.clone());

            // Cache the embedding
            let cache_key =
                super::cache::embedding_cache_key(&self.config.embedding_model, &uncached_texts[i]);
            if let Ok(json) = serde_json::to_string(&embedding.embedding) {
                let _ = self.cache.set(cache_key, json);
            }
        }

        // Track latency
        let elapsed = start.elapsed().as_millis() as u64;
        self.metrics
            .total_latency_ms
            .fetch_add(elapsed, Ordering::Relaxed);

        Ok(results.into_iter().map(|r| r.unwrap()).collect())
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
