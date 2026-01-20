//! LLaMA-based embedder using llama-cpp-2

use super::Embedder;
use crate::error::{AgentRootError, Result};
use async_trait::async_trait;
use llama_cpp_2::{
    context::params::LlamaContextParams,
    llama_backend::LlamaBackend,
    llama_batch::LlamaBatch,
    model::{params::LlamaModelParams, LlamaModel},
};
use std::path::Path;
use std::sync::Mutex;

/// Default embedding model (nomic-embed-text or similar)
pub const DEFAULT_EMBED_MODEL: &str = "nomic-embed-text-v1.5.Q4_K_M.gguf";

/// LLaMA-based embedder
pub struct LlamaEmbedder {
    #[allow(dead_code)]
    backend: LlamaBackend,
    model: LlamaModel,
    context: Mutex<LlamaEmbedderContext>,
    model_name: String,
    dimensions: usize,
}

struct LlamaEmbedderContext {
    ctx: llama_cpp_2::context::LlamaContext<'static>,
}

unsafe impl Send for LlamaEmbedderContext {}
unsafe impl Sync for LlamaEmbedderContext {}

impl LlamaEmbedder {
    /// Create a new LlamaEmbedder from a GGUF model file
    pub fn new(model_path: impl AsRef<Path>) -> Result<Self> {
        let model_path = model_path.as_ref();
        let model_name = model_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        // Initialize backend and suppress verbose output
        let mut backend = LlamaBackend::init()
            .map_err(|e| AgentRootError::Llm(format!("Failed to init backend: {}", e)))?;
        backend.void_logs();

        // Load model
        let model_params = LlamaModelParams::default();
        let model = LlamaModel::load_from_file(&backend, model_path, &model_params)
            .map_err(|e| AgentRootError::Llm(format!("Failed to load model: {}", e)))?;

        let dimensions = model.n_embd() as usize;

        // Create context with embeddings enabled
        // n_batch and n_ubatch must be >= n_tokens for encoder models
        let ctx_size = std::num::NonZeroU32::new(2048).unwrap();
        let ctx_params = LlamaContextParams::default()
            .with_embeddings(true)
            .with_n_ctx(Some(ctx_size))
            .with_n_batch(ctx_size.get())
            .with_n_ubatch(ctx_size.get());

        let ctx = model
            .new_context(&backend, ctx_params)
            .map_err(|e| AgentRootError::Llm(format!("Failed to create context: {}", e)))?;

        // SAFETY: We're storing the model alongside the context and ensuring
        // the model outlives the context through the struct layout
        let ctx: llama_cpp_2::context::LlamaContext<'static> = unsafe { std::mem::transmute(ctx) };

        Ok(Self {
            backend,
            model,
            context: Mutex::new(LlamaEmbedderContext { ctx }),
            model_name,
            dimensions,
        })
    }

    /// Create from default model location
    pub fn from_default() -> Result<Self> {
        let model_dir = dirs::data_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("agentroot")
            .join("models");

        let model_path = model_dir.join(DEFAULT_EMBED_MODEL);

        if !model_path.exists() {
            return Err(AgentRootError::ModelNotFound(format!(
                "Model not found at {}. Download an embedding model (e.g., nomic-embed-text) to this location.",
                model_path.display()
            )));
        }

        Self::new(model_path)
    }

    fn embed_sync(&self, text: &str) -> Result<Vec<f32>> {
        let mut ctx_guard = self
            .context
            .lock()
            .map_err(|e| AgentRootError::Llm(format!("Lock error: {}", e)))?;

        // Tokenize
        let tokens = self
            .model
            .str_to_token(text, llama_cpp_2::model::AddBos::Always)
            .map_err(|e| AgentRootError::Llm(format!("Tokenization error: {}", e)))?;

        if tokens.is_empty() {
            return Ok(vec![0.0; self.dimensions]);
        }

        // Create batch
        let mut batch = LlamaBatch::new(tokens.len(), 1);

        for (i, token) in tokens.iter().enumerate() {
            batch
                .add(*token, i as i32, &[0], i == tokens.len() - 1)
                .map_err(|e| AgentRootError::Llm(format!("Batch error: {}", e)))?;
        }

        // Encode (for embeddings)
        ctx_guard
            .ctx
            .encode(&mut batch)
            .map_err(|e| AgentRootError::Llm(format!("Encode error: {}", e)))?;

        // Get embeddings (sequence-level pooled embedding)
        let embeddings = ctx_guard
            .ctx
            .embeddings_seq_ith(0)
            .map_err(|e| AgentRootError::Llm(format!("Embeddings error: {}", e)))?;

        // Normalize the embedding
        let norm: f32 = embeddings.iter().map(|x| x * x).sum::<f32>().sqrt();
        let normalized: Vec<f32> = if norm > 0.0 {
            embeddings.iter().map(|x| x / norm).collect()
        } else {
            embeddings.to_vec()
        };

        Ok(normalized)
    }
}

#[async_trait]
impl Embedder for LlamaEmbedder {
    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        // Run synchronously since llama-cpp context is not async
        self.embed_sync(text)
    }

    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        // Process sequentially (context is not thread-safe)
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
    fn test_default_model_path() {
        let model_dir = dirs::data_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("agentroot")
            .join("models");
        let model_path = model_dir.join(DEFAULT_EMBED_MODEL);
        println!("Expected model path: {}", model_path.display());
    }
}
