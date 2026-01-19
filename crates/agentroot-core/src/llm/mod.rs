//! LLM integration
//!
//! Provides traits and implementations for:
//! - Embedding generation
//! - Document reranking
//! - Query expansion
//! - Tokenization
//! - Metadata generation

mod llama;
mod llama_metadata;
mod metadata_generator;
mod traits;

pub use llama::{LlamaEmbedder, DEFAULT_EMBED_MODEL};
pub use llama_metadata::{LlamaMetadataGenerator, DEFAULT_METADATA_MODEL};
pub use metadata_generator::{DocumentMetadata, MetadataContext, MetadataGenerator};
pub use traits::*;
