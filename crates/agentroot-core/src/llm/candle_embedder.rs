//! Candle-based embedder (pure Rust, no segfaults!)

use super::Embedder;
use crate::error::{AgentRootError, Result};
use async_trait::async_trait;
use candle_core::{Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::bert::{BertModel, Config, DTYPE};
use hf_hub::{api::sync::Api, Repo, RepoType};
use std::path::{Path, PathBuf};
use tokenizers::Tokenizer;

/// Default embedding model (BERT-based)
pub const DEFAULT_CANDLE_MODEL: &str = "sentence-transformers/all-MiniLM-L6-v2";

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

        // Load tokenizer
        let tokenizer_path = model_path.join("tokenizer.json");
        let tokenizer = Tokenizer::from_file(&tokenizer_path)
            .map_err(|e| AgentRootError::Llm(format!("Failed to load tokenizer: {}", e)))?;

        // Load config
        let config_path = model_path.join("config.json");
        let config_str = std::fs::read_to_string(&config_path)
            .map_err(|e| AgentRootError::Llm(format!("Failed to read config: {}", e)))?;
        let config: Config = serde_json::from_str(&config_str)
            .map_err(|e| AgentRootError::Llm(format!("Failed to parse config: {}", e)))?;

        // MiniLM-L6-v2 has 384 dimensions
        let dimensions = 384;

        // Use CPU for now (GPU support can be added later)
        let device = Device::Cpu;

        // Load model weights
        let weights_path = model_path.join("model.safetensors");
        let vb = unsafe { VarBuilder::from_mmaped_safetensors(&[weights_path], DTYPE, &device)? };

        let model = BertModel::load(vb, &config)
            .map_err(|e| AgentRootError::Llm(format!("Failed to load model: {}", e)))?;

        let model_name = model_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

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
        tracing::info!("Downloading model from Hugging Face: {}", model_name);

        let api =
            Api::new().map_err(|e| AgentRootError::Llm(format!("Failed to init HF API: {}", e)))?;

        let repo = api.model(model_name.to_string());

        // Download required files
        let config_path = repo
            .get("config.json")
            .map_err(|e| AgentRootError::Llm(format!("Failed to download config: {}", e)))?;
        let tokenizer_path = repo
            .get("tokenizer.json")
            .map_err(|e| AgentRootError::Llm(format!("Failed to download tokenizer: {}", e)))?;
        let weights_path = repo
            .get("model.safetensors")
            .map_err(|e| AgentRootError::Llm(format!("Failed to download weights: {}", e)))?;

        // Load tokenizer
        let tokenizer = Tokenizer::from_file(&tokenizer_path)
            .map_err(|e| AgentRootError::Llm(format!("Failed to load tokenizer: {}", e)))?;

        // Load config
        let config_str = std::fs::read_to_string(&config_path)
            .map_err(|e| AgentRootError::Llm(format!("Failed to read config: {}", e)))?;
        let config: Config = serde_json::from_str(&config_str)
            .map_err(|e| AgentRootError::Llm(format!("Failed to parse config: {}", e)))?;

        // MiniLM-L6-v2 has 384 dimensions
        let dimensions = 384;

        // Use CPU
        let device = Device::Cpu;

        // Load model weights
        let vb = unsafe { VarBuilder::from_mmaped_safetensors(&[weights_path], DTYPE, &device)? };

        let model = BertModel::load(vb, &config)
            .map_err(|e| AgentRootError::Llm(format!("Failed to load model: {}", e)))?;

        Ok(Self {
            model,
            tokenizer,
            device,
            model_name: model_name.to_string(),
            dimensions,
        })
    }

    /// Create from default model location
    pub fn from_default() -> Result<Self> {
        // Always download from Hugging Face (it caches locally)
        Self::from_hf(DEFAULT_CANDLE_MODEL)
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

        // Forward pass
        let embeddings = self.model.forward(&token_ids, &token_type_ids, None)?;

        // Mean pooling
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
