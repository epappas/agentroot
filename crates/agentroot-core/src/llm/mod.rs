//! LLM integration
//!
//! Provides traits and implementations for:
//! - Embedding generation via external services (vLLM, OpenAI, etc.)
//! - Document metadata generation
//! - Query parsing
//! - Reranking

mod cache;
mod client;
mod http_embedder;
mod http_metadata_generator;
mod http_query_parser;
mod llama;
mod llama_metadata;
mod metadata_generator;
mod query_parser;
mod traits;

pub use client::{generate_metadata_with_llm, ChatMessage, LLMClient, VLLMClient};
pub use http_embedder::HttpEmbedder;
pub use http_metadata_generator::HttpMetadataGenerator;
pub use http_query_parser::HttpQueryParser;
pub use llama::{LlamaEmbedder, DEFAULT_EMBED_MODEL};
pub use llama_metadata::{LlamaMetadataGenerator, DEFAULT_METADATA_MODEL};
pub use metadata_generator::{DocumentMetadata, MetadataContext, MetadataGenerator};
pub use query_parser::{MetadataFilterHint, ParsedQuery, QueryParser, SearchType, TemporalFilter};
pub use traits::*;
