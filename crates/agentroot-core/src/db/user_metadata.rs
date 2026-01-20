//! User metadata operations on documents

use super::Database;
use crate::db::metadata::{MetadataFilter, UserMetadata};
use crate::error::Result;
use rusqlite::params;

impl Database {
    /// Add or update user metadata for a document
    ///
    /// # Arguments
    /// * `docid` - Document ID (short hash like "abc123" or full "#abc123")
    /// * `metadata` - User metadata to add/update
    ///
    /// # Example
    /// ```ignore
    /// use agentroot_core::{Database, MetadataBuilder};
    ///
    /// let db = Database::open("index.sqlite")?;
    /// let metadata = MetadataBuilder::new()
    ///     .text("author", "John Doe")
    ///     .tags("labels", vec!["rust", "tutorial"])
    ///     .build();
    ///
    /// db.add_metadata("#abc123", &metadata)?;
    /// ```
    pub fn add_metadata(&self, docid: &str, metadata: &UserMetadata) -> Result<()> {
        let docid = docid.trim_start_matches('#');

        // Find document by docid
        let doc_id = self.conn.query_row(
            "SELECT d.id FROM documents d 
             JOIN content c ON c.hash = d.hash 
             WHERE substr(c.hash, 1, 6) = ?1 AND d.active = 1 
             LIMIT 1",
            params![docid],
            |row| row.get::<_, i64>(0),
        )?;

        // Get existing metadata
        let existing_json: Option<String> = self
            .conn
            .query_row(
                "SELECT user_metadata FROM documents WHERE id = ?1",
                params![doc_id],
                |row| row.get(0),
            )
            .ok()
            .flatten();

        // Merge with existing metadata
        let mut combined = if let Some(json) = existing_json {
            UserMetadata::from_json(&json).unwrap_or_default()
        } else {
            UserMetadata::new()
        };

        combined.merge(metadata);

        // Update database
        let json = combined.to_json()?;
        self.conn.execute(
            "UPDATE documents SET user_metadata = ?1 WHERE id = ?2",
            params![json, doc_id],
        )?;

        Ok(())
    }

    /// Get user metadata for a document
    pub fn get_metadata(&self, docid: &str) -> Result<Option<UserMetadata>> {
        let docid = docid.trim_start_matches('#');

        let result: Option<String> = self
            .conn
            .query_row(
                "SELECT d.user_metadata FROM documents d 
             JOIN content c ON c.hash = d.hash 
             WHERE substr(c.hash, 1, 6) = ?1 AND d.active = 1 
             LIMIT 1",
                params![docid],
                |row| row.get(0),
            )
            .ok()
            .flatten();

        match result {
            Some(json) => Ok(Some(UserMetadata::from_json(&json)?)),
            None => Ok(None),
        }
    }

    /// Remove specific metadata fields from a document
    pub fn remove_metadata_fields(&self, docid: &str, fields: &[String]) -> Result<()> {
        if let Some(mut metadata) = self.get_metadata(docid)? {
            for field in fields {
                metadata.remove(field);
            }

            let docid_clean = docid.trim_start_matches('#');
            let doc_id: i64 = self.conn.query_row(
                "SELECT d.id FROM documents d 
                 JOIN content c ON c.hash = d.hash 
                 WHERE substr(c.hash, 1, 6) = ?1 AND d.active = 1 
                 LIMIT 1",
                params![docid_clean],
                |row| row.get(0),
            )?;

            let json = metadata.to_json()?;
            self.conn.execute(
                "UPDATE documents SET user_metadata = ?1 WHERE id = ?2",
                params![json, doc_id],
            )?;
        }

        Ok(())
    }

    /// Clear all user metadata from a document
    pub fn clear_metadata(&self, docid: &str) -> Result<()> {
        let docid = docid.trim_start_matches('#');

        self.conn.execute(
            "UPDATE documents d 
             SET user_metadata = NULL 
             WHERE d.id IN (
                 SELECT d2.id FROM documents d2
                 JOIN content c ON c.hash = d2.hash 
                 WHERE substr(c.hash, 1, 6) = ?1 AND d2.active = 1 
                 LIMIT 1
             )",
            params![docid],
        )?;

        Ok(())
    }

