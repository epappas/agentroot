//! Collection operations

use super::Database;
use crate::error::Result;
use chrono::Utc;
use rusqlite::params;

/// Collection info
#[derive(Debug, Clone, serde::Serialize)]
pub struct CollectionInfo {
    pub name: String,
    pub path: String,
    pub pattern: String,
    pub document_count: usize,
    pub created_at: String,
    pub updated_at: String,
    pub provider_type: String,
    pub provider_config: Option<String>,
}

impl Database {
    /// Add a new collection
    pub fn add_collection(
        &self,
        name: &str,
        path: &str,
        pattern: &str,
        provider_type: &str,
        provider_config: Option<&str>,
    ) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO collections (name, path, pattern, created_at, updated_at, provider_type, provider_config)
             VALUES (?1, ?2, ?3, ?4, ?4, ?5, ?6)",
            params![name, path, pattern, now, provider_type, provider_config],
        )?;
        Ok(())
    }

    /// Remove a collection and its documents
    pub fn remove_collection(&self, name: &str) -> Result<bool> {
        // Deactivate all documents
        self.conn.execute(
            "UPDATE documents SET active = 0 WHERE collection = ?1",
            params![name],
        )?;

        // Remove collection
        let rows = self
            .conn
            .execute("DELETE FROM collections WHERE name = ?1", params![name])?;

        Ok(rows > 0)
    }

    /// Rename a collection
    pub fn rename_collection(&self, old_name: &str, new_name: &str) -> Result<bool> {
        let now = Utc::now().to_rfc3339();

        // Update documents
        self.conn.execute(
            "UPDATE documents SET collection = ?2 WHERE collection = ?1",
            params![old_name, new_name],
        )?;

        // Update collection
        let rows = self.conn.execute(
            "UPDATE collections SET name = ?2, updated_at = ?3 WHERE name = ?1",
            params![old_name, new_name, now],
        )?;

        Ok(rows > 0)
    }

    /// List all collections with document counts
    pub fn list_collections(&self) -> Result<Vec<CollectionInfo>> {
        let mut stmt = self.conn.prepare(
            "SELECT c.name, c.path, c.pattern, c.created_at, c.updated_at,
                    (SELECT COUNT(*) FROM documents d WHERE d.collection = c.name AND d.active = 1),
                    c.provider_type, c.provider_config
             FROM collections c
             ORDER BY c.name",
        )?;

        let results = stmt
            .query_map([], |row| {
                Ok(CollectionInfo {
                    name: row.get(0)?,
                    path: row.get(1)?,
                    pattern: row.get(2)?,
                    created_at: row.get(3)?,
                    updated_at: row.get(4)?,
                    document_count: row.get::<_, i64>(5)? as usize,
                    provider_type: row.get(6)?,
                    provider_config: row.get(7)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(results)
    }

    /// Get collection by name
    pub fn get_collection(&self, name: &str) -> Result<Option<CollectionInfo>> {
        let result = self.conn.query_row(
            "SELECT c.name, c.path, c.pattern, c.created_at, c.updated_at,
                    (SELECT COUNT(*) FROM documents d WHERE d.collection = c.name AND d.active = 1),
                    c.provider_type, c.provider_config
             FROM collections c WHERE c.name = ?1",
            params![name],
            |row| {
                Ok(CollectionInfo {
                    name: row.get(0)?,
                    path: row.get(1)?,
                    pattern: row.get(2)?,
                    created_at: row.get(3)?,
                    updated_at: row.get(4)?,
                    document_count: row.get::<_, i64>(5)? as usize,
                    provider_type: row.get(6)?,
                    provider_config: row.get(7)?,
                })
            },
        );
        match result {
            Ok(info) => Ok(Some(info)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Update collection's updated_at timestamp
    pub fn touch_collection(&self, name: &str) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE collections SET updated_at = ?2 WHERE name = ?1",
            params![name, now],
        )?;
        Ok(())
    }

    /// Reindex a collection using the provider system
    pub fn reindex_collection(&self, name: &str) -> Result<usize> {
        let coll = self
            .get_collection(name)?
            .ok_or_else(|| crate::error::AgentRootError::CollectionNotFound(name.to_string()))?;

        let registry = crate::providers::ProviderRegistry::with_defaults();
        let provider = registry.get(&coll.provider_type).ok_or_else(|| {
            crate::error::AgentRootError::InvalidInput(format!(
                "Unknown provider type: {}",
                coll.provider_type
            ))
        })?;

        let mut config =
            crate::providers::ProviderConfig::new(coll.path.clone(), coll.pattern.clone());

        if let Some(provider_config) = &coll.provider_config {
            if let Ok(config_map) =
                serde_json::from_str::<std::collections::HashMap<String, String>>(provider_config)
            {
                for (key, value) in config_map {
                    config = config.with_option(key, value);
                }
            }
        }

        let items = provider.list_items(&config)?;
        let mut updated = 0;

        for item in items {
            let now = Utc::now().to_rfc3339();

            if let Some(existing) = self.find_active_document(name, &item.uri)? {
                if existing.hash != item.hash {
                    self.update_document(existing.id, &item.title, &item.hash, &now)?;
                    self.insert_content(&item.hash, &item.content)?;
                    updated += 1;
                }
            } else {
                self.insert_content(&item.hash, &item.content)?;
                self.insert_document(
                    name,
                    &item.uri,
                    &item.title,
                    &item.hash,
                    &now,
                    &now,
                    &item.source_type,
                    item.metadata.get("source_uri").map(|s| s.as_str()),
                )?;
                updated += 1;
            }
        }

        self.touch_collection(name)?;
        Ok(updated)
    }
}
