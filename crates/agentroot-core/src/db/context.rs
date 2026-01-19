//! Context operations

use super::Database;
use crate::error::Result;
use rusqlite::params;

/// Context info
#[derive(Debug, Clone, serde::Serialize)]
pub struct ContextInfo {
    pub path: String,
    pub context: String,
    pub created_at: String,
}

impl Database {
    /// Add context for a path
    pub fn add_context(&self, path: &str, context: &str) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT OR REPLACE INTO contexts (path, context, created_at) VALUES (?1, ?2, ?3)",
            params![path, context, now],
        )?;
        Ok(())
    }

    /// List all contexts
    pub fn list_contexts(&self) -> Result<Vec<ContextInfo>> {
        let mut stmt = self
            .conn
            .prepare("SELECT path, context, created_at FROM contexts ORDER BY path")?;

        let results = stmt
            .query_map([], |row| {
                Ok(ContextInfo {
                    path: row.get(0)?,
                    context: row.get(1)?,
                    created_at: row.get(2)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(results)
    }

    /// Check for collections missing context
    pub fn check_missing_contexts(&self) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT c.name FROM collections c
             WHERE NOT EXISTS (
                 SELECT 1 FROM contexts ctx
                 WHERE ctx.path = 'agentroot://' || c.name || '/'
                    OR ctx.path = '/'
             )
             ORDER BY c.name",
        )?;

        let results = stmt
            .query_map([], |row| row.get(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(results)
    }

    /// Remove context for a path
    pub fn remove_context(&self, path: &str) -> Result<bool> {
        let rows = self
            .conn
            .execute("DELETE FROM contexts WHERE path = ?1", params![path])?;
        Ok(rows > 0)
    }

    /// Get context for a document path (hierarchical resolution)
    pub fn resolve_context(&self, virtual_path: &str) -> Result<Option<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT context FROM contexts
             WHERE ?1 LIKE path || '%'
             ORDER BY LENGTH(path) DESC
             LIMIT 1",
        )?;

        let result = stmt.query_row(params![virtual_path], |row| row.get(0));
        match result {
            Ok(context) => Ok(Some(context)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
}
