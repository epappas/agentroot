//! LLM integration
//!
//! Provides traits and implementations for:
//! - Embedding generation
//! - Document reranking
//! - Query expansion
//! - Tokenization
//! - Metadata generation

mod candle_embedder;
mod llama;
mod llama_metadata;
mod metadata_generator;
mod query_parser;
mod traits;

pub use candle_embedder::{CandleEmbedder, DEFAULT_CANDLE_MODEL};
pub use llama::{LlamaEmbedder, DEFAULT_EMBED_MODEL};
pub use llama_metadata::{LlamaMetadataGenerator, DEFAULT_METADATA_MODEL};
pub use metadata_generator::{DocumentMetadata, MetadataContext, MetadataGenerator};
pub use query_parser::{MetadataFilterHint, ParsedQuery, QueryParser, SearchType, TemporalFilter};
pub use traits::*;
