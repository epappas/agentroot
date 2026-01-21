//! SQL Provider for indexing database content
//!
//! Supports SQLite databases with configurable queries.
//! Can be extended to support PostgreSQL and MySQL.

use crate::db::hash_content;
use crate::error::{AgentRootError, Result};
use crate::providers::{ProviderConfig, SourceItem, SourceProvider};
use async_trait::async_trait;
use rusqlite::{params, Connection};
use std::path::Path;

/// Provider for extracting content from SQL databases
pub struct SQLProvider;

impl Default for SQLProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl SQLProvider {
    /// Create a new SQLProvider
    pub fn new() -> Self {
        Self
    }

    /// Execute query and extract rows as SourceItems
    fn query_database(
        &self,
        db_path: &str,
        query: &str,
        config: &ProviderConfig,
    ) -> Result<Vec<SourceItem>> {
        let conn = Connection::open(db_path).map_err(|e| {
            AgentRootError::Database(rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(1),
                Some(format!("Failed to open database {}: {}", db_path, e)),
            ))
        })?;

        let mut stmt = conn
            .prepare(query)
            .map_err(|e| AgentRootError::InvalidInput(format!("Invalid SQL query: {}", e)))?;

        let id_column = config
            .options
            .get("id_column")
            .map(|s| s.as_str())
            .unwrap_or("id");
        let title_column = config
            .options
            .get("title_column")
            .map(|s| s.as_str())
            .unwrap_or("title");
        let content_column = config
            .options
            .get("content_column")
            .map(|s| s.as_str())
            .unwrap_or("content");

        let column_count = stmt.column_count();
        let column_names: Vec<String> = (0..column_count)
            .map(|i| stmt.column_name(i).unwrap_or("").to_string())
            .collect();

        let id_idx = column_names
            .iter()
            .position(|name| name.eq_ignore_ascii_case(id_column))
            .ok_or_else(|| {
                AgentRootError::InvalidInput(format!(
                    "Column '{}' not found in query result",
                    id_column
                ))
            })?;

        let title_idx = column_names
            .iter()
            .position(|name| name.eq_ignore_ascii_case(title_column));

        let content_idx = column_names
            .iter()
            .position(|name| name.eq_ignore_ascii_case(content_column))
            .ok_or_else(|| {
                AgentRootError::InvalidInput(format!(
                    "Column '{}' not found in query result",
                    content_column
                ))
            })?;

        let rows = stmt
            .query_map(params![], |row| {
                let id: String = match row.get_ref(id_idx)? {
                    rusqlite::types::ValueRef::Integer(i) => i.to_string(),
                    rusqlite::types::ValueRef::Text(s) => String::from_utf8_lossy(s).to_string(),
                    rusqlite::types::ValueRef::Real(f) => f.to_string(),
                    _ => row.get(id_idx)?,
                };

                let title: String = if let Some(idx) = title_idx {
                    row.get(idx).unwrap_or_else(|_| id.clone())
                } else {
                    id.clone()
                };
                let content: String = row.get(content_idx)?;

                Ok((id, title, content))
            })
            .map_err(AgentRootError::Database)?;

        let mut items = Vec::new();
        for row_result in rows {
            let (id, title, content) = row_result.map_err(AgentRootError::Database)?;

            if content.trim().is_empty() {
                continue;
            }

            let hash = hash_content(&content);
            let uri = format!("sql://{}/{}", db_path, id);

            let mut item = SourceItem::new(uri, title, content, hash, "sql".to_string());
            item.metadata
                .insert("database".to_string(), db_path.to_string());
            item.metadata.insert("row_id".to_string(), id.clone());
            item.metadata.insert(
                "table".to_string(),
                config
                    .options
                    .get("table")
                    .cloned()
                    .unwrap_or_else(|| "unknown".to_string()),
            );

            items.push(item);
        }

        Ok(items)
    }
}

