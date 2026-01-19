//! Database schema and initialization

use rusqlite::{Connection, params};
use std::path::Path;
use crate::error::Result;

/// Main database handle
pub struct Database {
    pub(crate) conn: Connection,
}

const SCHEMA_VERSION: i32 = 2;

const CREATE_TABLES: &str = r#"
-- Content storage (content-addressable by SHA-256 hash)
CREATE TABLE IF NOT EXISTS content (
    hash TEXT PRIMARY KEY,
    doc TEXT NOT NULL,
    created_at TEXT NOT NULL
);

-- Document metadata
CREATE TABLE IF NOT EXISTS documents (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    collection TEXT NOT NULL,
    path TEXT NOT NULL,
    title TEXT NOT NULL,
    hash TEXT NOT NULL REFERENCES content(hash),
    created_at TEXT NOT NULL,
    modified_at TEXT NOT NULL,
    active INTEGER NOT NULL DEFAULT 1,
    UNIQUE(collection, path)
);

-- Full-text search index
CREATE VIRTUAL TABLE IF NOT EXISTS documents_fts USING fts5(
    filepath,
    title,
    body,
    tokenize='porter unicode61'
);

-- Vector embeddings metadata
CREATE TABLE IF NOT EXISTS content_vectors (
    hash TEXT NOT NULL,
    seq INTEGER NOT NULL,
    pos INTEGER NOT NULL,
    model TEXT NOT NULL,
    chunk_hash TEXT,
    created_at TEXT NOT NULL,
    PRIMARY KEY (hash, seq)
);

-- Model metadata for dimension validation
CREATE TABLE IF NOT EXISTS model_metadata (
    model TEXT PRIMARY KEY,
    dimensions INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    last_used_at TEXT NOT NULL
);

-- Global chunk embeddings cache
CREATE TABLE IF NOT EXISTS chunk_embeddings (
    chunk_hash TEXT NOT NULL,
    model TEXT NOT NULL,
    embedding BLOB NOT NULL,
    created_at TEXT NOT NULL,
    PRIMARY KEY (chunk_hash, model)
);

-- LLM response cache
CREATE TABLE IF NOT EXISTS llm_cache (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    model TEXT NOT NULL,
    created_at TEXT NOT NULL
);

