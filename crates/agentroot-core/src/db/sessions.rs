//! Session management for multi-turn agent interactions

use super::Database;
use crate::error::Result;
use crate::search::SearchResult;
use chrono::Utc;
use rusqlite::params;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, serde::Serialize)]
pub struct SessionInfo {
    pub id: String,
    pub created_at: String,
    pub last_active_at: String,
    pub ttl_seconds: i64,
    pub context: HashMap<String, String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SessionQuery {
    pub query: String,
    pub result_count: usize,
    pub top_results: Vec<String>,
    pub created_at: String,
}

impl Database {
    pub fn create_session(&self, ttl_seconds: Option<i64>) -> Result<String> {
        let id = generate_uuid();
        let now = Utc::now().to_rfc3339();
        let ttl = ttl_seconds.unwrap_or(3600);

        self.conn.execute(
            "INSERT INTO sessions (id, created_at, last_active_at, ttl_seconds, context)
             VALUES (?1, ?2, ?2, ?3, '{}')",
            params![id, now, ttl],
        )?;

        // Cleanup expired sessions while we're at it
        self.cleanup_expired_sessions()?;

        Ok(id)
    }

    pub fn get_session(&self, session_id: &str) -> Result<Option<SessionInfo>> {
        let result = self.conn.query_row(
            "SELECT id, created_at, last_active_at, ttl_seconds, context
             FROM sessions WHERE id = ?1",
            params![session_id],
            |row| {
                let context_json: Option<String> = row.get(4)?;
                let context = context_json
                    .and_then(|j| serde_json::from_str::<HashMap<String, String>>(&j).ok())
                    .unwrap_or_default();
                Ok(SessionInfo {
                    id: row.get(0)?,
                    created_at: row.get(1)?,
                    last_active_at: row.get(2)?,
                    ttl_seconds: row.get(3)?,
                    context,
                })
            },
        );
        match result {
            Ok(info) => Ok(Some(info)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn touch_session(&self, session_id: &str) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE sessions SET last_active_at = ?2 WHERE id = ?1",
            params![session_id, now],
        )?;
        Ok(())
    }

    pub fn set_session_context(&self, session_id: &str, key: &str, value: &str) -> Result<()> {
        let session = self.get_session(session_id)?.ok_or_else(|| {
            crate::error::AgentRootError::InvalidInput(format!("Session not found: {}", session_id))
        })?;

        let mut context = session.context;
        context.insert(key.to_string(), value.to_string());

        let context_json = serde_json::to_string(&context)?;
        self.conn.execute(
            "UPDATE sessions SET context = ?2 WHERE id = ?1",
            params![session_id, context_json],
        )?;

        self.touch_session(session_id)?;
        Ok(())
    }

    pub fn get_session_context(&self, session_id: &str) -> Result<HashMap<String, String>> {
        let session = self.get_session(session_id)?;
        Ok(session.map(|s| s.context).unwrap_or_default())
    }

    pub fn log_session_query(
        &self,
        session_id: &str,
        query: &str,
        results: &[SearchResult],
    ) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        let top_hashes: Vec<String> = results.iter().take(5).map(|r| r.hash.clone()).collect();
        let top_json = serde_json::to_string(&top_hashes)?;

        self.conn.execute(
            "INSERT INTO session_queries (session_id, query, result_count, top_results, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![session_id, query, results.len() as i64, top_json, now],
        )?;
        Ok(())
    }

