//! Chunk storage and retrieval operations
//!
//! Manages individual code/text chunks with LLM-generated metadata,
//! labels, and full-text search indexing.

use crate::db::Database;
use crate::error::Result;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Full chunk information including metadata and labels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkInfo {
    pub hash: String,
    pub document_hash: String,
    pub seq: i32,
    pub pos: i32,
    pub content: String,
    pub chunk_type: Option<String>,
    pub breadcrumb: Option<String>,
    pub start_line: i32,
    pub end_line: i32,
    pub language: Option<String>,
    pub llm_summary: Option<String>,
    pub llm_purpose: Option<String>,
    pub llm_concepts: Vec<String>,
    pub llm_labels: HashMap<String, String>,
    pub llm_related_to: Vec<String>,
    pub llm_model: Option<String>,
    pub llm_generated_at: Option<String>,
    pub created_at: String,
}

impl Database {
    /// Insert a new chunk with metadata
    pub fn insert_chunk(
        &self,
        hash: &str,
        document_hash: &str,
        seq: i32,
        pos: i32,
        content: &str,
        chunk_type: Option<&str>,
        breadcrumb: Option<&str>,
        start_line: i32,
        end_line: i32,
        language: Option<&str>,
        llm_summary: Option<&str>,
        llm_purpose: Option<&str>,
        llm_concepts: &[String],
        llm_labels: &HashMap<String, String>,
        llm_related_to: &[String],
        llm_model: Option<&str>,
        llm_generated_at: Option<&str>,
        created_at: &str,
    ) -> Result<()> {
        // Serialize JSON fields
        let concepts_json = serde_json::to_string(llm_concepts)?;
        let labels_json = serde_json::to_string(llm_labels)?;
        let related_json = serde_json::to_string(llm_related_to)?;

        // Insert chunk
        self.conn.execute(
            "INSERT INTO chunks (
                hash, document_hash, seq, pos, content,
                chunk_type, breadcrumb, start_line, end_line, language,
                llm_summary, llm_purpose, llm_concepts, llm_labels, llm_related_to,
                llm_model, llm_generated_at, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)
            ON CONFLICT(hash) DO UPDATE SET
                llm_summary = excluded.llm_summary,
                llm_purpose = excluded.llm_purpose,
                llm_concepts = excluded.llm_concepts,
                llm_labels = excluded.llm_labels,
                llm_related_to = excluded.llm_related_to,
                llm_model = excluded.llm_model,
                llm_generated_at = excluded.llm_generated_at",
            params![
                hash,
                document_hash,
                seq,
                pos,
                content,
                chunk_type,
                breadcrumb,
                start_line,
                end_line,
                language,
                llm_summary,
                llm_purpose,
                concepts_json,
                labels_json,
                related_json,
                llm_model,
                llm_generated_at,
                created_at
            ],
        )?;

        // Insert labels into normalized table
        // First delete existing labels for this chunk
        self.conn.execute(
            "DELETE FROM chunk_labels WHERE chunk_hash = ?1",
            params![hash],
        )?;

        // Then insert new labels
        for (key, value) in llm_labels {
            self.conn.execute(
                "INSERT INTO chunk_labels (chunk_hash, key, value) VALUES (?1, ?2, ?3)",
                params![hash, key, value],
            )?;
        }

