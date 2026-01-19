//! BM25 full-text search via FTS5

use super::{SearchOptions, SearchResult, SearchSource};
use crate::db::{docid_from_hash, Database};
use crate::error::Result;

impl Database {
    /// Perform BM25 full-text search
    pub fn search_fts(&self, query: &str, options: &SearchOptions) -> Result<Vec<SearchResult>> {
        let mut sql = String::from(
            r#"
            SELECT
                'agentroot://' || d.collection || '/' || d.path as filepath,
                d.collection || '/' || d.path as display_path,
                d.title,
                d.hash,
                d.collection,
                d.modified_at,
                c.doc,
                LENGTH(c.doc),
                1.0 / (1.0 + (-1.0 * bm25(documents_fts, 1.0, 10.0, 1.0))) as score,
                d.llm_summary,
                d.llm_title,
                d.llm_keywords,
                d.llm_category,
                d.llm_difficulty
            FROM documents_fts fts
            JOIN documents d ON d.id = fts.rowid
            JOIN content c ON c.hash = d.hash
            JOIN collections coll ON coll.name = d.collection
            WHERE documents_fts MATCH ?1 AND d.active = 1
        "#,
        );

        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(query.to_string())];

        if let Some(ref coll) = options.collection {
            sql.push_str(" AND d.collection = ?");
            sql.push_str(&(params_vec.len() + 1).to_string());
            params_vec.push(Box::new(coll.clone()));
        }

        if let Some(ref provider) = options.provider {
            sql.push_str(" AND coll.provider_type = ?");
            sql.push_str(&(params_vec.len() + 1).to_string());
            params_vec.push(Box::new(provider.clone()));
        }

        sql.push_str(" ORDER BY score DESC");

        if options.limit > 0 {
            sql.push_str(&format!(" LIMIT {}", options.limit));
        }

        let mut stmt = self.conn.prepare(&sql)?;
        let results = stmt
            .query_map(
                rusqlite::params_from_iter(params_vec.iter().map(|p| p.as_ref())),
                |row| {
                    let score: f64 = row.get(8)?;
                    let keywords_json: Option<String> = row.get(11)?;
                    let keywords = keywords_json
                        .and_then(|json| serde_json::from_str::<Vec<String>>(&json).ok());

                    Ok(SearchResult {
                        filepath: row.get(0)?,
                        display_path: row.get(1)?,
                        title: row.get(2)?,
                        hash: row.get(3)?,
                        collection_name: row.get(4)?,
                        modified_at: row.get(5)?,
                        body: if options.full_content {
                            Some(row.get(6)?)
                        } else {
                            None
                        },
                        body_length: row.get(7)?,
                        docid: docid_from_hash(&row.get::<_, String>(3)?),
                        context: None,
                        score,
                        source: SearchSource::Bm25,
                        chunk_pos: None,
                        llm_summary: row.get(9)?,
                        llm_title: row.get(10)?,
                        llm_keywords: keywords,
                        llm_category: row.get(12)?,
                        llm_difficulty: row.get(13)?,
                    })
                },
            )?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        // Filter by min_score
        let filtered: Vec<SearchResult> = results
            .into_iter()
            .filter(|r| r.score >= options.min_score)
            .collect();

        Ok(filtered)
    }
}
