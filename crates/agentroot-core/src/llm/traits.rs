//! LLM trait definitions

use async_trait::async_trait;
use crate::error::Result;

/// Embedding generation trait
#[async_trait]
pub trait Embedder: Send + Sync {
    /// Generate embedding for single text
    async fn embed(&self, text: &str) -> Result<Vec<f32>>;

    /// Generate embeddings for batch of texts
    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>>;

    /// Get embedding dimensions
    fn dimensions(&self) -> usize;

    /// Get model name
    fn model_name(&self) -> &str;
}

/// Document reranking trait
#[async_trait]
pub trait Reranker: Send + Sync {
    /// Rerank documents for a query
    async fn rerank(&self, query: &str, documents: &[RerankDocument]) -> Result<Vec<RerankResult>>;

    /// Get model name
    fn model_name(&self) -> &str;
}

/// Document for reranking
#[derive(Debug, Clone)]
pub struct RerankDocument {
    pub id: String,
    pub text: String,
}

/// Reranking result
#[derive(Debug, Clone)]
pub struct RerankResult {
    pub id: String,
    pub score: f64,
}

/// Query expansion trait
#[async_trait]
pub trait QueryExpander: Send + Sync {
    /// Expand query into variants
    async fn expand(&self, query: &str, context: Option<&str>) -> Result<ExpandedQuery>;

    /// Get model name
    fn model_name(&self) -> &str;
}

/// Expanded query variants
#[derive(Debug, Clone, Default)]
pub struct ExpandedQuery {
    /// Lexical variations for BM25
    pub lexical: Vec<String>,
    /// Semantic variations for vector search
    pub semantic: Vec<String>,
    /// Hypothetical document (HyDE)
    pub hyde: Option<String>,
}

/// Tokenization trait
#[async_trait]
pub trait Tokenizer: Send + Sync {
    /// Tokenize text
    async fn tokenize(&self, text: &str) -> Result<Vec<u32>>;

    /// Detokenize tokens back to text
    async fn detokenize(&self, tokens: &[u32]) -> Result<String>;

    /// Count tokens
    async fn count_tokens(&self, text: &str) -> Result<usize>;
}
