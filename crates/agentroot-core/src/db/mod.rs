//! Database layer for agentroot
//!
//! Provides SQLite-based storage with:
//! - FTS5 full-text search
//! - sqlite-vec vector storage
//! - Content-addressable storage

mod collections;
mod content;
mod context;
mod documents;
mod schema;
mod stats;
pub mod vectors;

pub use collections::CollectionInfo;
pub use content::{docid_from_hash, hash_content};
pub use context::ContextInfo;
pub use documents::DocumentInsert;
pub use schema::Database;
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
