//! LLM integration
//!
//! Provides traits and implementations for:
//! - Embedding generation
//! - Document reranking
//! - Query expansion
//! - Tokenization

mod llama;
mod traits;

pub use llama::{LlamaEmbedder, DEFAULT_EMBED_MODEL};
pub use traits::*;
