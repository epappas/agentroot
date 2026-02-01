//! Directory index operations

use super::Database;
use crate::error::Result;
use chrono::Utc;
use rusqlite::params;
use std::collections::HashMap;

#[derive(Debug, Clone, serde::Serialize)]
pub struct DirectoryInfo {
    pub path: String,
    pub collection: String,
    pub depth: usize,
    pub file_count: usize,
    pub child_dir_count: usize,
    pub summary: Option<String>,
    pub dominant_language: Option<String>,
    pub dominant_category: Option<String>,
    pub concepts: Vec<String>,
    pub updated_at: String,
}

impl Database {
    pub fn upsert_directory(&self, info: &DirectoryInfo) -> Result<()> {
        let concepts_json = serde_json::to_string(&info.concepts)?;
        self.conn.execute(
            "INSERT OR REPLACE INTO directories
             (path, collection, depth, file_count, child_dir_count, summary,
              dominant_language, dominant_category, concepts, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                info.path,
                info.collection,
                info.depth as i64,
                info.file_count as i64,
                info.child_dir_count as i64,
                info.summary,
                info.dominant_language,
                info.dominant_category,
                concepts_json,
                info.updated_at,
            ],
        )?;
        Ok(())
    }

    pub fn get_directory(&self, collection: &str, path: &str) -> Result<Option<DirectoryInfo>> {
        let full_path = format!("{}/{}", collection, path);
        let result = self.conn.query_row(
            "SELECT path, collection, depth, file_count, child_dir_count,
                    summary, dominant_language, dominant_category, concepts, updated_at
             FROM directories WHERE path = ?1",
            params![full_path],
            |row| {
                let concepts_json: Option<String> = row.get(8)?;
                let concepts = concepts_json
                    .and_then(|j| serde_json::from_str::<Vec<String>>(&j).ok())
                    .unwrap_or_default();
                Ok(DirectoryInfo {
                    path: row.get(0)?,
                    collection: row.get(1)?,
                    depth: row.get::<_, i64>(2)? as usize,
                    file_count: row.get::<_, i64>(3)? as usize,
                    child_dir_count: row.get::<_, i64>(4)? as usize,
                    summary: row.get(5)?,
                    dominant_language: row.get(6)?,
                    dominant_category: row.get(7)?,
                    concepts,
                    updated_at: row.get(9)?,
                })
            },
        );
        match result {
            Ok(info) => Ok(Some(info)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn list_directories(
        &self,
        collection: &str,
        parent: Option<&str>,
        max_depth: Option<usize>,
    ) -> Result<Vec<DirectoryInfo>> {
        let prefix = match parent {
            Some(p) if !p.is_empty() => format!("{}/{}/", collection, p),
            _ => format!("{}/", collection),
        };

        let base_depth = prefix.matches('/').count().saturating_sub(1) as i64;
        let depth_cap = match max_depth {
            Some(d) => base_depth + d as i64,
            None => i64::MAX,
        };

        let mut stmt = self.conn.prepare(
            "SELECT path, collection, depth, file_count, child_dir_count,
                    summary, dominant_language, dominant_category, concepts, updated_at
             FROM directories
             WHERE collection = ?1 AND path LIKE ?2 AND depth <= ?3
             ORDER BY path",
        )?;

        let results = stmt
            .query_map(
                params![collection, format!("{}%", prefix), depth_cap],
                |row| {
                    let concepts_json: Option<String> = row.get(8)?;
                    let concepts = concepts_json
                        .and_then(|j| serde_json::from_str::<Vec<String>>(&j).ok())
                        .unwrap_or_default();
                    Ok(DirectoryInfo {
                        path: row.get(0)?,
                        collection: row.get(1)?,
                        depth: row.get::<_, i64>(2)? as usize,
                        file_count: row.get::<_, i64>(3)? as usize,
                        child_dir_count: row.get::<_, i64>(4)? as usize,
                        summary: row.get(5)?,
                        dominant_language: row.get(6)?,
                        dominant_category: row.get(7)?,
                        concepts,
                        updated_at: row.get(9)?,
                    })
                },
            )?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(results)
    }

    pub fn search_directories_fts(
        &self,
        query: &str,
        collection: Option<&str>,
        limit: usize,
    ) -> Result<Vec<DirectoryInfo>> {
        let sanitized = crate::search::sanitize_fts5_query(query);
        if sanitized.is_empty() {
            return Ok(vec![]);
        }

        let mut sql = String::from(
            "SELECT d.path, d.collection, d.depth, d.file_count, d.child_dir_count,
                    d.summary, d.dominant_language, d.dominant_category, d.concepts, d.updated_at
             FROM directories_fts fts
             JOIN directories d ON d.rowid = fts.rowid
             WHERE directories_fts MATCH ?1",
        );
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(sanitized)];

        if let Some(coll) = collection {
            sql.push_str(&format!(" AND d.collection = ?{}", params_vec.len() + 1));
            params_vec.push(Box::new(coll.to_string()));
        }

        sql.push_str(&format!(" LIMIT {}", limit));

        let mut stmt = self.conn.prepare(&sql)?;
        let results = stmt
            .query_map(
                rusqlite::params_from_iter(params_vec.iter().map(|p| p.as_ref())),
                |row| {
                    let concepts_json: Option<String> = row.get(8)?;
                    let concepts = concepts_json
                        .and_then(|j| serde_json::from_str::<Vec<String>>(&j).ok())
                        .unwrap_or_default();
                    Ok(DirectoryInfo {
                        path: row.get(0)?,
                        collection: row.get(1)?,
                        depth: row.get::<_, i64>(2)? as usize,
                        file_count: row.get::<_, i64>(3)? as usize,
                        child_dir_count: row.get::<_, i64>(4)? as usize,
                        summary: row.get(5)?,
                        dominant_language: row.get(6)?,
                        dominant_category: row.get(7)?,
                        concepts,
                        updated_at: row.get(9)?,
                    })
                },
            )?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(results)
    }

    /// Rebuild directory index from existing documents in a collection.
    pub fn rebuild_directory_index(&self, collection: &str) -> Result<usize> {
        // Delete existing directory entries for this collection
        self.conn.execute(
            "DELETE FROM directories WHERE collection = ?1",
            params![collection],
        )?;

        // Query all active documents in this collection
        let mut stmt = self.conn.prepare(
            "SELECT path, llm_category, llm_concepts FROM documents
             WHERE collection = ?1 AND active = 1",
        )?;

        let docs: Vec<(String, Option<String>, Option<String>)> = stmt
            .query_map(params![collection], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        // Aggregate by directory
        let mut dir_files: HashMap<String, Vec<(String, Option<String>, Option<String>)>> =
            HashMap::new();

        for (path, category, concepts) in &docs {
            let dir = match path.rsplit_once('/') {
                Some((d, _)) => d.to_string(),
                None => ".".to_string(),
            };
            dir_files.entry(dir).or_default().push((
                path.clone(),
                category.clone(),
                concepts.clone(),
            ));
        }

        let now = Utc::now().to_rfc3339();
        let mut count = 0;

        // Also collect child directory counts
        let all_dirs: Vec<String> = dir_files.keys().cloned().collect();

        for (dir_path, files) in &dir_files {
            let depth = dir_path.matches('/').count();
            let file_count = files.len();

            // Count immediate child directories
            let child_dir_count = all_dirs
                .iter()
                .filter(|d| {
                    d.starts_with(&format!("{}/", dir_path))
                        && d[dir_path.len() + 1..].matches('/').count() == 0
                })
                .count();

            // Determine dominant language from file extensions
            let mut ext_counts: HashMap<&str, usize> = HashMap::new();
            for (path, _, _) in files {
                if let Some(ext) = path.rsplit_once('.').map(|(_, e)| e) {
                    *ext_counts.entry(ext).or_default() += 1;
                }
            }
            let dominant_language = ext_counts
                .into_iter()
                .max_by_key(|(_, count)| *count)
                .map(|(ext, _)| ext.to_string());

            // Determine dominant category
            let mut cat_counts: HashMap<String, usize> = HashMap::new();
            for (_, category, _) in files {
                if let Some(cat) = category {
                    *cat_counts.entry(cat.clone()).or_default() += 1;
                }
            }
            let dominant_category = cat_counts
                .into_iter()
                .max_by_key(|(_, count)| *count)
                .map(|(cat, _)| cat);

            // Collect unique concepts
            let mut all_concepts: Vec<String> = Vec::new();
            for (_, _, concepts_json) in files {
                if let Some(json) = concepts_json {
                    if let Ok(concepts) = serde_json::from_str::<Vec<String>>(json) {
                        all_concepts.extend(concepts);
                    }
                }
            }
            all_concepts.sort();
            all_concepts.dedup();
            all_concepts.truncate(20);

            let full_path = format!("{}/{}", collection, dir_path);
            let info = DirectoryInfo {
                path: full_path,
                collection: collection.to_string(),
                depth,
                file_count,
                child_dir_count,
                summary: None,
                dominant_language,
                dominant_category,
                concepts: all_concepts,
                updated_at: now.clone(),
            };

            self.upsert_directory(&info)?;
            count += 1;
        }

        Ok(count)
    }

    /// Find document hashes under a collection/path prefix.
    pub fn find_documents_by_path_prefix(&self, prefix: &str) -> Result<Vec<String>> {
        // prefix is like "collection/path/..."
        let parts: Vec<&str> = prefix.splitn(2, '/').collect();
        let (collection, path_prefix) = match parts.len() {
            2 => (parts[0], parts[1]),
            1 => (parts[0], ""),
            _ => return Ok(vec![]),
        };

        let mut stmt = self.conn.prepare(
            "SELECT hash FROM documents WHERE collection = ?1 AND path LIKE ?2 AND active = 1",
        )?;

        let results = stmt
            .query_map(params![collection, format!("{}%", path_prefix)], |row| {
                row.get(0)
            })?
            .collect::<std::result::Result<Vec<String>, _>>()?;

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_directory_crud() {
        let db = Database::open_in_memory().unwrap();
        db.initialize().unwrap();

        let now = Utc::now().to_rfc3339();
        let info = DirectoryInfo {
            path: "test/src/auth".to_string(),
            collection: "test".to_string(),
            depth: 2,
            file_count: 3,
            child_dir_count: 1,
            summary: Some("Authentication modules".to_string()),
            dominant_language: Some("rs".to_string()),
            dominant_category: Some("code".to_string()),
            concepts: vec!["jwt".to_string(), "oauth".to_string()],
            updated_at: now,
        };

        db.upsert_directory(&info).unwrap();

        let retrieved = db.get_directory("test", "src/auth").unwrap();
        assert!(retrieved.is_some());
        let dir = retrieved.unwrap();
        assert_eq!(dir.file_count, 3);
        assert_eq!(dir.concepts.len(), 2);
        assert_eq!(dir.summary.as_deref(), Some("Authentication modules"));
    }

    #[test]
    fn test_rebuild_directory_index() {
        let db = Database::open_in_memory().unwrap();
        db.initialize().unwrap();

        db.add_collection("test", "/tmp", "**/*.rs", "file", None)
            .unwrap();

        let now = Utc::now().to_rfc3339();
        let hash = crate::db::hash_content("fn main() {}");
        db.insert_content(&hash, "fn main() {}").unwrap();
        db.insert_document(
            "test",
            "src/auth/login.rs",
            "Login",
            &hash,
            &now,
            &now,
            "file",
            None,
        )
        .unwrap();

        let hash2 = crate::db::hash_content("fn verify() {}");
        db.insert_content(&hash2, "fn verify() {}").unwrap();
        db.insert_document(
            "test",
            "src/auth/jwt.rs",
            "JWT",
            &hash2,
            &now,
            &now,
            "file",
            None,
        )
        .unwrap();

        let hash3 = crate::db::hash_content("fn query() {}");
        db.insert_content(&hash3, "fn query() {}").unwrap();
        db.insert_document(
            "test",
            "src/db/query.rs",
            "Query",
            &hash3,
            &now,
            &now,
            "file",
            None,
        )
        .unwrap();

        let count = db.rebuild_directory_index("test").unwrap();
        assert_eq!(count, 2); // src/auth and src/db

        let dirs = db.list_directories("test", None, None).unwrap();
        assert_eq!(dirs.len(), 2);

        let auth_dir = dirs.iter().find(|d| d.path.contains("auth")).unwrap();
        assert_eq!(auth_dir.file_count, 2);
        assert_eq!(auth_dir.dominant_language.as_deref(), Some("rs"));
    }
}
