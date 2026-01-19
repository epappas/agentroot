//! Document operations

use super::content::docid_from_hash;
use super::Database;
use crate::config::virtual_path::{is_virtual_path, parse_virtual_path};
use crate::error::Result;
use rusqlite::params;
use std::collections::HashMap;
use std::path::PathBuf;

/// Document record from database
#[derive(Debug, Clone)]
pub struct Document {
    pub id: i64,
    pub collection: String,
    pub path: String,
    pub title: String,
    pub hash: String,
    pub created_at: String,
    pub modified_at: String,
    pub active: bool,
    pub source_type: String,
    pub source_uri: Option<String>,
}

/// Document result with content
#[derive(Debug, Clone)]
pub struct DocumentResult {
    pub filepath: String,
    pub display_path: String,
    pub title: String,
    pub context: Option<String>,
    pub hash: String,
    pub docid: String,
    pub collection_name: String,
    pub modified_at: String,
    pub body_length: usize,
    pub body: Option<String>,
}

impl Database {
    /// Insert new document using struct parameters
    pub fn insert_doc(&self, doc: &DocumentInsert) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO documents (collection, path, title, hash, created_at, modified_at, active, source_type, source_uri)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1, ?7, ?8)",
            params![
                doc.collection,
                doc.path,
                doc.title,
                doc.hash,
                doc.created_at,
                doc.modified_at,
                doc.source_type,
                doc.source_uri
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Insert new document (legacy method)
    #[allow(clippy::too_many_arguments)]
    pub fn insert_document(
        &self,
        collection: &str,
        path: &str,
        title: &str,
        hash: &str,
        created_at: &str,
        modified_at: &str,
        source_type: &str,
        source_uri: Option<&str>,
    ) -> Result<i64> {
        let doc = DocumentInsert {
            collection,
            path,
            title,
            hash,
            created_at,
            modified_at,
            source_type,
            source_uri,
        };
        self.insert_doc(&doc)
    }

    /// Update existing document (new content hash)
    pub fn update_document(
        &self,
        id: i64,
        title: &str,
        hash: &str,
        modified_at: &str,
    ) -> Result<()> {
        self.conn.execute(
            "UPDATE documents SET title = ?2, hash = ?3, modified_at = ?4 WHERE id = ?1",
            params![id, title, hash, modified_at],
        )?;
        Ok(())
    }

    /// Update document title only
    pub fn update_document_title(&self, id: i64, title: &str, modified_at: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE documents SET title = ?2, modified_at = ?3 WHERE id = ?1",
            params![id, title, modified_at],
        )?;
        Ok(())
    }

    /// Soft-delete document (set active = 0)
    pub fn deactivate_document(&self, collection: &str, path: &str) -> Result<bool> {
        let rows = self.conn.execute(
            "UPDATE documents SET active = 0 WHERE collection = ?1 AND path = ?2",
            params![collection, path],
        )?;
        Ok(rows > 0)
    }

    /// Find active document by collection and path
    pub fn find_active_document(&self, collection: &str, path: &str) -> Result<Option<Document>> {
        let result = self.conn.query_row(
            "SELECT id, collection, path, title, hash, created_at, modified_at, active, source_type, source_uri
             FROM documents WHERE collection = ?1 AND path = ?2 AND active = 1",
            params![collection, path],
            |row| {
                Ok(Document {
                    id: row.get(0)?,
                    collection: row.get(1)?,
                    path: row.get(2)?,
                    title: row.get(3)?,
                    hash: row.get(4)?,
                    created_at: row.get(5)?,
                    modified_at: row.get(6)?,
                    active: row.get::<_, i32>(7)? == 1,
                    source_type: row.get(8)?,
                    source_uri: row.get(9)?,
                })
            },
        );
        match result {
            Ok(doc) => Ok(Some(doc)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Get all active document paths in collection
    pub fn get_active_document_paths(&self, collection: &str) -> Result<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT path FROM documents WHERE collection = ?1 AND active = 1")?;
        let paths = stmt
            .query_map(params![collection], |row| row.get(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(paths)
    }

    /// Find document by docid (hash prefix)
    pub fn find_by_docid(&self, docid: &str) -> Result<Option<DocumentResult>> {
        let docid = docid.trim_start_matches('#');
        let result = self.conn.query_row(
            "SELECT d.id, d.collection, d.path, d.title, d.hash, d.modified_at,
                    c.doc, LENGTH(c.doc)
             FROM documents d
             JOIN content c ON c.hash = d.hash
             WHERE d.hash LIKE ?1 || '%' AND d.active = 1
             LIMIT 1",
            params![docid],
            |row| {
                Ok(DocumentResult {
                    filepath: format!(
                        "agentroot://{}/{}",
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?
                    ),
                    display_path: format!(
                        "{}/{}",
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?
                    ),
                    title: row.get(3)?,
                    context: None,
                    hash: row.get(4)?,
                    docid: docid_from_hash(&row.get::<_, String>(4)?),
                    collection_name: row.get(1)?,
                    modified_at: row.get(5)?,
                    body: Some(row.get(6)?),
                    body_length: row.get(7)?,
                })
            },
        );
        match result {
            Ok(doc) => Ok(Some(doc)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Hard delete inactive documents
    pub fn delete_inactive_documents(&self) -> Result<usize> {
        let rows = self
            .conn
            .execute("DELETE FROM documents WHERE active = 0", [])?;
        Ok(rows)
    }

    /// Multi-lookup with fallback chain
    pub fn lookup_document(
        &self,
        query: &str,
        collections: &HashMap<String, PathBuf>,
    ) -> Result<Option<DocumentResult>> {
        let query = query.trim();

        // 1. Docid lookup
        if query.starts_with('#')
            || (query.len() == 6 && query.chars().all(|c| c.is_ascii_hexdigit()))
        {
            if let Some(doc) = self.find_by_docid(query)? {
                return Ok(Some(doc));
            }
        }

        // 2. Virtual path lookup
        if is_virtual_path(query) {
            if let Ok((collection, path)) = parse_virtual_path(query) {
                if let Some(doc) = self.find_active_document(&collection, &path)? {
                    return Ok(Some(self.document_to_result(&doc)?));
                }
            }
        }

        // 3. Absolute path with collection lookup
        let expanded = if query.starts_with("~/") {
            dirs::home_dir()
                .map(|home| home.join(&query[2..]).to_string_lossy().to_string())
                .unwrap_or_else(|| query.to_string())
        } else {
            query.to_string()
        };

        let abs_path = std::path::Path::new(&expanded);
        if abs_path.is_absolute() {
            for (coll_name, coll_path) in collections {
                if let Ok(rel_path) = abs_path.strip_prefix(coll_path) {
                    let path = rel_path.to_string_lossy().to_string();
                    if let Some(doc) = self.find_active_document(coll_name, &path)? {
                        return Ok(Some(self.document_to_result(&doc)?));
                    }
                }
            }
        }

        // 4. Fuzzy matching fallback
        let candidates = self.fuzzy_find_documents(query, 1)?;
        Ok(candidates.into_iter().next())
    }

    /// Fuzzy matching using simple contains + length
    pub fn fuzzy_find_documents(&self, query: &str, limit: usize) -> Result<Vec<DocumentResult>> {
        let query_lower = query.to_lowercase();
        let mut stmt = self.conn.prepare(
            "SELECT d.collection, d.path, d.title, d.hash, d.modified_at, c.doc, LENGTH(c.doc)
             FROM documents d
             JOIN content c ON c.hash = d.hash
             WHERE d.active = 1 AND (LOWER(d.path) LIKE '%' || ?1 || '%' OR LOWER(d.title) LIKE '%' || ?1 || '%')
             ORDER BY LENGTH(d.path)
             LIMIT ?2"
        )?;

        let results = stmt
            .query_map(params![query_lower, limit as i64], |row| {
                Ok(DocumentResult {
                    filepath: format!(
                        "agentroot://{}/{}",
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?
                    ),
                    display_path: format!(
                        "{}/{}",
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?
                    ),
                    title: row.get(2)?,
                    context: None,
                    hash: row.get(3)?,
                    docid: docid_from_hash(&row.get::<_, String>(3)?),
                    collection_name: row.get(0)?,
                    modified_at: row.get(4)?,
                    body: Some(row.get(5)?),
                    body_length: row.get(6)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(results)
    }

    fn document_to_result(&self, doc: &Document) -> Result<DocumentResult> {
        let body = self.get_content(&doc.hash)?;
        let body_length = body.as_ref().map(|b| b.len()).unwrap_or(0);

        Ok(DocumentResult {
            filepath: format!("agentroot://{}/{}", doc.collection, doc.path),
            display_path: format!("{}/{}", doc.collection, doc.path),
            title: doc.title.clone(),
            context: None,
            hash: doc.hash.clone(),
            docid: docid_from_hash(&doc.hash),
            collection_name: doc.collection.clone(),
            modified_at: doc.modified_at.clone(),
            body_length,
            body,
        })
    }

    /// Get document content by query (docid, virtual path, etc)
    pub fn get_document(&self, query: &str) -> Result<String> {
        let query = query.trim();

        // Docid lookup
        if query.starts_with('#')
            || (query.len() == 6 && query.chars().all(|c| c.is_ascii_hexdigit()))
        {
            if let Some(doc) = self.find_by_docid(query)? {
                return doc.body.ok_or_else(|| {
                    crate::error::AgentRootError::DocumentNotFound(query.to_string())
                });
            }
        }

        // Virtual path lookup
        if is_virtual_path(query) {
            if let Ok((collection, path)) = parse_virtual_path(query) {
                if let Some(doc) = self.find_active_document(&collection, &path)? {
                    if let Some(content) = self.get_content(&doc.hash)? {
                        return Ok(content);
                    }
                }
            }
        }

        // Path prefix lookup (collection/path)
        if query.contains('/') {
            let parts: Vec<&str> = query.splitn(2, '/').collect();
            if parts.len() == 2 {
                if let Some(doc) = self.find_active_document(parts[0], parts[1])? {
                    if let Some(content) = self.get_content(&doc.hash)? {
                        return Ok(content);
                    }
                }
            }
        }

        Err(crate::error::AgentRootError::DocumentNotFound(
            query.to_string(),
        ))
    }

    /// List documents by prefix
    pub fn list_documents_by_prefix(&self, prefix: &str) -> Result<Vec<DocumentListItem>> {
        let prefix = prefix.trim_start_matches("agentroot://");
        let like_pattern = format!("{}%", prefix);

        let mut stmt = self.conn.prepare(
            "SELECT d.collection, d.path, d.title, d.hash
             FROM documents d
             WHERE d.active = 1 AND (d.collection || '/' || d.path) LIKE ?1
             ORDER BY d.collection, d.path",
        )?;

        let results = stmt
            .query_map(params![like_pattern], |row| {
                Ok(DocumentListItem {
                    path: format!("{}/{}", row.get::<_, String>(0)?, row.get::<_, String>(1)?),
                    title: row.get(2)?,
                    docid: docid_from_hash(&row.get::<_, String>(3)?),
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(results)
    }

    /// Get multiple documents by pattern
    pub fn get_documents_by_pattern(&self, pattern: &str) -> Result<Vec<DocumentContent>> {
        // Handle comma-separated list of docids
        if pattern.contains(',') {
            let mut results = Vec::new();
            for part in pattern.split(',') {
                let part = part.trim();
                if let Ok(content) = self.get_document(part) {
                    results.push(DocumentContent {
                        path: part.to_string(),
                        content,
                    });
                }
            }
            return Ok(results);
        }

        // Glob pattern matching
        let pattern = glob::Pattern::new(pattern)?;
        let mut stmt = self.conn.prepare(
            "SELECT d.collection, d.path, c.doc
             FROM documents d
             JOIN content c ON c.hash = d.hash
             WHERE d.active = 1",
        )?;

        let results = stmt
            .query_map([], |row| {
                let path = format!("{}/{}", row.get::<_, String>(0)?, row.get::<_, String>(1)?);
                Ok((path, row.get::<_, String>(2)?))
            })?
            .filter_map(|r| r.ok())
            .filter(|(path, _)| pattern.matches(path))
            .map(|(path, content)| DocumentContent { path, content })
            .collect();

        Ok(results)
    }
}

/// Document list item (for ls command)
#[derive(Debug, Clone, serde::Serialize)]
pub struct DocumentListItem {
    pub path: String,
    pub title: String,
    pub docid: String,
}

/// Document content (for multi-get)
#[derive(Debug, Clone)]
pub struct DocumentContent {
    pub path: String,
    pub content: String,
}

/// Document insert parameters
#[derive(Debug, Clone)]
pub struct DocumentInsert<'a> {
    pub collection: &'a str,
    pub path: &'a str,
    pub title: &'a str,
    pub hash: &'a str,
    pub created_at: &'a str,
    pub modified_at: &'a str,
    pub source_type: &'a str,
    pub source_uri: Option<&'a str>,
}

impl<'a> DocumentInsert<'a> {
    /// Create new document insert parameters
    pub fn new(
        collection: &'a str,
        path: &'a str,
        title: &'a str,
        hash: &'a str,
        created_at: &'a str,
        modified_at: &'a str,
    ) -> Self {
        Self {
            collection,
            path,
            title,
            hash,
            created_at,
            modified_at,
            source_type: "file",
            source_uri: None,
        }
    }

    /// Set source type
    pub fn with_source_type(mut self, source_type: &'a str) -> Self {
        self.source_type = source_type;
        self
    }

    /// Set source URI
    pub fn with_source_uri(mut self, source_uri: &'a str) -> Self {
        self.source_uri = Some(source_uri);
        self
    }
}
