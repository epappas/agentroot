//! Content storage operations

use super::Database;
use crate::error::Result;
use chrono::Utc;
use rusqlite::params;
use sha2::{Digest, Sha256};

/// Hash content using SHA-256
pub fn hash_content(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Generate short docid (first 6 chars of hash)
pub fn docid_from_hash(hash: &str) -> String {
    hash.chars().take(6).collect()
}

impl Database {
    /// Insert content if not exists (content-addressable)
    pub fn insert_content(&self, hash: &str, content: &str) -> Result<bool> {
        let now = Utc::now().to_rfc3339();
        let rows = self.conn.execute(
            "INSERT OR IGNORE INTO content (hash, doc, created_at) VALUES (?1, ?2, ?3)",
            params![hash, content, now],
        )?;
        Ok(rows > 0)
    }

    /// Get content by hash
    pub fn get_content(&self, hash: &str) -> Result<Option<String>> {
        let result = self.conn.query_row(
            "SELECT doc FROM content WHERE hash = ?1",
            params![hash],
            |row| row.get(0),
        );
        match result {
            Ok(content) => Ok(Some(content)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Delete orphaned content (not referenced by any active document)
    pub fn cleanup_orphaned_content(&self) -> Result<usize> {
        let rows = self.conn.execute(
            "DELETE FROM content WHERE hash NOT IN
             (SELECT DISTINCT hash FROM documents WHERE active = 1)",
            [],
        )?;
        Ok(rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_content() {
        let hash = hash_content("Hello, World!");
        assert_eq!(hash.len(), 64);
    }

    #[test]
    fn test_docid_from_hash() {
        let hash = "abcdef1234567890";
        assert_eq!(docid_from_hash(hash), "abcdef");
    }
}
