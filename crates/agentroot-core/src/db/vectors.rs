//! Vector storage operations
//!
//! Stores embeddings as BLOBs and computes cosine similarity in Rust.

use super::Database;
use crate::error::Result;
use chrono::Utc;
use rusqlite::params;

/// Result of looking up a cached embedding
#[derive(Debug, Clone)]
pub enum CacheLookupResult {
    /// Cache hit with the embedding
    Hit(Vec<f32>),
    /// Cache miss - need to compute
    Miss,
    /// Model dimensions changed - need to recompute
    ModelMismatch,
}

impl Database {
    /// Ensure vector storage table exists
    pub fn ensure_vec_table(&self, _dimensions: usize) -> Result<()> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS embeddings (
                hash_seq TEXT PRIMARY KEY,
                embedding BLOB NOT NULL
            )",
            [],
        )?;
        // Note: No index needed on hash_seq - SQLite automatically indexes PRIMARY KEY
        Ok(())
    }

    /// Insert embedding for a document chunk
    pub fn insert_embedding(
        &self,
        hash: &str,
        seq: u32,
        pos: usize,
        model: &str,
        embedding: &[f32],
    ) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        let hash_seq = format!("{}_{}", hash, seq);
        let embedding_bytes = embedding_to_bytes(embedding);

        self.conn.execute("BEGIN IMMEDIATE", [])?;
        let result = (|| {
            self.conn.execute(
                "INSERT OR REPLACE INTO content_vectors (hash, seq, pos, model, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![hash, seq, pos, model, now],
            )?;
            self.conn.execute(
                "INSERT OR REPLACE INTO embeddings (hash_seq, embedding) VALUES (?1, ?2)",
                params![hash_seq, embedding_bytes],
            )?;
            Ok(())
        })();

        if result.is_ok() {
            self.conn.execute("COMMIT", [])?;
        } else {
            let _ = self.conn.execute("ROLLBACK", []);
        }
        result
    }

    /// Check if vector index exists and has data
    pub fn has_vector_index(&self) -> bool {
        self.conn
            .query_row("SELECT COUNT(*) FROM content_vectors", [], |row| {
                row.get::<_, i64>(0)
            })
            .map(|count| count > 0)
            .unwrap_or(false)
    }

    /// Get all embeddings for similarity search
    pub fn get_all_embeddings(&self) -> Result<Vec<(String, Vec<f32>)>> {
        let mut stmt = self
            .conn
            .prepare("SELECT hash_seq, embedding FROM embeddings")?;

        let results = stmt
            .query_map([], |row| {
                let hash_seq: String = row.get(0)?;
                let embedding_bytes: Vec<u8> = row.get(1)?;
                let embedding = bytes_to_embedding(&embedding_bytes);
                Ok((hash_seq, embedding))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(results)
    }

    /// Get embeddings for specific hashes (for filtered search)
    pub fn get_embeddings_for_collection(
        &self,
        collection: &str,
    ) -> Result<Vec<(String, Vec<f32>)>> {
        let mut stmt = self.conn.prepare(
            "SELECT e.hash_seq, e.embedding
             FROM embeddings e
             JOIN content_vectors cv ON e.hash_seq = cv.hash || '_' || cv.seq
             JOIN documents d ON d.hash = cv.hash AND d.active = 1
             WHERE d.collection = ?1",
        )?;

        let results = stmt
            .query_map(params![collection], |row| {
                let hash_seq: String = row.get(0)?;
                let embedding_bytes: Vec<u8> = row.get(1)?;
                let embedding = bytes_to_embedding(&embedding_bytes);
                Ok((hash_seq, embedding))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(results)
    }

    /// Get hashes that need embedding
    pub fn get_hashes_needing_embedding(&self) -> Result<Vec<(String, String)>> {
        let mut stmt = self.conn.prepare(
            "SELECT c.hash, c.doc FROM content c
             JOIN documents d ON d.hash = c.hash AND d.active = 1
             WHERE c.hash NOT IN (SELECT DISTINCT hash FROM content_vectors)",
        )?;

        let results = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(results)
    }

    /// Count hashes needing embedding
    pub fn count_hashes_needing_embedding(&self) -> Result<usize> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(DISTINCT c.hash) FROM content c
             JOIN documents d ON d.hash = c.hash AND d.active = 1
             WHERE c.hash NOT IN (SELECT DISTINCT hash FROM content_vectors)",
            [],
            |row| row.get(0),
        )?;
        Ok(count as usize)
    }

    /// Delete embeddings for a hash
    pub fn delete_embeddings(&self, hash: &str) -> Result<usize> {
        let pattern = format!("{}_*", hash);

        self.conn.execute("BEGIN IMMEDIATE", [])?;
        let result = (|| {
            self.conn
                .execute("DELETE FROM content_vectors WHERE hash = ?1", params![hash])?;
            // Use GLOB instead of LIKE to avoid issues with special characters.
            // GLOB uses * and ? as wildcards, which won't appear in SHA-256 hex hashes.
            let rows = self.conn.execute(
                "DELETE FROM embeddings WHERE hash_seq GLOB ?1",
                params![pattern],
            )?;
            Ok(rows)
        })();

        if result.is_ok() {
            self.conn.execute("COMMIT", [])?;
        } else {
            let _ = self.conn.execute("ROLLBACK", []);
        }
        result
    }

    /// Get all hashes for embedding (for force re-embedding)
    pub fn get_all_hashes_for_embedding(&self) -> Result<Vec<(String, String)>> {
        let mut stmt = self.conn.prepare(
            "SELECT c.hash, c.doc FROM content c
             JOIN documents d ON d.hash = c.hash AND d.active = 1",
        )?;

        let results = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(results)
    }

    /// Check if model dimensions are compatible with expected dimensions
    pub fn check_model_compatibility(&self, model: &str, expected_dims: usize) -> Result<bool> {
        match self.get_model_dimensions(model)? {
            Some(stored_dims) => Ok(stored_dims == expected_dims),
            None => Ok(true), // No stored model = compatible (will be registered)
        }
    }

    /// Look up a cached embedding by chunk hash (performs dimension check)
    pub fn get_cached_embedding(
        &self,
        chunk_hash: &str,
        model: &str,
        expected_dims: usize,
    ) -> Result<CacheLookupResult> {
        if !self.check_model_compatibility(model, expected_dims)? {
            return Ok(CacheLookupResult::ModelMismatch);
        }
        self.get_cached_embedding_fast(chunk_hash, model)
    }

    /// Look up a cached embedding by chunk hash (skips dimension check - caller must verify compatibility)
    pub fn get_cached_embedding_fast(
        &self,
        chunk_hash: &str,
        model: &str,
    ) -> Result<CacheLookupResult> {
        let result = self.conn.query_row(
            "SELECT embedding FROM chunk_embeddings WHERE chunk_hash = ?1 AND model = ?2",
            params![chunk_hash, model],
            |row| {
                let bytes: Vec<u8> = row.get(0)?;
                Ok(bytes_to_embedding(&bytes))
            },
        );

        match result {
            Ok(embedding) => Ok(CacheLookupResult::Hit(embedding)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(CacheLookupResult::Miss),
            Err(e) => Err(e.into()),
        }
    }

    /// Insert a chunk embedding with cache support
    pub fn insert_chunk_embedding(
        &self,
        doc_hash: &str,
        seq: u32,
        pos: usize,
        chunk_hash: &str,
        model: &str,
        embedding: &[f32],
    ) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        let hash_seq = format!("{}_{}", doc_hash, seq);
        let embedding_bytes = embedding_to_bytes(embedding);

        self.conn.execute("BEGIN IMMEDIATE", [])?;
        let result = (|| {
            self.conn.execute(
                "INSERT OR REPLACE INTO content_vectors (hash, seq, pos, model, chunk_hash, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![doc_hash, seq, pos, model, chunk_hash, now],
            )?;
            self.conn.execute(
                "INSERT OR REPLACE INTO embeddings (hash_seq, embedding) VALUES (?1, ?2)",
                params![hash_seq, embedding_bytes],
            )?;
            self.conn.execute(
                "INSERT OR REPLACE INTO chunk_embeddings (chunk_hash, model, embedding, created_at)
                 VALUES (?1, ?2, ?3, ?4)",
                params![chunk_hash, model, &embedding_bytes, now],
            )?;
            Ok(())
        })();

        if result.is_ok() {
            self.conn.execute("COMMIT", [])?;
        } else {
            let _ = self.conn.execute("ROLLBACK", []);
        }
        result
    }

    /// Get chunk hashes for a document
    pub fn get_chunk_hashes_for_doc(&self, doc_hash: &str) -> Result<Vec<(u32, String)>> {
        let mut stmt = self.conn.prepare(
            "SELECT seq, chunk_hash FROM content_vectors WHERE hash = ?1 AND chunk_hash IS NOT NULL"
        )?;

        let results = stmt
            .query_map(params![doc_hash], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(results)
    }

    /// Clean up orphaned chunk embeddings (not referenced by any document)
    pub fn cleanup_orphaned_chunk_embeddings(&self) -> Result<usize> {
        let count = self.conn.execute(
            "DELETE FROM chunk_embeddings WHERE chunk_hash NOT IN (
                SELECT DISTINCT chunk_hash FROM content_vectors WHERE chunk_hash IS NOT NULL
            )",
            [],
        )?;
        Ok(count)
    }

    /// Register model with its dimensions
    pub fn register_model(&self, model: &str, dimensions: usize) -> Result<()> {
        let now = Utc::now().to_rfc3339();

        self.conn.execute(
            "INSERT INTO model_metadata (model, dimensions, created_at, last_used_at)
             VALUES (?1, ?2, ?3, ?3)
             ON CONFLICT(model) DO UPDATE SET last_used_at = ?3",
            params![model, dimensions as i64, now],
        )?;

        Ok(())
    }

    /// Get stored model dimensions
    pub fn get_model_dimensions(&self, model: &str) -> Result<Option<usize>> {
        let result = self.conn.query_row(
            "SELECT dimensions FROM model_metadata WHERE model = ?1",
            params![model],
            |row| row.get::<_, i64>(0),
        );

        match result {
            Ok(dims) => Ok(Some(dims as usize)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Count cached chunk embeddings
    pub fn count_cached_embeddings(&self, model: &str) -> Result<usize> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM chunk_embeddings WHERE model = ?1",
            params![model],
            |row| row.get(0),
        )?;
        Ok(count as usize)
    }
}

/// Convert f32 embedding to bytes (little-endian)
pub fn embedding_to_bytes(embedding: &[f32]) -> Vec<u8> {
    embedding.iter().flat_map(|f| f.to_le_bytes()).collect()
}

/// Convert bytes to f32 embedding
pub fn bytes_to_embedding(bytes: &[u8]) -> Vec<f32> {
    bytes
        .chunks_exact(4)
        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect()
}

/// Compute cosine similarity between two embeddings
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }

    dot / (norm_a * norm_b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedding_roundtrip() {
        let original = vec![1.0f32, 2.0, 3.0, -1.5];
        let bytes = embedding_to_bytes(&original);
        let restored = bytes_to_embedding(&bytes);
        assert_eq!(original, restored);
    }

    #[test]
    fn test_cosine_similarity_identical() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        let sim = cosine_similarity(&a, &b);
        assert!((sim - 1.0).abs() < 0.0001);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        let sim = cosine_similarity(&a, &b);
        assert!(sim.abs() < 0.0001);
    }
}
