//! Database schema and initialization

use crate::error::Result;
use rusqlite::{params, Connection};
use std::path::Path;

/// Main database handle
pub struct Database {
    pub(crate) conn: Connection,
}

const SCHEMA_VERSION: i32 = 9;

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
    source_type TEXT NOT NULL DEFAULT 'file',
    source_uri TEXT,
    llm_summary TEXT,
    llm_title TEXT,
    llm_keywords TEXT,
    llm_category TEXT,
    llm_intent TEXT,
    llm_concepts TEXT,
    llm_difficulty TEXT,
    llm_queries TEXT,
    llm_metadata_generated_at TEXT,
    llm_model TEXT,
    user_metadata TEXT,
    UNIQUE(collection, path)
);

-- Full-text search index
CREATE VIRTUAL TABLE IF NOT EXISTS documents_fts USING fts5(
    filepath,
    title,
    body,
    llm_summary,
    llm_title,
    llm_keywords,
    llm_intent,
    llm_concepts,
    user_metadata,
    modified_at,
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
    updated_at TEXT NOT NULL,
    provider_type TEXT NOT NULL DEFAULT 'file',
    provider_config TEXT
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
    INSERT INTO documents_fts(rowid, filepath, title, body, llm_summary, llm_title, llm_keywords, llm_intent, llm_concepts, user_metadata, modified_at)
    SELECT
        new.id,
        new.collection || '/' || new.path,
        new.title,
        (SELECT doc FROM content WHERE hash = new.hash),
        new.llm_summary,
        new.llm_title,
        new.llm_keywords,
        new.llm_intent,
        new.llm_concepts,
        new.user_metadata,
        new.modified_at;
END;

-- Sync FTS on update: handle activation/deactivation/content change
CREATE TRIGGER IF NOT EXISTS documents_au
AFTER UPDATE ON documents
BEGIN
    DELETE FROM documents_fts WHERE rowid = old.id;
    INSERT INTO documents_fts(rowid, filepath, title, body, llm_summary, llm_title, llm_keywords, llm_intent, llm_concepts, user_metadata, modified_at)
    SELECT
        new.id,
        new.collection || '/' || new.path,
        new.title,
        (SELECT doc FROM content WHERE hash = new.hash),
        new.llm_summary,
        new.llm_title,
        new.llm_keywords,
        new.llm_intent,
        new.llm_concepts,
        new.user_metadata,
        new.modified_at
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
             PRAGMA busy_timeout = 5000;",
        )?;

        // Create tables
        self.conn.execute_batch(CREATE_TABLES)?;

        // Create triggers
        self.conn.execute_batch(CREATE_TRIGGERS)?;

        // Run migrations to upgrade existing databases (BEFORE setting version)
        self.migrate()?;

        // Set schema version (after migrations complete)
        self.conn.execute(
            "INSERT OR REPLACE INTO schema_version (version) VALUES (?1)",
            params![SCHEMA_VERSION],
        )?;

        Ok(())
    }

    /// Get current schema version
    pub fn schema_version(&self) -> Result<Option<i32>> {
        let version = self
            .conn
            .query_row(
                "SELECT version FROM schema_version ORDER BY version DESC LIMIT 1",
                [],
                |row| row.get(0),
            )
            .ok();
        Ok(version)
    }

    /// Run migrations to upgrade schema to current version
    pub fn migrate(&self) -> Result<()> {
        let current = self.schema_version()?.unwrap_or(0);

        if current < 2 {
            self.migrate_to_v2()?;
        }

        if current < 3 {
            self.migrate_to_v3()?;
        }

        if current < 4 {
            self.migrate_to_v4()?;
        }

        if current < 5 {
            self.migrate_to_v5()?;
        }

        if current < 6 {
            self.migrate_to_v6()?;
        }

        if current < 7 {
            self.migrate_to_v7()?;
        }

        if current < 8 {
            self.migrate_to_v8()?;
        }

        if current < 9 {
            self.migrate_to_v9()?;
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
            self.conn
                .execute("ALTER TABLE content_vectors ADD COLUMN chunk_hash TEXT", [])?;
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

    fn migrate_to_v3(&self) -> Result<()> {
        // Add source_type column to documents if not exists
        let has_source_type: bool = self
            .conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM pragma_table_info('documents') WHERE name = 'source_type'",
                [],
                |row| row.get(0),
            )
            .unwrap_or(false);

        if !has_source_type {
            self.conn.execute(
                "ALTER TABLE documents ADD COLUMN source_type TEXT NOT NULL DEFAULT 'file'",
                [],
            )?;
        }

        // Add source_uri column to documents if not exists
        let has_source_uri: bool = self
            .conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM pragma_table_info('documents') WHERE name = 'source_uri'",
                [],
                |row| row.get(0),
            )
            .unwrap_or(false);

        if !has_source_uri {
            self.conn
                .execute("ALTER TABLE documents ADD COLUMN source_uri TEXT", [])?;
        }

        // Add provider_type column to collections if not exists
        let has_provider_type: bool = self
            .conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM pragma_table_info('collections') WHERE name = 'provider_type'",
                [],
                |row| row.get(0),
            )
            .unwrap_or(false);

        if !has_provider_type {
            self.conn.execute(
                "ALTER TABLE collections ADD COLUMN provider_type TEXT NOT NULL DEFAULT 'file'",
                [],
            )?;
        }

        // Add provider_config column to collections if not exists
        let has_provider_config: bool = self
            .conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM pragma_table_info('collections') WHERE name = 'provider_config'",
                [],
                |row| row.get(0),
            )
            .unwrap_or(false);

        if !has_provider_config {
            self.conn.execute(
                "ALTER TABLE collections ADD COLUMN provider_config TEXT",
                [],
            )?;
        }

        // Update schema version
        self.conn.execute(
            "INSERT OR REPLACE INTO schema_version (version) VALUES (?1)",
            params![3],
        )?;

        Ok(())
    }

    fn migrate_to_v4(&self) -> Result<()> {
        // Add LLM metadata columns to documents table
        let columns_to_add = vec![
            "llm_summary",
            "llm_title",
            "llm_keywords",
            "llm_category",
            "llm_intent",
            "llm_concepts",
            "llm_difficulty",
            "llm_queries",
            "llm_metadata_generated_at",
            "llm_model",
        ];

        for column in columns_to_add {
            let has_column: bool = self
                .conn
                .query_row(
                    "SELECT COUNT(*) > 0 FROM pragma_table_info('documents') WHERE name = ?1",
                    params![column],
                    |row| row.get(0),
                )
                .unwrap_or(false);

            if !has_column {
                self.conn.execute(
                    &format!("ALTER TABLE documents ADD COLUMN {} TEXT", column),
                    [],
                )?;
            }
        }

        // Rebuild FTS index to include metadata columns
        // Drop and recreate is the safest approach for FTS5
        self.conn
            .execute("DROP TABLE IF EXISTS documents_fts", [])?;
        self.conn.execute(
            "CREATE VIRTUAL TABLE documents_fts USING fts5(
                filepath,
                title,
                body,
                llm_summary,
                llm_title,
                llm_keywords,
                llm_intent,
                llm_concepts,
                tokenize='porter unicode61'
            )",
            [],
        )?;

        // Rebuild FTS data from existing documents
        self.conn.execute(
            "INSERT INTO documents_fts(rowid, filepath, title, body, llm_summary, llm_title, llm_keywords, llm_intent, llm_concepts)
             SELECT
                d.id,
                d.collection || '/' || d.path,
                d.title,
                c.doc,
                d.llm_summary,
                d.llm_title,
                d.llm_keywords,
                d.llm_intent,
                d.llm_concepts
             FROM documents d
             JOIN content c ON c.hash = d.hash
             WHERE d.active = 1",
            [],
        )?;

        // Recreate triggers with metadata support
        self.conn
            .execute("DROP TRIGGER IF EXISTS documents_ai", [])?;
        self.conn
            .execute("DROP TRIGGER IF EXISTS documents_au", [])?;
        self.conn
            .execute("DROP TRIGGER IF EXISTS documents_ad", [])?;

        self.conn.execute(
            "CREATE TRIGGER documents_ai
             AFTER INSERT ON documents
             WHEN new.active = 1
             BEGIN
                 INSERT INTO documents_fts(rowid, filepath, title, body, llm_summary, llm_title, llm_keywords, llm_intent, llm_concepts)
                 SELECT
                     new.id,
                     new.collection || '/' || new.path,
                     new.title,
                     (SELECT doc FROM content WHERE hash = new.hash),
                     new.llm_summary,
                     new.llm_title,
                     new.llm_keywords,
                     new.llm_intent,
                     new.llm_concepts;
             END",
            [],
        )?;

        self.conn.execute(
            "CREATE TRIGGER documents_au
             AFTER UPDATE ON documents
             BEGIN
                 DELETE FROM documents_fts WHERE rowid = old.id;
                 INSERT INTO documents_fts(rowid, filepath, title, body, llm_summary, llm_title, llm_keywords, llm_intent, llm_concepts)
                 SELECT
                     new.id,
                     new.collection || '/' || new.path,
                     new.title,
                     (SELECT doc FROM content WHERE hash = new.hash),
                     new.llm_summary,
                     new.llm_title,
                     new.llm_keywords,
                     new.llm_intent,
                     new.llm_concepts
                 WHERE new.active = 1;
             END",
            [],
        )?;

        self.conn.execute(
            "CREATE TRIGGER documents_ad
             AFTER DELETE ON documents
             BEGIN
                 DELETE FROM documents_fts WHERE rowid = old.id;
             END",
            [],
        )?;

        // Update schema version
        self.conn.execute(
            "INSERT OR REPLACE INTO schema_version (version) VALUES (?1)",
            params![4],
        )?;

        Ok(())
    }

    fn migrate_to_v5(&self) -> Result<()> {
        // Add user_metadata column to documents table
        let has_user_metadata: bool = self
            .conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM pragma_table_info('documents') WHERE name = 'user_metadata'",
                [],
                |row| row.get(0),
            )
            .unwrap_or(false);

        if !has_user_metadata {
            self.conn
                .execute("ALTER TABLE documents ADD COLUMN user_metadata TEXT", [])?;
        }

        // Create index on user_metadata for efficient queries
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_documents_user_metadata ON documents(user_metadata)",
            [],
        )?;

        // Update schema version
        self.conn.execute(
            "INSERT OR REPLACE INTO schema_version (version) VALUES (?1)",
            params![5],
        )?;

        Ok(())
    }

    fn migrate_to_v6(&self) -> Result<()> {
        // Rebuild FTS index to include user_metadata and modified_at
        // This makes user metadata and timestamps full-text searchable

        // Drop and recreate FTS table with new columns
        self.conn
            .execute("DROP TABLE IF EXISTS documents_fts", [])?;

        self.conn.execute(
            "CREATE VIRTUAL TABLE documents_fts USING fts5(
                filepath,
                title,
                body,
                llm_summary,
                llm_title,
                llm_keywords,
                llm_intent,
                llm_concepts,
                user_metadata,
                modified_at,
                tokenize='porter unicode61'
            )",
            [],
        )?;

        // Rebuild FTS data from existing documents
        self.conn.execute(
            "INSERT INTO documents_fts(rowid, filepath, title, body, llm_summary, llm_title, llm_keywords, llm_intent, llm_concepts, user_metadata, modified_at)
             SELECT
                d.id,
                d.collection || '/' || d.path,
                d.title,
                c.doc,
                d.llm_summary,
                d.llm_title,
                d.llm_keywords,
                d.llm_intent,
                d.llm_concepts,
                d.user_metadata,
                d.modified_at
             FROM documents d
             JOIN content c ON c.hash = d.hash
             WHERE d.active = 1",
            [],
        )?;

        // Recreate triggers with user_metadata and modified_at support
        self.conn
            .execute("DROP TRIGGER IF EXISTS documents_ai", [])?;
        self.conn
            .execute("DROP TRIGGER IF EXISTS documents_au", [])?;
        self.conn
            .execute("DROP TRIGGER IF EXISTS documents_ad", [])?;

        self.conn.execute(
            "CREATE TRIGGER documents_ai
             AFTER INSERT ON documents
             WHEN new.active = 1
             BEGIN
                 INSERT INTO documents_fts(rowid, filepath, title, body, llm_summary, llm_title, llm_keywords, llm_intent, llm_concepts, user_metadata, modified_at)
                 SELECT
                     new.id,
                     new.collection || '/' || new.path,
                     new.title,
                     (SELECT doc FROM content WHERE hash = new.hash),
                     new.llm_summary,
                     new.llm_title,
                     new.llm_keywords,
                     new.llm_intent,
                     new.llm_concepts,
                     new.user_metadata,
                     new.modified_at;
             END",
            [],
        )?;

        self.conn.execute(
            "CREATE TRIGGER documents_au
             AFTER UPDATE ON documents
             BEGIN
                 DELETE FROM documents_fts WHERE rowid = old.id;
                 INSERT INTO documents_fts(rowid, filepath, title, body, llm_summary, llm_title, llm_keywords, llm_intent, llm_concepts, user_metadata, modified_at)
                 SELECT
                     new.id,
                     new.collection || '/' || new.path,
                     new.title,
                     (SELECT doc FROM content WHERE hash = new.hash),
                     new.llm_summary,
                     new.llm_title,
                     new.llm_keywords,
                     new.llm_intent,
                     new.llm_concepts,
                     new.user_metadata,
                     new.modified_at
                 WHERE new.active = 1;
             END",
            [],
        )?;

        self.conn.execute(
            "CREATE TRIGGER documents_ad
             AFTER DELETE ON documents
             BEGIN
                 DELETE FROM documents_fts WHERE rowid = old.id;
             END",
            [],
        )?;

        // Update schema version
        self.conn.execute(
            "INSERT OR REPLACE INTO schema_version (version) VALUES (?1)",
            params![6],
        )?;

        Ok(())
    }

    fn migrate_to_v7(&self) -> Result<()> {
        // Add intelligent glossary support for semantic concept discovery
        // This adds concept extraction and linking for enhanced search

        // Create concepts table (global across all collections)
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS concepts (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                term TEXT NOT NULL UNIQUE,
                normalized TEXT NOT NULL,
                chunk_count INTEGER DEFAULT 0,
                created_at TEXT NOT NULL
            )",
            [],
        )?;

        // Create indexes for concepts
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_concepts_normalized ON concepts(normalized)",
            [],
        )?;
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_concepts_term ON concepts(term)",
            [],
        )?;

        // Create concept_chunks table (links concepts to specific chunks)
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS concept_chunks (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                concept_id INTEGER NOT NULL,
                chunk_hash TEXT NOT NULL,
                document_hash TEXT NOT NULL,
                snippet TEXT,
                created_at TEXT NOT NULL,
                UNIQUE(concept_id, chunk_hash),
                FOREIGN KEY (concept_id) REFERENCES concepts(id)
            )",
            [],
        )?;

        // Create indexes for concept_chunks
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_concept_chunks_concept ON concept_chunks(concept_id)",
            [],
        )?;
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_concept_chunks_chunk ON concept_chunks(chunk_hash)",
            [],
        )?;
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_concept_chunks_doc ON concept_chunks(document_hash)",
            [],
        )?;

        // Create FTS index for concept search
        self.conn.execute(
            "CREATE VIRTUAL TABLE IF NOT EXISTS concepts_fts USING fts5(
                term,
                normalized,
                tokenize='porter unicode61'
            )",
            [],
        )?;

        // Create triggers to sync concepts_fts
        self.conn.execute(
            "CREATE TRIGGER IF NOT EXISTS concepts_ai
             AFTER INSERT ON concepts
             BEGIN
                 INSERT INTO concepts_fts(rowid, term, normalized)
                 VALUES (new.id, new.term, new.normalized);
             END",
            [],
        )?;

        self.conn.execute(
            "CREATE TRIGGER IF NOT EXISTS concepts_au
             AFTER UPDATE ON concepts
             BEGIN
                 DELETE FROM concepts_fts WHERE rowid = old.id;
                 INSERT INTO concepts_fts(rowid, term, normalized)
                 VALUES (new.id, new.term, new.normalized);
             END",
            [],
        )?;

        self.conn.execute(
            "CREATE TRIGGER IF NOT EXISTS concepts_ad
             AFTER DELETE ON concepts
             BEGIN
                 DELETE FROM concepts_fts WHERE rowid = old.id;
             END",
            [],
        )?;

        // Update schema version
        self.conn.execute(
            "INSERT OR REPLACE INTO schema_version (version) VALUES (?1)",
            params![7],
        )?;

        Ok(())
    }

    fn migrate_to_v8(&self) -> Result<()> {
        // Add chunk-level storage and LLM-generated chunk metadata
        // This enables returning specific chunks instead of whole documents

        // Create chunks table - stores chunk content and metadata
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS chunks (
                hash TEXT PRIMARY KEY,
                document_hash TEXT NOT NULL REFERENCES content(hash),
                seq INTEGER NOT NULL,
                pos INTEGER NOT NULL,
                content TEXT NOT NULL,
                chunk_type TEXT,
                breadcrumb TEXT,
                start_line INTEGER,
                end_line INTEGER,
                language TEXT,
                llm_summary TEXT,
                llm_purpose TEXT,
                llm_concepts TEXT,
                llm_labels TEXT,
                llm_related_to TEXT,
                llm_model TEXT,
                llm_generated_at TEXT,
                created_at TEXT NOT NULL,
                UNIQUE(document_hash, seq)
            )",
            [],
        )?;

        // Create indexes for chunks
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_chunks_document ON chunks(document_hash)",
            [],
        )?;
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_chunks_type ON chunks(chunk_type)",
            [],
        )?;
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_chunks_breadcrumb ON chunks(breadcrumb)",
            [],
        )?;

        // Create chunk_labels table for normalized labels
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS chunk_labels (
                chunk_hash TEXT NOT NULL REFERENCES chunks(hash),
                key TEXT NOT NULL,
                value TEXT NOT NULL,
                PRIMARY KEY (chunk_hash, key, value)
            )",
            [],
        )?;

        // Create indexes for chunk_labels
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_chunk_labels_key ON chunk_labels(key)",
            [],
        )?;
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_chunk_labels_value ON chunk_labels(value)",
            [],
        )?;

        // Create FTS index for chunk search
        self.conn.execute(
            "CREATE VIRTUAL TABLE IF NOT EXISTS chunks_fts USING fts5(
                content,
                breadcrumb,
                llm_summary,
                llm_purpose,
                content='chunks',
                content_rowid='rowid',
                tokenize='porter unicode61'
            )",
            [],
        )?;

        // Create triggers to sync chunks_fts
        self.conn.execute(
            "CREATE TRIGGER IF NOT EXISTS chunks_ai
             AFTER INSERT ON chunks
             BEGIN
                 INSERT INTO chunks_fts(rowid, content, breadcrumb, llm_summary, llm_purpose)
                 VALUES (new.rowid, new.content, new.breadcrumb, new.llm_summary, new.llm_purpose);
             END",
            [],
        )?;

        self.conn.execute(
            "CREATE TRIGGER IF NOT EXISTS chunks_au
             AFTER UPDATE ON chunks
             BEGIN
                 DELETE FROM chunks_fts WHERE rowid = old.rowid;
                 INSERT INTO chunks_fts(rowid, content, breadcrumb, llm_summary, llm_purpose)
                 VALUES (new.rowid, new.content, new.breadcrumb, new.llm_summary, new.llm_purpose);
             END",
            [],
        )?;

        self.conn.execute(
            "CREATE TRIGGER IF NOT EXISTS chunks_ad
             AFTER DELETE ON chunks
             BEGIN
                 DELETE FROM chunks_fts WHERE rowid = old.rowid;
             END",
            [],
        )?;

        // Update schema version
        self.conn.execute(
            "INSERT OR REPLACE INTO schema_version (version) VALUES (?1)",
            params![8],
        )?;

        Ok(())
    }

    fn migrate_to_v9(&self) -> Result<()> {
        // Add PageRank support: document_links table and importance_score column

        // Add importance_score column to documents table
        let has_importance: bool = self
            .conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM pragma_table_info('documents') WHERE name = 'importance_score'",
                [],
                |row| row.get(0),
            )
            .unwrap_or(false);

        if !has_importance {
            self.conn.execute(
                "ALTER TABLE documents ADD COLUMN importance_score REAL DEFAULT 1.0",
                [],
            )?;
            self.conn.execute(
                "CREATE INDEX IF NOT EXISTS idx_documents_importance ON documents(importance_score DESC)",
                [],
            )?;
        }

        // Create document_links table (if not exists)
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS document_links (
                source_id INTEGER NOT NULL,
                target_id INTEGER NOT NULL,
                link_type TEXT NOT NULL,
                created_at TEXT NOT NULL,
                FOREIGN KEY (source_id) REFERENCES documents(id) ON DELETE CASCADE,
                FOREIGN KEY (target_id) REFERENCES documents(id) ON DELETE CASCADE,
                PRIMARY KEY (source_id, target_id, link_type)
            )",
            [],
        )?;

        // Create indexes for document_links
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_document_links_source ON document_links(source_id)",
            [],
        )?;
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_document_links_target ON document_links(target_id)",
            [],
        )?;

        // Update schema version
        self.conn.execute(
            "INSERT OR REPLACE INTO schema_version (version) VALUES (?1)",
            params![9],
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

    #[test]
    fn test_migration_v2_to_v3() {
        let db = Database::open_in_memory().unwrap();

        db.conn
            .execute_batch(
                "CREATE TABLE collections (
                name TEXT PRIMARY KEY,
                path TEXT NOT NULL,
                pattern TEXT NOT NULL DEFAULT '**/*.md',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE TABLE documents (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                collection TEXT NOT NULL,
                path TEXT NOT NULL,
                title TEXT NOT NULL,
                hash TEXT NOT NULL,
                created_at TEXT NOT NULL,
                modified_at TEXT NOT NULL,
                active INTEGER NOT NULL DEFAULT 1,
                UNIQUE(collection, path)
            );
            CREATE TABLE schema_version (version INTEGER PRIMARY KEY);
            INSERT INTO schema_version VALUES (2);",
            )
            .unwrap();

        assert_eq!(db.schema_version().unwrap(), Some(2));

        db.initialize().unwrap();

        assert_eq!(db.schema_version().unwrap(), Some(9));

        let has_provider_type: bool = db.conn.query_row(
            "SELECT COUNT(*) > 0 FROM pragma_table_info('collections') WHERE name = 'provider_type'",
            [],
            |row| row.get(0),
        ).unwrap();
        assert!(
            has_provider_type,
            "collections should have provider_type column"
        );

        let has_provider_config: bool = db.conn.query_row(
            "SELECT COUNT(*) > 0 FROM pragma_table_info('collections') WHERE name = 'provider_config'",
            [],
            |row| row.get(0),
        ).unwrap();
        assert!(
            has_provider_config,
            "collections should have provider_config column"
        );

        let has_source_type: bool = db.conn.query_row(
            "SELECT COUNT(*) > 0 FROM pragma_table_info('documents') WHERE name = 'source_type'",
            [],
            |row| row.get(0),
        ).unwrap();
        assert!(has_source_type, "documents should have source_type column");

        let has_source_uri: bool = db
            .conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM pragma_table_info('documents') WHERE name = 'source_uri'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(has_source_uri, "documents should have source_uri column");
    }

    #[test]
    fn test_migration_v3_to_v4() {
        let db = Database::open_in_memory().unwrap();

        db.conn
            .execute_batch(
                "CREATE TABLE collections (
                name TEXT PRIMARY KEY,
                path TEXT NOT NULL,
                pattern TEXT NOT NULL DEFAULT '**/*.md',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                provider_type TEXT NOT NULL DEFAULT 'file',
                provider_config TEXT
            );
            CREATE TABLE documents (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                collection TEXT NOT NULL,
                path TEXT NOT NULL,
                title TEXT NOT NULL,
                hash TEXT NOT NULL,
                created_at TEXT NOT NULL,
                modified_at TEXT NOT NULL,
                active INTEGER NOT NULL DEFAULT 1,
                source_type TEXT NOT NULL DEFAULT 'file',
                source_uri TEXT,
                UNIQUE(collection, path)
            );
            CREATE TABLE schema_version (version INTEGER PRIMARY KEY);
            INSERT INTO schema_version VALUES (3);",
            )
            .unwrap();

        assert_eq!(db.schema_version().unwrap(), Some(3));

        db.initialize().unwrap();

        assert_eq!(db.schema_version().unwrap(), Some(9));

        let metadata_columns = vec![
            "llm_summary",
            "llm_title",
            "llm_keywords",
            "llm_category",
            "llm_intent",
            "llm_concepts",
            "llm_difficulty",
            "llm_queries",
            "llm_metadata_generated_at",
            "llm_model",
        ];

        for column in metadata_columns {
            let has_column: bool = db
                .conn
                .query_row(
                    "SELECT COUNT(*) > 0 FROM pragma_table_info('documents') WHERE name = ?1",
                    params![column],
                    |row| row.get(0),
                )
                .unwrap();
            assert!(has_column, "documents should have {} column", column);
        }
    }

    #[test]
    fn test_migration_v4_to_v5() {
        let db = Database::open_in_memory().unwrap();

        db.conn
            .execute_batch(
                "CREATE TABLE collections (
                name TEXT PRIMARY KEY,
                path TEXT NOT NULL,
                pattern TEXT NOT NULL DEFAULT '**/*.md',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                provider_type TEXT NOT NULL DEFAULT 'file',
                provider_config TEXT
            );
            CREATE TABLE documents (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                collection TEXT NOT NULL,
                path TEXT NOT NULL,
                title TEXT NOT NULL,
                hash TEXT NOT NULL,
                created_at TEXT NOT NULL,
                modified_at TEXT NOT NULL,
                active INTEGER NOT NULL DEFAULT 1,
                source_type TEXT NOT NULL DEFAULT 'file',
                source_uri TEXT,
                llm_summary TEXT,
                llm_title TEXT,
                llm_keywords TEXT,
                llm_category TEXT,
                llm_intent TEXT,
                llm_concepts TEXT,
                llm_difficulty TEXT,
                llm_queries TEXT,
                llm_metadata_generated_at TEXT,
                llm_model TEXT,
                UNIQUE(collection, path)
            );
            CREATE TABLE content (
                hash TEXT PRIMARY KEY,
                doc TEXT NOT NULL,
                created_at TEXT NOT NULL
            );
            CREATE VIRTUAL TABLE documents_fts USING fts5(
                filepath,
                title,
                body,
                llm_summary,
                llm_title,
                llm_keywords,
                llm_intent,
                llm_concepts,
                tokenize='porter unicode61'
            );
            CREATE TABLE schema_version (version INTEGER PRIMARY KEY);
            INSERT INTO schema_version VALUES (4);",
            )
            .unwrap();

        assert_eq!(db.schema_version().unwrap(), Some(4));

        db.initialize().unwrap();

        assert_eq!(db.schema_version().unwrap(), Some(9));

        let has_user_metadata: bool = db
            .conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM pragma_table_info('documents') WHERE name = 'user_metadata'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(
            has_user_metadata,
            "documents should have user_metadata column"
        );

        let has_index: bool = db
            .conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type = 'index' AND name = 'idx_documents_user_metadata'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(has_index, "user_metadata should have index");
    }
}
