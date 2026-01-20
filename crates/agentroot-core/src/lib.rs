//! Agentroot Core Library
//!
//! Core functionality for the agentroot local markdown search engine.
//!
//! # Features
//! - SQLite FTS5 full-text search with BM25 scoring
//! - Vector similarity search via sqlite-vec
//! - Hybrid search with Reciprocal Rank Fusion (RRF)
//! - LLM-powered query expansion and reranking
//! - Content-addressable storage with SHA-256

pub mod config;
pub mod db;
pub mod error;
pub mod index;
pub mod llm;
pub mod providers;
pub mod search;

pub use config::{CollectionConfig, Config};
pub use db::{Database, MetadataBuilder, MetadataFilter, MetadataValue, UserMetadata};
pub use error::{AgentRootError, Error, Result};
pub use index::{chunk_semantic, ChunkType, SemanticChunk, SemanticChunker};
pub use llm::{
    DocumentMetadata, Embedder, LlamaEmbedder, LlamaMetadataGenerator, MetadataContext,
    MetadataFilterHint, MetadataGenerator, ParsedQuery, QueryParser, SearchType, TemporalFilter,
    DEFAULT_EMBED_MODEL, DEFAULT_METADATA_MODEL,
};
pub use providers::{
    CSVProvider, FileProvider, GitHubProvider, JSONProvider, PDFProvider, ProviderConfig,
    ProviderRegistry, SQLProvider, SourceItem, SourceProvider, URLProvider,
};
pub use search::{smart_search, SearchOptions, SearchResult, SearchSource};

/// Virtual path prefix for agentroot URIs
pub const VIRTUAL_PATH_PREFIX: &str = "agentroot://";

/// Default cache directory name
pub const CACHE_DIR_NAME: &str = "agentroot";

/// Default config directory name
pub const CONFIG_DIR_NAME: &str = "agentroot";
