//! LLM integration
//!
//! Provides traits and implementations for:
//! - Embedding generation
//! - Document reranking
//! - Query expansion
//! - Tokenization

mod traits;
mod llama;

pub use traits::*;
pub use llama::{LlamaEmbedder, DEFAULT_EMBED_MODEL};
