//! Long-term memory storage and retrieval

use super::Database;
use crate::error::Result;
use chrono::Utc;
use rusqlite::params;
use std::collections::HashMap;

#[derive(Debug, Clone, serde::Serialize)]
pub struct MemoryInfo {
    pub id: String,
    pub session_id: Option<String>,
    pub category: String,
    pub content: String,
    pub confidence: f64,
    pub source_query: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub access_count: i64,
    pub last_accessed_at: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct MemoryStats {
    pub total: usize,
    pub by_category: HashMap<String, usize>,
    pub avg_confidence: f64,
}

impl Database {
    /// Store a memory. Deduplicates by content hash: on conflict, updates
    /// confidence to max(old, new) and bumps access_count.
    pub fn store_memory(
        &self,
        session_id: Option<&str>,
        category: &str,
        content: &str,
        confidence: f64,
        source_query: Option<&str>,
    ) -> Result<String> {
        let content_hash = blake3::hash(content.as_bytes()).to_hex().to_string();
        let now = Utc::now().to_rfc3339();
        let id = generate_memory_id();

        // Try insert; on content_hash conflict, update existing
        let rows = self.conn.execute(
            "INSERT INTO memories (id, session_id, category, content, content_hash, confidence, source_query, created_at, updated_at, access_count)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8, 0)
             ON CONFLICT(content_hash) DO UPDATE SET
                confidence = MAX(memories.confidence, excluded.confidence),
                access_count = memories.access_count + 1,
                updated_at = excluded.updated_at",
            params![id, session_id, category, content, content_hash, confidence, source_query, now],
        )?;

        if rows == 0 {
            return Err(crate::error::AgentRootError::InvalidInput(
                "Failed to store memory".to_string(),
            ));
        }

        // Return the actual ID (could be existing if deduped)
        let actual_id: String = self.conn.query_row(
            "SELECT id FROM memories WHERE content_hash = ?1",
            params![content_hash],
            |row| row.get(0),
        )?;

        Ok(actual_id)
    }

    /// Full-text search across memories.
    pub fn search_memories(
        &self,
        query: &str,
        category: Option<&str>,
        limit: usize,
    ) -> Result<Vec<MemoryInfo>> {
        let sanitized = crate::search::sanitize_fts5_query(query);
        if sanitized.is_empty() {
            return Ok(vec![]);
        }

        let sql = if category.is_some() {
            "SELECT m.id, m.session_id, m.category, m.content, m.confidence,
                    m.source_query, m.created_at, m.updated_at, m.access_count, m.last_accessed_at
             FROM memories m
             JOIN memories_fts f ON m.rowid = f.rowid
             WHERE memories_fts MATCH ?1 AND m.category = ?2
             ORDER BY f.rank
             LIMIT ?3"
        } else {
            "SELECT m.id, m.session_id, m.category, m.content, m.confidence,
                    m.source_query, m.created_at, m.updated_at, m.access_count, m.last_accessed_at
             FROM memories m
             JOIN memories_fts f ON m.rowid = f.rowid
             WHERE memories_fts MATCH ?1
             ORDER BY f.rank
             LIMIT ?3"
        };

        let mut stmt = self.conn.prepare(sql)?;
        let rows = if let Some(cat) = category {
            stmt.query_map(params![sanitized, cat, limit as i64], row_to_memory)?
                .collect::<std::result::Result<Vec<_>, _>>()?
        } else {
            stmt.query_map(params![sanitized, "", limit as i64], row_to_memory)?
                .collect::<std::result::Result<Vec<_>, _>>()?
        };

        Ok(rows)
    }

    /// List memories with optional category filter, ordered by updated_at DESC.
    pub fn list_memories(
        &self,
        category: Option<&str>,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<MemoryInfo>> {
        let (sql, use_category) = match category {
            Some(_) => (
                "SELECT id, session_id, category, content, confidence,
                        source_query, created_at, updated_at, access_count, last_accessed_at
                 FROM memories WHERE category = ?1
                 ORDER BY updated_at DESC LIMIT ?2 OFFSET ?3",
                true,
            ),
            None => (
                "SELECT id, session_id, category, content, confidence,
                        source_query, created_at, updated_at, access_count, last_accessed_at
                 FROM memories
                 ORDER BY updated_at DESC LIMIT ?1 OFFSET ?2",
                false,
            ),
        };

        let mut stmt = self.conn.prepare(sql)?;
        let rows = if use_category {
            stmt.query_map(
                params![category.unwrap(), limit as i64, offset as i64],
                row_to_memory,
            )?
            .collect::<std::result::Result<Vec<_>, _>>()?
        } else {
            stmt.query_map(params![limit as i64, offset as i64], row_to_memory)?
                .collect::<std::result::Result<Vec<_>, _>>()?
        };

        Ok(rows)
    }

    /// Get a single memory by ID. Bumps access_count and last_accessed_at.
    pub fn get_memory(&self, id: &str) -> Result<Option<MemoryInfo>> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE memories SET access_count = access_count + 1, last_accessed_at = ?1 WHERE id = ?2",
            params![now, id],
        )?;

        let result = self
            .conn
            .query_row(
                "SELECT id, session_id, category, content, confidence,
                        source_query, created_at, updated_at, access_count, last_accessed_at
                 FROM memories WHERE id = ?1",
                params![id],
                row_to_memory,
            )
            .ok();

        Ok(result)
    }

    /// Delete a memory by ID. Returns true if row existed.
    pub fn delete_memory(&self, id: &str) -> Result<bool> {
        let rows = self
            .conn
            .execute("DELETE FROM memories WHERE id = ?1", params![id])?;
        Ok(rows > 0)
    }