    pub fn get_session_queries(&self, session_id: &str) -> Result<Vec<SessionQuery>> {
        let mut stmt = self.conn.prepare(
            "SELECT query, result_count, top_results, created_at
             FROM session_queries WHERE session_id = ?1 ORDER BY id",
        )?;

        let results = stmt
            .query_map(params![session_id], |row| {
                let top_json: Option<String> = row.get(2)?;
                let top_results = top_json
                    .and_then(|j| serde_json::from_str::<Vec<String>>(&j).ok())
                    .unwrap_or_default();
                Ok(SessionQuery {
                    query: row.get(0)?,
                    result_count: row.get::<_, i64>(1)? as usize,
                    top_results,
                    created_at: row.get(3)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(results)
    }

    pub fn mark_seen(
        &self,
        session_id: &str,
        doc_hash: &str,
        chunk_hash: Option<&str>,
        detail_level: &str,
    ) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        let chunk = chunk_hash.unwrap_or("");

        self.conn.execute(
            "INSERT OR REPLACE INTO session_seen
             (session_id, document_hash, chunk_hash, detail_level, seen_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![session_id, doc_hash, chunk, detail_level, now],
        )?;
        Ok(())
    }

    pub fn get_seen_hashes(&self, session_id: &str) -> Result<HashSet<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT document_hash, chunk_hash FROM session_seen WHERE session_id = ?1")?;

        let mut seen = HashSet::new();
        let rows = stmt.query_map(params![session_id], |row| {
            let doc_hash: String = row.get(0)?;
            let chunk_hash: String = row.get(1)?;
            Ok((doc_hash, chunk_hash))
        })?;

        for row in rows {
            let (doc_hash, chunk_hash) = row?;
            seen.insert(doc_hash);
            if !chunk_hash.is_empty() {
                seen.insert(chunk_hash);
            }
        }

        Ok(seen)
    }

    pub fn cleanup_expired_sessions(&self) -> Result<usize> {
        // Delete sessions where last_active_at + ttl_seconds < now
        let now = Utc::now().to_rfc3339();
        let deleted = self.conn.execute(
            "DELETE FROM sessions WHERE datetime(last_active_at, '+' || ttl_seconds || ' seconds') < datetime(?1)",
            params![now],
        )?;
        Ok(deleted)
    }

    pub fn delete_session(&self, session_id: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM sessions WHERE id = ?1", params![session_id])?;
        Ok(())
    }
}

fn generate_uuid() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();

    let pid = std::process::id();
    let seq = COUNTER.fetch_add(1, Ordering::Relaxed);
    let random_part = timestamp ^ (pid as u128 * 6_364_136_223_846_793_005) ^ ((seq as u128) << 32);

    format!(
        "{:08x}-{:04x}-4{:03x}-{:04x}-{:012x}",
        (random_part >> 96) as u32,
        (random_part >> 80) as u16,
        (random_part >> 64) as u16 & 0x0FFF,
        ((random_part >> 48) as u16 & 0x3FFF) | 0x8000,
        random_part as u64 & 0xFFFF_FFFF_FFFF,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_lifecycle() {
        let db = Database::open_in_memory().unwrap();
        db.initialize().unwrap();

        // Create session
        let session_id = db.create_session(Some(3600)).unwrap();
        assert!(!session_id.is_empty());

        // Get session
        let session = db.get_session(&session_id).unwrap();
        assert!(session.is_some());
        let session = session.unwrap();
        assert_eq!(session.ttl_seconds, 3600);
        assert!(session.context.is_empty());

        // Set context
        db.set_session_context(&session_id, "topic", "authentication")
            .unwrap();
        let ctx = db.get_session_context(&session_id).unwrap();
        assert_eq!(ctx.get("topic").unwrap(), "authentication");

        // Delete session
        db.delete_session(&session_id).unwrap();
        let session = db.get_session(&session_id).unwrap();
        assert!(session.is_none());
    }

    #[test]
    fn test_session_seen_tracking() {
        let db = Database::open_in_memory().unwrap();
        db.initialize().unwrap();

        let session_id = db.create_session(None).unwrap();

        db.mark_seen(&session_id, "hash1", None, "L1").unwrap();
        db.mark_seen(&session_id, "hash2", Some("chunk_abc"), "L0")
            .unwrap();

        let seen = db.get_seen_hashes(&session_id).unwrap();
        assert!(seen.contains("hash1"));
        assert!(seen.contains("hash2"));
        assert!(seen.contains("chunk_abc"));
    }

    #[test]
    fn test_session_query_logging() {
        let db = Database::open_in_memory().unwrap();
        db.initialize().unwrap();

        let session_id = db.create_session(None).unwrap();

        // Log with empty results
        db.log_session_query(&session_id, "test query", &[])
            .unwrap();

        let queries = db.get_session_queries(&session_id).unwrap();
        assert_eq!(queries.len(), 1);
        assert_eq!(queries[0].query, "test query");
        assert_eq!(queries[0].result_count, 0);
    }

    #[test]
    fn test_generate_uuid() {
        let uuid1 = generate_uuid();
        let uuid2 = generate_uuid();
        assert!(!uuid1.is_empty());
        assert!(uuid1.contains('-'));
        // Atomic counter guarantees uniqueness even in same nanosecond
        assert_ne!(uuid1, uuid2);
    }

    #[test]
    fn test_generate_uuid_batch_uniqueness() {
        let uuids: Vec<String> = (0..100).map(|_| generate_uuid()).collect();
        let unique: std::collections::HashSet<&String> = uuids.iter().collect();
        assert_eq!(uuids.len(), unique.len());
    }
}
