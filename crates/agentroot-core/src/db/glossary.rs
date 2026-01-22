//! Intelligent glossary for semantic concept discovery
//!
//! Manages extraction, storage, and retrieval of concepts from indexed content.
//! Concepts are linked to specific chunks for granular search and discovery.

use crate::error::Result;
use chrono::Utc;
use rusqlite::{params, Row};

use super::Database;

/// Concept information
#[derive(Debug, Clone)]
pub struct ConceptInfo {
    pub id: i64,
    pub term: String,
    pub normalized: String,
    pub chunk_count: usize,
}

/// Concept-chunk linkage information
#[derive(Debug, Clone)]
pub struct ConceptChunkInfo {
    pub concept_term: String,
    pub chunk_hash: String,
    pub document_hash: String,
    pub document_path: String,
    pub document_title: String,
    pub snippet: String,
}

impl Database {
    /// Insert or get existing concept
    /// Returns concept ID
    pub fn upsert_concept(&self, term: &str) -> Result<i64> {
        let normalized = normalize_term(term);
        let now = Utc::now().to_rfc3339();

        // Try to insert, if exists just return the existing id
        self.conn.execute(
            "INSERT OR IGNORE INTO concepts (term, normalized, created_at)
             VALUES (?1, ?2, ?3)",
            params![term, normalized, now],
        )?;

        // Get the id (either newly inserted or existing)
        let id: i64 = self.conn.query_row(
            "SELECT id FROM concepts WHERE term = ?1",
            params![term],
            |row| row.get(0),
        )?;

        Ok(id)
    }

    /// Link a concept to a chunk
    pub fn link_concept_to_chunk(
        &self,
        concept_id: i64,
        chunk_hash: &str,
        document_hash: &str,
        snippet: &str,
    ) -> Result<()> {
        let now = Utc::now().to_rfc3339();

        self.conn.execute(
            "INSERT OR IGNORE INTO concept_chunks 
             (concept_id, chunk_hash, document_hash, snippet, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![concept_id, chunk_hash, document_hash, snippet, now],
        )?;

        Ok(())
    }

    /// Search concepts using FTS
    pub fn search_concepts(&self, query: &str, limit: usize) -> Result<Vec<ConceptInfo>> {
        let normalized_query = normalize_term(query);

        let mut stmt = self.conn.prepare(
            "SELECT c.id, c.term, c.normalized, c.chunk_count
             FROM concepts c
             JOIN concepts_fts fts ON fts.rowid = c.id
             WHERE concepts_fts MATCH ?1
             ORDER BY rank
             LIMIT ?2",
        )?;

        let concepts = stmt
            .query_map(params![normalized_query, limit], map_concept_row)?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(concepts)
    }

    /// Get all chunks associated with a concept
    pub fn get_chunks_for_concept(&self, concept_id: i64) -> Result<Vec<ConceptChunkInfo>> {
        let mut stmt = self.conn.prepare(
            "SELECT 
                c.term,
                cc.chunk_hash,
                cc.document_hash,
                d.path,
                d.title,
                cc.snippet
             FROM concept_chunks cc
             JOIN concepts c ON c.id = cc.concept_id
             JOIN documents d ON d.hash = cc.document_hash
             WHERE cc.concept_id = ?1 AND d.active = 1",
        )?;

        let chunk_infos = stmt
            .query_map(params![concept_id], map_concept_chunk_row)?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(chunk_infos)
    }

    /// Get all concepts for a document
    pub fn get_concepts_for_document(&self, doc_hash: &str) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT DISTINCT c.term
             FROM concepts c
             JOIN concept_chunks cc ON cc.concept_id = c.id
             WHERE cc.document_hash = ?1
             ORDER BY c.term",
        )?;