        Ok(())
    }

    /// Get a single chunk by hash
    pub fn get_chunk(&self, hash: &str) -> Result<Option<ChunkInfo>> {
        let mut stmt = self.conn.prepare(
            "SELECT hash, document_hash, seq, pos, content,
                    chunk_type, breadcrumb, start_line, end_line, language,
                    llm_summary, llm_purpose, llm_concepts, llm_labels, llm_related_to,
                    llm_model, llm_generated_at, created_at
             FROM chunks WHERE hash = ?1",
        )?;

        let result = stmt.query_row(params![hash], |row| {
            let concepts_json: String = row.get(12)?;
            let labels_json: String = row.get(13)?;
            let related_json: String = row.get(14)?;

            let concepts: Vec<String> = serde_json::from_str(&concepts_json).unwrap_or_default();
            let labels: HashMap<String, String> =
                serde_json::from_str(&labels_json).unwrap_or_default();
            let related: Vec<String> = serde_json::from_str(&related_json).unwrap_or_default();

            Ok(ChunkInfo {
                hash: row.get(0)?,
                document_hash: row.get(1)?,
                seq: row.get(2)?,
                pos: row.get(3)?,
                content: row.get(4)?,
                chunk_type: row.get(5)?,
                breadcrumb: row.get(6)?,
                start_line: row.get(7)?,
                end_line: row.get(8)?,
                language: row.get(9)?,
                llm_summary: row.get(10)?,
                llm_purpose: row.get(11)?,
                llm_concepts: concepts,
                llm_labels: labels,
                llm_related_to: related,
                llm_model: row.get(15)?,
                llm_generated_at: row.get(16)?,
                created_at: row.get(17)?,
            })
        });

        match result {
            Ok(chunk) => Ok(Some(chunk)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Get all chunks for a document
    pub fn get_chunks_for_document(&self, document_hash: &str) -> Result<Vec<ChunkInfo>> {
        let mut stmt = self.conn.prepare(
            "SELECT hash, document_hash, seq, pos, content,
                    chunk_type, breadcrumb, start_line, end_line, language,
                    llm_summary, llm_purpose, llm_concepts, llm_labels, llm_related_to,
                    llm_model, llm_generated_at, created_at
             FROM chunks WHERE document_hash = ?1 ORDER BY seq",
        )?;

        let chunks = stmt
            .query_map(params![document_hash], |row| {
                let concepts_json: String = row.get(12)?;
                let labels_json: String = row.get(13)?;
                let related_json: String = row.get(14)?;

                let concepts: Vec<String> =
                    serde_json::from_str(&concepts_json).unwrap_or_default();
                let labels: HashMap<String, String> =
                    serde_json::from_str(&labels_json).unwrap_or_default();
                let related: Vec<String> = serde_json::from_str(&related_json).unwrap_or_default();

                Ok(ChunkInfo {
                    hash: row.get(0)?,
                    document_hash: row.get(1)?,
                    seq: row.get(2)?,
                    pos: row.get(3)?,
                    content: row.get(4)?,
                    chunk_type: row.get(5)?,
                    breadcrumb: row.get(6)?,
                    start_line: row.get(7)?,
                    end_line: row.get(8)?,
                    language: row.get(9)?,
                    llm_summary: row.get(10)?,
                    llm_purpose: row.get(11)?,
                    llm_concepts: concepts,
                    llm_labels: labels,
                    llm_related_to: related,
                    llm_model: row.get(15)?,
                    llm_generated_at: row.get(16)?,
                    created_at: row.get(17)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(chunks)
    }

    /// Search chunks using full-text search
    pub fn search_chunks_fts(&self, query: &str, limit: usize) -> Result<Vec<ChunkInfo>> {
        let mut stmt = self.conn.prepare(
            "SELECT c.hash, c.document_hash, c.seq, c.pos, c.content,
                    c.chunk_type, c.breadcrumb, c.start_line, c.end_line, c.language,
                    c.llm_summary, c.llm_purpose, c.llm_concepts, c.llm_labels, c.llm_related_to,
                    c.llm_model, c.llm_generated_at, c.created_at,
                    cf.rank
             FROM chunks_fts cf
             JOIN chunks c ON cf.rowid = c.rowid
             WHERE chunks_fts MATCH ?1
             ORDER BY cf.rank
             LIMIT ?2",
        )?;

        let chunks = stmt
            .query_map(params![query, limit as i64], |row| {
                let concepts_json: String = row.get(12)?;
                let labels_json: String = row.get(13)?;
                let related_json: String = row.get(14)?;

                let concepts: Vec<String> =
                    serde_json::from_str(&concepts_json).unwrap_or_default();
                let labels: HashMap<String, String> =
                    serde_json::from_str(&labels_json).unwrap_or_default();
                let related: Vec<String> = serde_json::from_str(&related_json).unwrap_or_default();

                Ok(ChunkInfo {
                    hash: row.get(0)?,
                    document_hash: row.get(1)?,
                    seq: row.get(2)?,
                    pos: row.get(3)?,
                    content: row.get(4)?,
                    chunk_type: row.get(5)?,
                    breadcrumb: row.get(6)?,
                    start_line: row.get(7)?,
                    end_line: row.get(8)?,
                    language: row.get(9)?,
                    llm_summary: row.get(10)?,
                    llm_purpose: row.get(11)?,
                    llm_concepts: concepts,
                    llm_labels: labels,
                    llm_related_to: related,
                    llm_model: row.get(15)?,
                    llm_generated_at: row.get(16)?,
                    created_at: row.get(17)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(chunks)
    }

    /// Search chunks by label
    pub fn search_chunks_by_label(&self, key: &str, value: &str) -> Result<Vec<ChunkInfo>> {
        let mut stmt = self.conn.prepare(
            "SELECT c.hash, c.document_hash, c.seq, c.pos, c.content,
                    c.chunk_type, c.breadcrumb, c.start_line, c.end_line, c.language,
                    c.llm_summary, c.llm_purpose, c.llm_concepts, c.llm_labels, c.llm_related_to,
                    c.llm_model, c.llm_generated_at, c.created_at
             FROM chunks c
             JOIN chunk_labels cl ON c.hash = cl.chunk_hash
             WHERE cl.key = ?1 AND cl.value = ?2
             ORDER BY c.seq",
        )?;

        let chunks = stmt
            .query_map(params![key, value], |row| {
                let concepts_json: String = row.get(12)?;
                let labels_json: String = row.get(13)?;
                let related_json: String = row.get(14)?;

                let concepts: Vec<String> =
                    serde_json::from_str(&concepts_json).unwrap_or_default();
                let labels: HashMap<String, String> =
                    serde_json::from_str(&labels_json).unwrap_or_default();
                let related: Vec<String> = serde_json::from_str(&related_json).unwrap_or_default();

                Ok(ChunkInfo {
                    hash: row.get(0)?,
                    document_hash: row.get(1)?,
                    seq: row.get(2)?,
                    pos: row.get(3)?,
                    content: row.get(4)?,
                    chunk_type: row.get(5)?,
                    breadcrumb: row.get(6)?,
                    start_line: row.get(7)?,
                    end_line: row.get(8)?,
                    language: row.get(9)?,
                    llm_summary: row.get(10)?,
                    llm_purpose: row.get(11)?,
                    llm_concepts: concepts,
                    llm_labels: labels,
                    llm_related_to: related,
                    llm_model: row.get(15)?,
                    llm_generated_at: row.get(16)?,
                    created_at: row.get(17)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(chunks)
    }

    /// Delete all chunks for a document
    pub fn delete_chunks_for_document(&self, document_hash: &str) -> Result<()> {
        self.conn.execute(
            "DELETE FROM chunk_labels WHERE chunk_hash IN 
             (SELECT hash FROM chunks WHERE document_hash = ?1)",
            params![document_hash],
        )?;

        self.conn.execute(
            "DELETE FROM chunks WHERE document_hash = ?1",
            params![document_hash],
        )?;

        Ok(())
    }

    /// Get chunk labels for a specific chunk
    pub fn get_chunk_labels(&self, chunk_hash: &str) -> Result<HashMap<String, String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT key, value FROM chunk_labels WHERE chunk_hash = ?1")?;

        let labels = stmt
            .query_map(params![chunk_hash], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?
            .collect::<std::result::Result<HashMap<_, _>, _>>()?;

        Ok(labels)
    }

    /// Get surrounding chunks (previous and next)
    pub fn get_surrounding_chunks(
        &self,
        chunk_hash: &str,
    ) -> Result<(Option<ChunkInfo>, Option<ChunkInfo>)> {
        let chunk = self.get_chunk(chunk_hash)?;
        if chunk.is_none() {
            return Ok((None, None));
        }

        let chunk = chunk.unwrap();

        let prev = {
            let mut stmt = self.conn.prepare(
                "SELECT hash, document_hash, seq, pos, content,
                        chunk_type, breadcrumb, start_line, end_line, language,
                        llm_summary, llm_purpose, llm_concepts, llm_labels, llm_related_to,
                        llm_model, llm_generated_at, created_at
                 FROM chunks 
                 WHERE document_hash = ?1 AND seq < ?2
                 ORDER BY seq DESC LIMIT 1",
            )?;

            stmt.query_row(params![chunk.document_hash, chunk.seq], |row| {
                let concepts_json: String = row.get(12)?;
                let labels_json: String = row.get(13)?;
                let related_json: String = row.get(14)?;

                let concepts: Vec<String> =
                    serde_json::from_str(&concepts_json).unwrap_or_default();
                let labels: HashMap<String, String> =
                    serde_json::from_str(&labels_json).unwrap_or_default();
                let related: Vec<String> = serde_json::from_str(&related_json).unwrap_or_default();

                Ok(ChunkInfo {
                    hash: row.get(0)?,
                    document_hash: row.get(1)?,
                    seq: row.get(2)?,
                    pos: row.get(3)?,
                    content: row.get(4)?,
                    chunk_type: row.get(5)?,
                    breadcrumb: row.get(6)?,
                    start_line: row.get(7)?,
                    end_line: row.get(8)?,
                    language: row.get(9)?,
                    llm_summary: row.get(10)?,
                    llm_purpose: row.get(11)?,
                    llm_concepts: concepts,
                    llm_labels: labels,
                    llm_related_to: related,
                    llm_model: row.get(15)?,
                    llm_generated_at: row.get(16)?,
                    created_at: row.get(17)?,
                })
            })
            .ok()
        };

        let next = {
            let mut stmt = self.conn.prepare(
                "SELECT hash, document_hash, seq, pos, content,
                        chunk_type, breadcrumb, start_line, end_line, language,
                        llm_summary, llm_purpose, llm_concepts, llm_labels, llm_related_to,
                        llm_model, llm_generated_at, created_at
                 FROM chunks 
                 WHERE document_hash = ?1 AND seq > ?2
                 ORDER BY seq ASC LIMIT 1",
            )?;

            stmt.query_row(params![chunk.document_hash, chunk.seq], |row| {
                let concepts_json: String = row.get(12)?;
                let labels_json: String = row.get(13)?;
                let related_json: String = row.get(14)?;

                let concepts: Vec<String> =
                    serde_json::from_str(&concepts_json).unwrap_or_default();
                let labels: HashMap<String, String> =
                    serde_json::from_str(&labels_json).unwrap_or_default();
                let related: Vec<String> = serde_json::from_str(&related_json).unwrap_or_default();

                Ok(ChunkInfo {
                    hash: row.get(0)?,
                    document_hash: row.get(1)?,
                    seq: row.get(2)?,
                    pos: row.get(3)?,
                    content: row.get(4)?,
                    chunk_type: row.get(5)?,
                    breadcrumb: row.get(6)?,
                    start_line: row.get(7)?,
                    end_line: row.get(8)?,
                    language: row.get(9)?,
                    llm_summary: row.get(10)?,
                    llm_purpose: row.get(11)?,
                    llm_concepts: concepts,
                    llm_labels: labels,
                    llm_related_to: related,
                    llm_model: row.get(15)?,
                    llm_generated_at: row.get(16)?,
                    created_at: row.get(17)?,
                })
            })
            .ok()
        };

        Ok((prev, next))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_and_get_chunk() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db = Database::open(&db_path).unwrap();
        db.initialize().unwrap();

        use crate::db::content::hash_content;

        let doc_content = "test document content";
        let doc_hash = hash_content(doc_content);
        db.insert_content(&doc_hash, doc_content).unwrap();

        let mut labels = HashMap::new();
        labels.insert("operation".to_string(), "validation".to_string());
        labels.insert("entity".to_string(), "email".to_string());

        let concepts = vec!["validation".to_string(), "regex".to_string()];
        let related: Vec<String> = vec![];

        db.insert_chunk(
            "test123",
            &doc_hash,
            0,
            0,
            "fn validate() {}",
            Some("Function"),
            Some("validate"),
            10,
            15,
            Some("rust"),
            Some("Validates input"),
            Some("Ensure data quality"),
            &concepts,
            &labels,
            &related,
            Some("test-model"),
            Some("2024-01-01T00:00:00Z"),
            "2024-01-01T00:00:00Z",
        )
        .unwrap();

        let chunk = db.get_chunk("test123").unwrap().unwrap();
        assert_eq!(chunk.hash, "test123");
        assert_eq!(chunk.document_hash, doc_hash);
        assert_eq!(chunk.content, "fn validate() {}");
        assert_eq!(chunk.llm_summary, Some("Validates input".to_string()));
        assert_eq!(chunk.llm_concepts.len(), 2);
        assert_eq!(chunk.llm_labels.len(), 2);
    }

    #[test]
    fn test_search_chunks_by_label() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db = Database::open(&db_path).unwrap();
        db.initialize().unwrap();

        use crate::db::content::hash_content;

        let doc_content = "test document content";
        let doc_hash = hash_content(doc_content);
        db.insert_content(&doc_hash, doc_content).unwrap();

        let mut labels1 = HashMap::new();
        labels1.insert("layer".to_string(), "service".to_string());

        let mut labels2 = HashMap::new();
        labels2.insert("layer".to_string(), "controller".to_string());

        db.insert_chunk(
            "chunk1",
            &doc_hash,
            0,
            0,
            "service code",
            None,
            None,
            1,
            10,
            None,
            None,
            None,
            &vec![],
            &labels1,
            &vec![],
            None,
            None,
            "2024-01-01T00:00:00Z",
        )
        .unwrap();

        db.insert_chunk(
            "chunk2",
            &doc_hash,
            1,
            100,
            "controller code",
            None,
            None,
            11,
            20,
            None,
            None,
            None,
            &vec![],
            &labels2,
            &vec![],
            None,
            None,
            "2024-01-01T00:00:00Z",
        )
        .unwrap();

        let results = db.search_chunks_by_label("layer", "service").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].hash, "chunk1");
    }

    #[test]
    fn test_get_surrounding_chunks() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db = Database::open(&db_path).unwrap();
        db.initialize().unwrap();

        use crate::db::content::hash_content;

        let doc_content = "test document content";
        let doc_hash = hash_content(doc_content);
        db.insert_content(&doc_hash, doc_content).unwrap();

        let empty_labels = HashMap::new();
        let empty_vec = vec![];

        db.insert_chunk(
            "chunk1",
            &doc_hash,
            0,
            0,
            "first",
            None,
            None,
            1,
            5,
            None,
            None,
            None,
            &empty_vec,
            &empty_labels,
            &empty_vec,
            None,
            None,
            "2024-01-01T00:00:00Z",
        )
        .unwrap();

        db.insert_chunk(
            "chunk2",
            &doc_hash,
            1,
            100,
            "second",
            None,
            None,
            6,
            10,
            None,
            None,
            None,
            &empty_vec,
            &empty_labels,
            &empty_vec,
            None,
            None,
            "2024-01-01T00:00:00Z",
        )
        .unwrap();

        db.insert_chunk(
            "chunk3",
            &doc_hash,
            2,
            200,
            "third",
            None,
            None,
            11,
            15,
            None,
            None,
            None,
            &empty_vec,
            &empty_labels,
            &empty_vec,
            None,
            None,
            "2024-01-01T00:00:00Z",
        )
        .unwrap();

        let (prev, next) = db.get_surrounding_chunks("chunk2").unwrap();
        assert!(prev.is_some());
        assert!(next.is_some());
        assert_eq!(prev.unwrap().hash, "chunk1");
        assert_eq!(next.unwrap().hash, "chunk3");
    }
}
