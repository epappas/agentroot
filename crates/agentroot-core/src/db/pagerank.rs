//! PageRank-related database operations

use crate::db::Database;
use crate::error::Result;
use crate::graph::{compute_pagerank, extract_links};
use chrono::Utc;
use rusqlite::{params, OptionalExtension};

impl Database {
    /// Build document link graph by extracting links from all documents
    pub fn build_link_graph(&self) -> Result<usize> {
        self.conn.execute("DELETE FROM document_links", [])?;

        let mut stmt = self.conn.prepare(
            "SELECT d.id, d.path, d.collection, c.doc, coll.path as coll_path
             FROM documents d
             JOIN content c ON c.hash = d.hash
             JOIN collections coll ON coll.name = d.collection
             WHERE d.active = 1",
        )?;

        let docs: Vec<(i64, String, String, String, String)> = stmt
            .query_map([], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        let mut total_links = 0;
        let now = Utc::now().to_rfc3339();

        for (doc_id, path, collection, content, coll_path) in docs {
            let links = extract_links(&content, &path, &coll_path);

            for link in links {
                if let Some(target_id) =
                    self.find_document_by_path(&collection, &link.target_path)?
                {
                    self.conn.execute(
                        "INSERT OR IGNORE INTO document_links (source_id, target_id, link_type, created_at)
                         VALUES (?1, ?2, ?3, ?4)",
                        params![doc_id, target_id, link.link_type.as_str(), now],
                    )?;
                    total_links += 1;
                }
            }
        }

        tracing::info!("Built link graph with {} links", total_links);
        Ok(total_links)
    }

    /// Compute PageRank scores and store in documents table
    pub fn compute_and_store_pagerank(&self) -> Result<()> {
        let scores = compute_pagerank(&self.conn)?;
        let count = scores.len();

        for (doc_id, score) in &scores {
            self.conn.execute(
                "UPDATE documents SET importance_score = ?1 WHERE id = ?2",
                params![score, doc_id],
            )?;
        }

        tracing::info!("Updated PageRank scores for {} documents", count);
        Ok(())
    }

    /// Get PageRank statistics and top documents
    pub fn get_pagerank_stats(&self) -> Result<(usize, Vec<(String, f64)>)> {
        let doc_count: usize = self.conn.query_row(
            "SELECT COUNT(*) FROM documents WHERE active = 1",
            [],
            |row| row.get(0),
        )?;

        let top_docs: Vec<(String, f64)> = self
            .conn
            .prepare(
                "SELECT collection || '/' || path, importance_score
             FROM documents
             WHERE active = 1
             ORDER BY importance_score DESC
             LIMIT 10",
            )?
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok((doc_count, top_docs))
    }

    fn find_document_by_path(&self, collection: &str, path: &str) -> Result<Option<i64>> {
        let id = self
            .conn
            .query_row(
                "SELECT id FROM documents WHERE collection = ?1 AND path = ?2 AND active = 1",
                params![collection, path],
                |row| row.get(0),
            )
            .optional()?;

        Ok(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_build_link_graph() {
        let db = Database::open_in_memory().unwrap();
        db.initialize().unwrap();

        let now = Utc::now().to_rfc3339();

        db.conn.execute(
            "INSERT INTO collections (name, path, pattern, created_at, updated_at, provider_type)
             VALUES ('test', '/test', '**/*.md', ?1, ?1, 'file')",
            params![now],
        ).unwrap();

        db.conn
            .execute(
                "INSERT INTO content (hash, doc, created_at) 
             VALUES ('hash1', 'See [doc2](doc2.md)', ?1)",
                params![now],
            )
            .unwrap();

        db.conn
            .execute(
                "INSERT INTO content (hash, doc, created_at) 
             VALUES ('hash2', 'Content of doc2', ?1)",
                params![now],
            )
            .unwrap();

        db.conn
            .execute(
                "INSERT INTO documents (collection, path, title, hash, created_at, modified_at)
             VALUES ('test', 'doc1.md', 'Doc 1', 'hash1', ?1, ?1)",
                params![now],
            )
            .unwrap();

        db.conn
            .execute(
                "INSERT INTO documents (collection, path, title, hash, created_at, modified_at)
             VALUES ('test', 'doc2.md', 'Doc 2', 'hash2', ?1, ?1)",
                params![now],
            )
            .unwrap();

        let link_count = db.build_link_graph().unwrap();
        assert_eq!(link_count, 1);
    }

    #[test]
    fn test_compute_and_store_pagerank() {
        let db = Database::open_in_memory().unwrap();
        db.initialize().unwrap();

        let now = Utc::now().to_rfc3339();

        db.conn.execute(
            "INSERT INTO collections (name, path, pattern, created_at, updated_at, provider_type)
             VALUES ('test', '/test', '**/*.md', ?1, ?1, 'file')",
            params![now],
        ).unwrap();

        db.conn
            .execute(
                "INSERT INTO content (hash, doc, created_at) VALUES ('hash1', 'doc1', ?1)",
                params![now],
            )
            .unwrap();

        db.conn
            .execute(
                "INSERT INTO documents (collection, path, title, hash, created_at, modified_at)
             VALUES ('test', 'doc1.md', 'Doc 1', 'hash1', ?1, ?1)",
                params![now],
            )
            .unwrap();

        db.compute_and_store_pagerank().unwrap();

        let score: f64 = db
            .conn
            .query_row(
                "SELECT importance_score FROM documents WHERE path = 'doc1.md'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert!(score > 0.0, "Score should be positive");
    }
}
