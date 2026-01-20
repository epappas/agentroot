//! Database schema and initialization

use crate::error::Result;
use rusqlite::{params, Connection};
use std::path::Path;

/// Main database handle
pub struct Database {
    pub(crate) conn: Connection,
}

const SCHEMA_VERSION: i32 = 5;

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
END;

-- Sync FTS on update: handle activation/deactivation/content change
CREATE TRIGGER IF NOT EXISTS documents_au
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

        assert_eq!(db.schema_version().unwrap(), Some(5));

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

        assert_eq!(db.schema_version().unwrap(), Some(5));

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

        assert_eq!(db.schema_version().unwrap(), Some(5));

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
