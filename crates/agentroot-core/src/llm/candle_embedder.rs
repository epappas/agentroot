//! Candle-based embedder (pure Rust, no segfaults!)

use super::Embedder;
use crate::error::{AgentRootError, Result};
use async_trait::async_trait;
use candle_core::{Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::bert::{BertModel, Config, DTYPE};
use std::path::Path;
use tokenizers::Tokenizer;

/// Default embedding model (BERT-based)
pub const DEFAULT_CANDLE_MODEL: &str = "sentence-transformers/all-MiniLM-L6-v2";

/// Model dimensions for known models
const MINILM_L6_DIM: usize = 384;

/// Candle-based embedder using sentence-transformers models
pub struct CandleEmbedder {
    model: BertModel,
    tokenizer: Tokenizer,
    device: Device,
    model_name: String,
    dimensions: usize,
}

impl CandleEmbedder {
    /// Create a new CandleEmbedder from a local model directory
    pub fn new(model_path: impl AsRef<Path>) -> Result<Self> {
        let model_path = model_path.as_ref();

        if !model_path.exists() {
            return Err(AgentRootError::ModelNotFound(format!(
                "Model directory not found: {}",
                model_path.display()
            )));
        }

        // Load tokenizer
        let tokenizer_path = model_path.join("tokenizer.json");
        if !tokenizer_path.exists() {
            return Err(AgentRootError::ModelNotFound(format!(
                "tokenizer.json not found in {}",
                model_path.display()
            )));
        }
        let tokenizer = Tokenizer::from_file(&tokenizer_path)
            .map_err(|e| AgentRootError::Llm(format!("Failed to load tokenizer: {}", e)))?;

        // Load config
        let config_path = model_path.join("config.json");
        if !config_path.exists() {
            return Err(AgentRootError::ModelNotFound(format!(
                "config.json not found in {}",
                model_path.display()
            )));
        }
        let config_str = std::fs::read_to_string(&config_path)?;
        let config: Config = serde_json::from_str(&config_str)
            .map_err(|e| AgentRootError::Llm(format!("Failed to parse config: {}", e)))?;

        // Use known dimensions for MiniLM-L6-v2
        let dimensions = MINILM_L6_DIM;

        // Use CPU
        let device = Device::Cpu;

        // Load model weights
        let weights_path = model_path.join("model.safetensors");
        if !weights_path.exists() {
            return Err(AgentRootError::ModelNotFound(format!(
                "model.safetensors not found in {}",
                model_path.display()
            )));
        }

        let vb = unsafe { VarBuilder::from_mmaped_safetensors(&[weights_path], DTYPE, &device)? };

        let model = BertModel::load(vb, &config)
            .map_err(|e| AgentRootError::Llm(format!("Failed to load BERT model: {}", e)))?;

        let model_name = model_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        tracing::info!(
            "Model loaded from {}: {} ({} dims)",
            model_path.display(),
            model_name,
            dimensions
        );

        Ok(Self {
            model,
            tokenizer,
            device,
            model_name,
            dimensions,
        })
    }

    /// Create from Hugging Face model name (downloads if needed)
    pub fn from_hf(model_name: &str) -> Result<Self> {
        tracing::info!("Loading model from Hugging Face: {}", model_name);

        // Check if model exists in HF cache first
        let cache_dir = dirs::cache_dir()
            .ok_or_else(|| AgentRootError::Config("Cannot determine cache directory".to_string()))?
            .join("huggingface")
            .join("hub");

        // Convert model name to cache path (e.g., "sentence-transformers/all-MiniLM-L6-v2" -> "models--sentence-transformers--all-MiniLM-L6-v2")
        let cache_model_name = format!("models--{}", model_name.replace('/', "--"));
        let model_cache_path = cache_dir.join(&cache_model_name);

        // Check if snapshots directory exists (indicates model was previously downloaded)
        let snapshots_dir = model_cache_path.join("snapshots");
        if snapshots_dir.exists() {
            // Find the first snapshot directory
            if let Ok(mut entries) = std::fs::read_dir(&snapshots_dir) {
                if let Some(Ok(entry)) = entries.next() {
                    let snapshot_path = entry.path();
                    tracing::info!("Found cached model at: {}", snapshot_path.display());
                    return Self::new(&snapshot_path);
                }
            }
        }

        // Model not in cache, download using our HTTP downloader
        tracing::info!("Model not found in cache, downloading from Hugging Face...");

        let snapshot_path = super::download::download_sentence_transformer(model_name)?;

        tracing::info!("Model downloaded successfully, loading...");

        Self::new(&snapshot_path)
    }

    /// Create from default model location  
    pub fn from_default() -> Result<Self> {
        // Try downloading from HF, but provide helpful error message if it fails
        Self::from_hf(DEFAULT_CANDLE_MODEL).map_err(|e| {
            AgentRootError::ModelNotFound(format!(
                "Failed to load embedding model: {}\n\n\
                 To use embeddings, the model will be automatically downloaded from Hugging Face.\n\
                 If download fails, you can manually download the model to:\n\
                 ~/.cache/huggingface/hub/models--sentence-transformers--all-MiniLM-L6-v2/\n\n\
                 Or use a different model with: agentroot embed --model <model-name>",
                e
            ))
        })
    }

    fn embed_sync(&self, text: &str) -> Result<Vec<f32>> {
        // Tokenize
        let encoding = self
            .tokenizer
            .encode(text, true)
            .map_err(|e| AgentRootError::Llm(format!("Tokenization error: {}", e)))?;

        let tokens = encoding.get_ids();
        let token_ids = Tensor::new(tokens, &self.device)?;

        // Add batch dimension
        let token_ids = token_ids.unsqueeze(0)?;
        let token_type_ids = token_ids.zeros_like()?;

        // Forward pass (returns [batch, tokens, features])
        let embeddings = self.model.forward(&token_ids, &token_type_ids, None)?;

        // Remove batch dimension (squeeze first dimension)
        let embeddings = embeddings.squeeze(0)?;

        // Mean pooling over token dimension
        let (n_tokens, _n_features) = embeddings.dims2()?;
        let embeddings = (embeddings.sum(0)? / (n_tokens as f64))?;

        // Normalize
        let embedding_vec = embeddings.to_vec1::<f32>()?;
        let norm: f32 = embedding_vec.iter().map(|x| x * x).sum::<f32>().sqrt();
        let normalized = if norm > 0.0 {
            embedding_vec.iter().map(|x| x / norm).collect()
        } else {
            embedding_vec
        };

        Ok(normalized)
    }
}

#[async_trait]
impl Embedder for CandleEmbedder {
    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        // Candle operations are CPU-bound, but we run sync for simplicity
        self.embed_sync(text)
    }

    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        // Process sequentially (Candle models are typically fast enough)
        let mut results = Vec::with_capacity(texts.len());
        for text in texts {
            results.push(self.embed_sync(text)?);
        }
        Ok(results)
    }

    fn dimensions(&self) -> usize {
        self.dimensions
    }

    fn model_name(&self) -> &str {
        &self.model_name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_candle_model_path() {
        let model_dir = dirs::data_local_dir()
            .unwrap()
            .join("agentroot")
            .join("models")
            .join("sentence-transformers");
        println!("Expected model path: {}", model_dir.display());
    }
}
