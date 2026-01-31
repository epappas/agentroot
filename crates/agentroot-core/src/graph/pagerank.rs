//! Document importance computation with type-aware scoring

use crate::error::Result;
use rusqlite::Connection;
use std::collections::HashMap;

/// Document type classification for importance weighting
#[derive(Debug, Clone, Copy)]
enum DocType {
    Readme,
    UserDoc,
    MetaDoc,
    CodeFile,
}

impl DocType {
    fn base_weight(&self) -> f64 {
        match self {
            DocType::Readme => 2.0,
            DocType::UserDoc => 1.8,
            DocType::MetaDoc => 0.6,
            DocType::CodeFile => 1.0,
        }
    }
}

fn classify_document(path: &str) -> DocType {
    if path == "README.md" || path.ends_with("/README.md") {
        DocType::Readme
    } else if path.starts_with("docs/") && path.ends_with(".md") {
        DocType::UserDoc
    } else if path.ends_with(".md") {
        DocType::MetaDoc
    } else {
        DocType::CodeFile
    }
}

/// Compute importance scores for all documents using type-aware algorithm
pub fn compute_pagerank(conn: &Connection) -> Result<HashMap<i64, f64>> {
    let doc_data = get_document_data(conn)?;

    if doc_data.is_empty() {
        return Ok(HashMap::new());
    }

    let incoming_links = build_incoming_links(conn)?;

    let mut scores = HashMap::new();

    for (doc_id, path) in &doc_data {
        let doc_type = classify_document(path);
        let base_weight = doc_type.base_weight();

        let inbound_count = incoming_links.get(doc_id).map(|v| v.len()).unwrap_or(0);
        let inbound_bonus = (inbound_count as f64 * 0.3).min(2.0);

        let final_score = base_weight * (1.0 + inbound_bonus);
        scores.insert(*doc_id, final_score);
    }

    Ok(scores)
}

fn get_document_data(conn: &Connection) -> Result<Vec<(i64, String)>> {
    let mut stmt = conn.prepare("SELECT id, path FROM documents WHERE active = 1")?;
    let docs: Vec<(i64, String)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(docs)
}

fn build_incoming_links(conn: &Connection) -> Result<HashMap<i64, Vec<i64>>> {
    let mut stmt = conn.prepare("SELECT source_id, target_id FROM document_links")?;
    let mut links: HashMap<i64, Vec<i64>> = HashMap::new();

    for row in stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))? {
        let (source, target): (i64, i64) = row?;
        links.entry(target).or_default().push(source);
    }

    Ok(links)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    #[test]
    fn test_pagerank_empty() {
        let db = Database::open_in_memory().unwrap();
        db.initialize().unwrap();

        let scores = compute_pagerank(&db.conn).unwrap();
        assert!(scores.is_empty());
    }

    #[test]
    fn test_pagerank_no_links() {
        let db = Database::open_in_memory().unwrap();
        db.initialize().unwrap();

        db.conn.execute(
            "INSERT INTO content (hash, doc, created_at) VALUES ('hash1', 'doc1', '2024-01-01')",
            [],
        ).unwrap();

        db.conn
            .execute(
                "INSERT INTO documents (collection, path, title, hash, created_at, modified_at) 
             VALUES ('test', 'doc1.md', 'Doc 1', 'hash1', '2024-01-01', '2024-01-01')",
                [],
            )
            .unwrap();

        let scores = compute_pagerank(&db.conn).unwrap();
        assert_eq!(scores.len(), 1);
        let score = scores.values().next().unwrap();
        assert!(*score > 0.0, "Score should be positive");
    }
}