        let concepts = stmt
            .query_map(params![doc_hash], |row| row.get::<_, String>(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(concepts)
    }

    /// Update concept statistics (chunk_count)
    pub fn update_concept_stats(&self, concept_id: i64) -> Result<()> {
        self.conn.execute(
            "UPDATE concepts
             SET chunk_count = (
                 SELECT COUNT(DISTINCT chunk_hash)
                 FROM concept_chunks
                 WHERE concept_id = ?1
             )
             WHERE id = ?1",
            params![concept_id],
        )?;

        Ok(())
    }

    /// Clean up orphaned concepts (no associated chunks)
    pub fn cleanup_orphaned_concepts(&self) -> Result<usize> {
        let deleted = self.conn.execute(
            "DELETE FROM concepts
             WHERE id NOT IN (
                 SELECT DISTINCT concept_id FROM concept_chunks
             )",
            [],
        )?;

        Ok(deleted)
    }

    /// Delete all concept links for a document
    pub fn delete_concepts_for_document(&self, doc_hash: &str) -> Result<usize> {
        let deleted = self.conn.execute(
            "DELETE FROM concept_chunks WHERE document_hash = ?1",
            params![doc_hash],
        )?;

        Ok(deleted)
    }

    /// Get concept count statistics
    pub fn get_concept_stats(&self) -> Result<(usize, usize)> {
        let total_concepts: usize =
            self.conn
                .query_row("SELECT COUNT(*) FROM concepts", [], |row| row.get(0))?;

        let total_links: usize =
            self.conn
                .query_row("SELECT COUNT(*) FROM concept_chunks", [], |row| row.get(0))?;

        Ok((total_concepts, total_links))
    }
}

/// Normalize term for search
/// Converts to lowercase and replaces spaces with underscores
fn normalize_term(term: &str) -> String {
    term.to_lowercase().replace(' ', "_")
}

/// Map database row to ConceptInfo
fn map_concept_row(row: &Row) -> rusqlite::Result<ConceptInfo> {
    Ok(ConceptInfo {
        id: row.get(0)?,
        term: row.get(1)?,
        normalized: row.get(2)?,
        chunk_count: row.get(3)?,
    })
}

/// Map database row to ConceptChunkInfo
fn map_concept_chunk_row(row: &Row) -> rusqlite::Result<ConceptChunkInfo> {
    Ok(ConceptChunkInfo {
        concept_term: row.get(0)?,
        chunk_hash: row.get(1)?,
        document_hash: row.get(2)?,
        document_path: row.get(3)?,
        document_title: row.get(4)?,
        snippet: row.get(5)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_term() {
        assert_eq!(normalize_term("Machine Learning"), "machine_learning");
        assert_eq!(normalize_term("Rust"), "rust");
        assert_eq!(
            normalize_term("Neural Network Architecture"),
            "neural_network_architecture"
        );
    }

    #[test]
    fn test_upsert_concept() {
        let db = Database::open_in_memory().unwrap();
        db.initialize().unwrap();

        // Insert new concept
        let id1 = db.upsert_concept("machine learning").unwrap();
        assert!(id1 > 0);

        // Upsert same concept should return same id
        let id2 = db.upsert_concept("machine learning").unwrap();
        assert_eq!(id1, id2);

        // Different concept gets different id
        let id3 = db.upsert_concept("neural network").unwrap();
        assert_ne!(id1, id3);
    }

    #[test]
    fn test_link_concept_to_chunk() {
        let db = Database::open_in_memory().unwrap();
        db.initialize().unwrap();

        let concept_id = db.upsert_concept("kubernetes").unwrap();

        // Link concept to chunk
        db.link_concept_to_chunk(
            concept_id,
            "chunk_abc123",
            "doc_hash_456",
            "discusses kubernetes orchestration",
        )
        .unwrap();

        // Linking same concept to same chunk should be idempotent
        db.link_concept_to_chunk(
            concept_id,
            "chunk_abc123",
            "doc_hash_456",
            "discusses kubernetes orchestration",
        )
        .unwrap();
    }

    #[test]
    fn test_update_concept_stats() {
        let db = Database::open_in_memory().unwrap();
        db.initialize().unwrap();

        let concept_id = db.upsert_concept("rust programming").unwrap();

        // Link to multiple chunks
        db.link_concept_to_chunk(concept_id, "chunk1", "doc1", "rust snippet 1")
            .unwrap();
        db.link_concept_to_chunk(concept_id, "chunk2", "doc1", "rust snippet 2")
            .unwrap();
        db.link_concept_to_chunk(concept_id, "chunk3", "doc2", "rust snippet 3")
            .unwrap();

        // Update stats
        db.update_concept_stats(concept_id).unwrap();

        // Verify count
        let info: ConceptInfo = db
            .conn
            .query_row(
                "SELECT id, term, normalized, chunk_count FROM concepts WHERE id = ?1",
                params![concept_id],
                map_concept_row,
            )
            .unwrap();

        assert_eq!(info.chunk_count, 3);
    }

    #[test]
    fn test_cleanup_orphaned_concepts() {
        let db = Database::open_in_memory().unwrap();
        db.initialize().unwrap();

        // Create concept with links
        let id1 = db.upsert_concept("concept_with_links").unwrap();
        db.link_concept_to_chunk(id1, "chunk1", "doc1", "snippet")
            .unwrap();

        // Create orphaned concept (no links)
        let id2 = db.upsert_concept("orphaned_concept").unwrap();

        // Cleanup should remove only orphaned
        let deleted = db.cleanup_orphaned_concepts().unwrap();
        assert_eq!(deleted, 1);

        // Verify orphaned concept is gone
        let exists: bool = db
            .conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM concepts WHERE id = ?1",
                params![id2],
                |row| row.get(0),
            )
            .unwrap();
        assert!(!exists);

        // Verify linked concept still exists
        let exists: bool = db
            .conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM concepts WHERE id = ?1",
                params![id1],
                |row| row.get(0),
            )
            .unwrap();
        assert!(exists);
    }

    #[test]
    fn test_get_concept_stats() {
        let db = Database::open_in_memory().unwrap();
        db.initialize().unwrap();

        let id1 = db.upsert_concept("concept1").unwrap();
        let id2 = db.upsert_concept("concept2").unwrap();

        db.link_concept_to_chunk(id1, "chunk1", "doc1", "snippet1")
            .unwrap();
        db.link_concept_to_chunk(id1, "chunk2", "doc1", "snippet2")
            .unwrap();
        db.link_concept_to_chunk(id2, "chunk3", "doc2", "snippet3")
            .unwrap();

        let (total_concepts, total_links) = db.get_concept_stats().unwrap();
        assert_eq!(total_concepts, 2);
        assert_eq!(total_links, 3);
    }
}
