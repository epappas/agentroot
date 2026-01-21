//! LLM integration
//!
//! Provides traits and implementations for:
//! - Embedding generation via external services (vLLM, OpenAI, Basilica, etc.)
//! - Document metadata generation
//! - Query parsing
//! - Reranking
//!
//! All inference is performed via external HTTP services.
//! No local models are downloaded or executed.

mod cache;
mod client;
mod http_embedder;
mod http_metadata_generator;
mod http_query_parser;
mod metadata_generator;
mod query_parser;
mod traits;

pub use client::{generate_metadata_with_llm, ChatMessage, LLMClient, MetricsSnapshot, VLLMClient};
pub use http_embedder::HttpEmbedder;
pub use http_metadata_generator::HttpMetadataGenerator;
pub use http_query_parser::HttpQueryParser;
pub use metadata_generator::{DocumentMetadata, MetadataContext, MetadataGenerator};
pub use query_parser::{MetadataFilterHint, ParsedQuery, SearchType, TemporalFilter};
pub use traits::*;