-- Collections metadata
CREATE TABLE IF NOT EXISTS collections (
    name TEXT PRIMARY KEY,
    path TEXT NOT NULL,
    pattern TEXT NOT NULL DEFAULT '**/*.md',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Context metadata (hierarchical context for paths)
CREATE TABLE IF NOT EXISTS contexts (
    path TEXT PRIMARY KEY,
    context TEXT NOT NULL,
    created_at TEXT NOT NULL
);

-- Schema version tracking
CREATE TABLE IF NOT EXISTS schema_version (
    version INTEGER PRIMARY KEY
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_documents_collection ON documents(collection);
CREATE INDEX IF NOT EXISTS idx_documents_hash ON documents(hash);
CREATE INDEX IF NOT EXISTS idx_documents_active ON documents(active);
CREATE INDEX IF NOT EXISTS idx_content_vectors_hash ON content_vectors(hash);
CREATE INDEX IF NOT EXISTS idx_content_vectors_chunk_hash ON content_vectors(chunk_hash);
CREATE INDEX IF NOT EXISTS idx_chunk_embeddings_hash ON chunk_embeddings(chunk_hash);
"#;

const CREATE_TRIGGERS: &str = r#"
-- Sync FTS on insert (only for active documents)
CREATE TRIGGER IF NOT EXISTS documents_ai
AFTER INSERT ON documents
WHEN new.active = 1
BEGIN
    INSERT INTO documents_fts(rowid, filepath, title, body)
    SELECT
        new.id,
        new.collection || '/' || new.path,
        new.title,
        (SELECT doc FROM content WHERE hash = new.hash);
END;

-- Sync FTS on update: handle activation/deactivation/content change
CREATE TRIGGER IF NOT EXISTS documents_au
AFTER UPDATE ON documents
BEGIN
    DELETE FROM documents_fts WHERE rowid = old.id;
    INSERT INTO documents_fts(rowid, filepath, title, body)
    SELECT
        new.id,
        new.collection || '/' || new.path,
        new.title,
        (SELECT doc FROM content WHERE hash = new.hash)
    WHERE new.active = 1;
END;

-- Sync FTS on delete
CREATE TRIGGER IF NOT EXISTS documents_ad
AFTER DELETE ON documents
BEGIN
    DELETE FROM documents_fts WHERE rowid = old.id;
END;
"#;

impl Database {
    /// Open database at path, creating if necessary
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(path)?;
        Ok(Self { conn })
    }

    /// Open in-memory database (for testing)
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        Ok(Self { conn })
    }

    /// Initialize database schema
    pub fn initialize(&self) -> Result<()> {
        // Set PRAGMAs for performance
        self.conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA foreign_keys = ON;
             PRAGMA cache_size = -64000;
             PRAGMA busy_timeout = 5000;"
        )?;

        // Create tables
        self.conn.execute_batch(CREATE_TABLES)?;

        // Create triggers
        self.conn.execute_batch(CREATE_TRIGGERS)?;

        // Set schema version
        self.conn.execute(
            "INSERT OR IGNORE INTO schema_version (version) VALUES (?1)",
            params![SCHEMA_VERSION],
        )?;

        Ok(())
    }

    /// Get current schema version
    pub fn schema_version(&self) -> Result<Option<i32>> {
        let version = self.conn.query_row(
            "SELECT version FROM schema_version ORDER BY version DESC LIMIT 1",
            [],
            |row| row.get(0),
        ).ok();
        Ok(version)
    }

    /// Run migrations to upgrade schema to current version
    pub fn migrate(&self) -> Result<()> {
        let current = self.schema_version()?.unwrap_or(0);

        if current < 2 {
            self.migrate_to_v2()?;
        }

        Ok(())
    }

    fn migrate_to_v2(&self) -> Result<()> {
        // Add chunk_hash column to content_vectors if not exists
        let has_chunk_hash: bool = self.conn.query_row(
            "SELECT COUNT(*) > 0 FROM pragma_table_info('content_vectors') WHERE name = 'chunk_hash'",
            [],
            |row| row.get(0),
        ).unwrap_or(false);

        if !has_chunk_hash {
            self.conn.execute(
                "ALTER TABLE content_vectors ADD COLUMN chunk_hash TEXT",
                [],
            )?;
            self.conn.execute(
                "CREATE INDEX IF NOT EXISTS idx_content_vectors_chunk_hash ON content_vectors(chunk_hash)",
                [],
            )?;
        }

        // Create model_metadata table
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS model_metadata (
                model TEXT PRIMARY KEY,
                dimensions INTEGER NOT NULL,
                created_at TEXT NOT NULL,
                last_used_at TEXT NOT NULL
            )",
            [],
        )?;

        // Create chunk_embeddings cache table
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS chunk_embeddings (
                chunk_hash TEXT NOT NULL,
                model TEXT NOT NULL,
                embedding BLOB NOT NULL,
                created_at TEXT NOT NULL,
                PRIMARY KEY (chunk_hash, model)
            )",
            [],
        )?;
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_chunk_embeddings_hash ON chunk_embeddings(chunk_hash)",
            [],
        )?;

        // Update schema version
        self.conn.execute(
            "INSERT OR REPLACE INTO schema_version (version) VALUES (?1)",
            params![2],
        )?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_open_in_memory() {
        let db = Database::open_in_memory().unwrap();
        db.initialize().unwrap();
        assert_eq!(db.schema_version().unwrap(), Some(SCHEMA_VERSION));
    }
}
