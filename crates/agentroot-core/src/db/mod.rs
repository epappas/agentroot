//! Database layer for agentroot
//!
//! Provides SQLite-based storage with:
//! - FTS5 full-text search
//! - sqlite-vec vector storage
//! - Content-addressable storage

mod schema;
mod content;
mod documents;
pub mod vectors;
mod collections;
mod context;
mod stats;

pub use schema::Database;
pub use content::{hash_content, docid_from_hash};
pub use collections::CollectionInfo;
pub use context::ContextInfo;
pub use vectors::CacheLookupResult;
use std::path::PathBuf;

impl Database {
    /// Get the default database path
    pub fn default_path() -> PathBuf {
        dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(crate::CACHE_DIR_NAME)
            .join("index.sqlite")
    }
}
