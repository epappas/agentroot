//! Database layer for agentroot
//!
//! Provides SQLite-based storage with:
//! - FTS5 full-text search
//! - sqlite-vec vector storage
//! - Content-addressable storage

mod chunks;
mod collections;
mod content;
mod context;
pub mod directories;
mod documents;
pub mod glossary;
pub mod metadata;
mod pagerank;
mod schema;
pub mod sessions;
mod stats;
mod user_metadata;
pub mod vectors;

pub use chunks::ChunkInfo;
pub use collections::CollectionInfo;
pub use content::{docid_from_hash, hash_content};
pub use context::ContextInfo;
pub use directories::DirectoryInfo;
pub use documents::{Document, DocumentInsert};
pub use glossary::{ConceptChunkInfo, ConceptInfo};
pub use metadata::{MetadataBuilder, MetadataFilter, MetadataValue, UserMetadata};
pub use schema::Database;
pub use sessions::{SessionInfo, SessionQuery};
use std::path::PathBuf;
pub use vectors::CacheLookupResult;

impl Database {
    /// Get the default database path
    pub fn default_path() -> PathBuf {
        dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(crate::CACHE_DIR_NAME)
            .join("index.sqlite")
    }
}
