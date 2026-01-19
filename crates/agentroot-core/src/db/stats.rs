//! Database statistics

use super::Database;
use crate::error::Result;

/// Database stats
#[derive(Debug, Clone, serde::Serialize)]
pub struct DatabaseStats {
    pub collection_count: usize,
    pub document_count: usize,
    pub embedded_count: usize,
    pub pending_embedding: usize,
}

impl Database {
    /// Get database statistics
    pub fn get_stats(&self) -> Result<DatabaseStats> {
        let collection_count: i64 =
            self.conn
                .query_row("SELECT COUNT(*) FROM collections", [], |row| row.get(0))?;

        let document_count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM documents WHERE active = 1",
            [],
            |row| row.get(0),
        )?;

        let embedded_count: i64 = self
            .conn
            .query_row(
                "SELECT COUNT(DISTINCT hash) FROM content_vectors",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);

        let pending_embedding: i64 = self
            .conn
            .query_row(
                "SELECT COUNT(DISTINCT c.hash) FROM content c
             JOIN documents d ON d.hash = c.hash AND d.active = 1
             WHERE c.hash NOT IN (SELECT DISTINCT hash FROM content_vectors)",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);

        Ok(DatabaseStats {
            collection_count: collection_count as usize,
            document_count: document_count as usize,
            embedded_count: embedded_count as usize,
            pending_embedding: pending_embedding as usize,
        })
    }

    /// Vacuum the database
    pub fn vacuum(&self) -> Result<()> {
        self.conn.execute("VACUUM", [])?;
        Ok(())
    }

    /// Cleanup orphaned vectors
    pub fn cleanup_orphaned_vectors(&self) -> Result<usize> {
        let rows = self.conn.execute(
            "DELETE FROM content_vectors WHERE hash NOT IN
             (SELECT DISTINCT hash FROM documents WHERE active = 1)",
            [],
        )?;
        Ok(rows)
    }
}