    /// Aggregate memory stats.
    pub fn get_memory_stats(&self) -> Result<MemoryStats> {
        let total: usize = self
            .conn
            .query_row("SELECT COUNT(*) FROM memories", [], |row| row.get(0))?;

        let avg_confidence: f64 = self.conn.query_row(
            "SELECT COALESCE(AVG(confidence), 0.0) FROM memories",
            [],
            |row| row.get(0),
        )?;

        let mut stmt = self
            .conn
            .prepare("SELECT category, COUNT(*) FROM memories GROUP BY category")?;
        let by_category: HashMap<String, usize> = stmt
            .query_map([], |row| {
                let cat: String = row.get(0)?;
                let count: usize = row.get(1)?;
                Ok((cat, count))
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(MemoryStats {
            total,
            by_category,
            avg_confidence,
        })
    }
}

fn row_to_memory(row: &rusqlite::Row) -> rusqlite::Result<MemoryInfo> {
    Ok(MemoryInfo {
        id: row.get(0)?,
        session_id: row.get(1)?,
        category: row.get(2)?,
        content: row.get(3)?,
        confidence: row.get(4)?,
        source_query: row.get(5)?,
        created_at: row.get(6)?,
        updated_at: row.get(7)?,
        access_count: row.get(8)?,
        last_accessed_at: row.get(9)?,
    })
}

fn generate_memory_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();

    let pid = std::process::id();
    let seq = COUNTER.fetch_add(1, Ordering::Relaxed);
    let mixed = timestamp ^ (pid as u128 * 6_364_136_223_846_793_005) ^ ((seq as u128) << 32);

    format!("mem-{:016x}{:016x}", (mixed >> 64) as u64, mixed as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_store_and_get_memory() {
        let db = Database::open_in_memory().unwrap();
        db.initialize().unwrap();

        let id = db
            .store_memory(
                Some("sess1"),
                "fact",
                "Rust is a systems language",
                0.9,
                Some("what is rust"),
            )
            .unwrap();

        let mem = db.get_memory(&id).unwrap().unwrap();
        assert_eq!(mem.category, "fact");
        assert_eq!(mem.content, "Rust is a systems language");
        assert!((mem.confidence - 0.9).abs() < 0.001);
        assert_eq!(mem.access_count, 1); // get_memory bumps
    }

    #[test]
    fn test_store_memory_dedup() {
        let db = Database::open_in_memory().unwrap();
        db.initialize().unwrap();

        let id1 = db
            .store_memory(None, "fact", "dedup content", 0.5, None)
            .unwrap();
        let id2 = db
            .store_memory(None, "fact", "dedup content", 0.8, None)
            .unwrap();

        // Same content returns same ID
        assert_eq!(id1, id2);

        let mem = db.get_memory(&id1).unwrap().unwrap();
        // Confidence updated to max
        assert!((mem.confidence - 0.8).abs() < 0.001);
        // access_count bumped by dedup + get
        assert!(mem.access_count >= 1);
    }

    #[test]
    fn test_search_memories() {
        let db = Database::open_in_memory().unwrap();
        db.initialize().unwrap();

        db.store_memory(None, "fact", "Rust ownership model", 0.9, None)
            .unwrap();
        db.store_memory(None, "preference", "User prefers dark mode", 0.7, None)
            .unwrap();
        db.store_memory(None, "fact", "Python uses garbage collection", 0.8, None)
            .unwrap();

        let results = db.search_memories("Rust", None, 10).unwrap();
        assert!(!results.is_empty());
        assert!(results.iter().any(|m| m.content.contains("Rust")));

        // Filter by category
        let results = db.search_memories("Rust", Some("preference"), 10).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_list_memories() {
        let db = Database::open_in_memory().unwrap();
        db.initialize().unwrap();

        db.store_memory(None, "fact", "mem1", 0.5, None).unwrap();
        db.store_memory(None, "entity", "mem2", 0.6, None).unwrap();
        db.store_memory(None, "fact", "mem3", 0.7, None).unwrap();

        let all = db.list_memories(None, 10, 0).unwrap();
        assert_eq!(all.len(), 3);

        let facts = db.list_memories(Some("fact"), 10, 0).unwrap();
        assert_eq!(facts.len(), 2);

        // Pagination
        let page = db.list_memories(None, 2, 0).unwrap();
        assert_eq!(page.len(), 2);
    }

    #[test]
    fn test_delete_memory() {
        let db = Database::open_in_memory().unwrap();
        db.initialize().unwrap();

        let id = db
            .store_memory(None, "fact", "to delete", 0.5, None)
            .unwrap();

        assert!(db.delete_memory(&id).unwrap());
        assert!(!db.delete_memory(&id).unwrap()); // already gone
        assert!(db.get_memory(&id).unwrap().is_none());
    }

    #[test]
    fn test_memory_stats() {
        let db = Database::open_in_memory().unwrap();
        db.initialize().unwrap();

        db.store_memory(None, "fact", "f1", 0.8, None).unwrap();
        db.store_memory(None, "fact", "f2", 0.6, None).unwrap();
        db.store_memory(None, "entity", "e1", 1.0, None).unwrap();

        let stats = db.get_memory_stats().unwrap();
        assert_eq!(stats.total, 3);
        assert_eq!(stats.by_category.get("fact"), Some(&2));
        assert_eq!(stats.by_category.get("entity"), Some(&1));
        assert!((stats.avg_confidence - 0.8).abs() < 0.01);
    }
}