    /// Find documents matching metadata filter
    pub fn find_by_metadata(&self, filter: &MetadataFilter, limit: usize) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT d.id, c.hash, d.user_metadata 
             FROM documents d 
             JOIN content c ON c.hash = d.hash 
             WHERE d.active = 1 AND d.user_metadata IS NOT NULL 
             LIMIT ?1",
        )?;

        let docids: Vec<String> = stmt
            .query_map(params![limit], |row| {
                let hash: String = row.get(1)?;
                let metadata_json: Option<String> = row.get(2)?;

                if let Some(json) = metadata_json {
                    if let Ok(metadata) = UserMetadata::from_json(&json) {
                        if filter.matches(&metadata) {
                            return Ok(Some(format!("#{}", &hash[..6])));
                        }
                    }
                }
                Ok(None)
            })?
            .filter_map(|r| r.ok().flatten())
            .collect();

        Ok(docids)
    }

    /// List all documents with user metadata
    pub fn list_with_metadata(&self, limit: usize) -> Result<Vec<(String, UserMetadata)>> {
        let mut stmt = self.conn.prepare(
            "SELECT c.hash, d.user_metadata 
             FROM documents d 
             JOIN content c ON c.hash = d.hash 
             WHERE d.active = 1 AND d.user_metadata IS NOT NULL 
             LIMIT ?1",
        )?;

        let results: Vec<(String, UserMetadata)> = stmt
            .query_map(params![limit], |row| {
                let hash: String = row.get(0)?;
                let metadata_json: String = row.get(1)?;
                Ok((hash, metadata_json))
            })?
            .filter_map(|r| r.ok())
            .filter_map(|(hash, json)| {
                UserMetadata::from_json(&json)
                    .ok()
                    .map(|m| (format!("#{}", &hash[..6]), m))
            })
            .collect();

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::metadata::{MetadataBuilder, MetadataFilter};
    use chrono::Utc;

    #[test]
    fn test_add_and_get_metadata() {
        let db = Database::open_in_memory().unwrap();
        db.initialize().unwrap();

        // Insert a test document
        let now = Utc::now().to_rfc3339();
        db.insert_content("testhash123", "test content").unwrap();
        db.insert_document(
            "test",
            "test.md",
            "Test",
            "testhash123",
            &now,
            &now,
            "file",
            None,
        )
        .unwrap();

        // Add metadata
        let metadata = MetadataBuilder::new()
            .text("author", "Alice")
            .tags("labels", vec!["test", "example"])
            .integer("version", 1)
            .build();

        // Get docid
        let docid = format!("#{}", &"testhash123"[..6]);

        db.add_metadata(&docid, &metadata).unwrap();

        // Retrieve metadata
        let retrieved = db.get_metadata(&docid).unwrap();
        assert!(retrieved.is_some());

        let retrieved = retrieved.unwrap();
        assert!(retrieved.contains("author"));
        assert!(retrieved.contains("labels"));
        assert!(retrieved.contains("version"));
    }

    #[test]
    fn test_metadata_merge() {
        let db = Database::open_in_memory().unwrap();
        db.initialize().unwrap();

        let now = Utc::now().to_rfc3339();
        db.insert_content("testhash456", "test content").unwrap();
        db.insert_document(
            "test",
            "test.md",
            "Test",
            "testhash456",
            &now,
            &now,
            "file",
            None,
        )
        .unwrap();

        let docid = format!("#{}", &"testhash456"[..6]);

        // Add initial metadata
        let meta1 = MetadataBuilder::new().text("author", "Alice").build();
        db.add_metadata(&docid, &meta1).unwrap();

        // Add more metadata (should merge)
        let meta2 = MetadataBuilder::new().tags("labels", vec!["rust"]).build();
        db.add_metadata(&docid, &meta2).unwrap();

        // Should have both fields
        let retrieved = db.get_metadata(&docid).unwrap().unwrap();
        assert!(retrieved.contains("author"));
        assert!(retrieved.contains("labels"));
    }

    #[test]
    fn test_find_by_metadata() {
        let db = Database::open_in_memory().unwrap();
        db.initialize().unwrap();

        let now = Utc::now().to_rfc3339();

        // Insert multiple documents with distinct hashes
        for i in 1..=3 {
            let hash = format!("hash{}_abcdef", i);
            let content = format!("content {}", i);
            db.insert_content(&hash, &content).unwrap();
            db.insert_document(
                "test",
                &format!("doc{}.md", i),
                "Test",
                &hash,
                &now,
                &now,
                "file",
                None,
            )
            .unwrap();

            let metadata = MetadataBuilder::new().integer("score", i as i64).build();

            let docid = format!("#{}", &hash[..6]);
            db.add_metadata(&docid, &metadata).unwrap();
        }

        // Find documents with score > 1
        let filter = MetadataFilter::IntegerGt("score".to_string(), 1);
        let results = db.find_by_metadata(&filter, 10).unwrap();

        assert_eq!(results.len(), 2); // hash2_abcdef and hash3_abcdef
    }
}