#[async_trait]
impl SourceProvider for SQLProvider {
    fn provider_type(&self) -> &'static str {
        "sql"
    }

    async fn list_items(&self, config: &ProviderConfig) -> Result<Vec<SourceItem>> {
        let db_path = &config.base_path;

        if !Path::new(db_path).exists() {
            return Err(AgentRootError::InvalidInput(format!(
                "Database file does not exist: {}",
                db_path
            )));
        }

        let query = if let Some(custom_query) = config.options.get("query") {
            custom_query.clone()
        } else if let Some(table) = config.options.get("table") {
            let id_col = config
                .options
                .get("id_column")
                .map(|s| s.as_str())
                .unwrap_or("id");
            let title_col = config
                .options
                .get("title_column")
                .map(|s| s.as_str())
                .unwrap_or("title");
            let content_col = config
                .options
                .get("content_column")
                .map(|s| s.as_str())
                .unwrap_or("content");

            format!(
                "SELECT {}, {}, {} FROM {}",
                id_col, title_col, content_col, table
            )
        } else {
            return Err(AgentRootError::InvalidInput(
                "SQL provider requires either 'query' or 'table' option".to_string(),
            ));
        };

        self.query_database(db_path, &query, config)
    }

    async fn fetch_item(&self, uri: &str) -> Result<SourceItem> {
        if !uri.starts_with("sql://") {
            return Err(AgentRootError::InvalidInput(format!(
                "Invalid SQL URI: {}. Expected format: sql://path/to/db.sqlite/id",
                uri
            )));
        }

        let parts: Vec<&str> = uri.strip_prefix("sql://").unwrap().splitn(2, '/').collect();
        if parts.len() != 2 {
            return Err(AgentRootError::InvalidInput(format!(
                "Invalid SQL URI format: {}. Expected: sql://path/to/db.sqlite/id",
                uri
            )));
        }

        let (db_path, id) = (parts[0], parts[1]);

        let conn = Connection::open(db_path)?;

        let mut stmt = conn.prepare("SELECT id, title, content FROM items WHERE id = ?1")?;

        let result = stmt.query_row(params![id], |row| {
            let id_val: String = match row.get_ref(0)? {
                rusqlite::types::ValueRef::Integer(i) => i.to_string(),
                rusqlite::types::ValueRef::Text(s) => String::from_utf8_lossy(s).to_string(),
                rusqlite::types::ValueRef::Real(f) => f.to_string(),
                _ => row.get(0)?,
            };
            let title: String = row.get(1)?;
            let content: String = row.get(2)?;
            Ok((id_val, title, content))
        })?;

        let (id, title, content) = result;
        let hash = hash_content(&content);

        let mut item = SourceItem::new(uri.to_string(), title, content, hash, "sql".to_string());
        item.metadata
            .insert("database".to_string(), db_path.to_string());
        item.metadata.insert("row_id".to_string(), id);

        Ok(item)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    fn create_test_db() -> NamedTempFile {
        let temp_file = NamedTempFile::new().unwrap();
        let conn = Connection::open(temp_file.path()).unwrap();

        conn.execute(
            "CREATE TABLE documents (
                id INTEGER PRIMARY KEY,
                title TEXT NOT NULL,
                content TEXT NOT NULL
            )",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO documents (id, title, content) VALUES (1, 'First Document', 'Content of first document')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO documents (id, title, content) VALUES (2, 'Second Document', 'Content of second document')",
            [],
        )
        .unwrap();

        temp_file
    }

    #[test]
    fn test_provider_type() {
        let provider = SQLProvider::new();
        assert_eq!(provider.provider_type(), "sql");
    }

    #[tokio::test]
    async fn test_query_database() {
        let temp_db = create_test_db();
        let provider = SQLProvider::new();

        let mut config =
            ProviderConfig::new(temp_db.path().to_string_lossy().to_string(), "".to_string());
        config
            .options
            .insert("table".to_string(), "documents".to_string());
        config
            .options
            .insert("id_column".to_string(), "id".to_string());
        config
            .options
            .insert("title_column".to_string(), "title".to_string());
        config
            .options
            .insert("content_column".to_string(), "content".to_string());

        let items = provider.list_items(&config).await.unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].title, "First Document");
        assert_eq!(items[1].title, "Second Document");
    }

    #[tokio::test]
    async fn test_custom_query() {
        let temp_db = create_test_db();
        let provider = SQLProvider::new();

        let mut config =
            ProviderConfig::new(temp_db.path().to_string_lossy().to_string(), "".to_string());
        config.options.insert(
            "query".to_string(),
            "SELECT id, title, content FROM documents WHERE id = 1".to_string(),
        );

        let items = provider.list_items(&config).await.unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].title, "First Document");
    }

    #[tokio::test]
    async fn test_missing_table_option() {
        let temp_db = create_test_db();
        let provider = SQLProvider::new();

        let config =
            ProviderConfig::new(temp_db.path().to_string_lossy().to_string(), "".to_string());

        let result = provider.list_items(&config).await;
        assert!(result.is_err());
    }
}
