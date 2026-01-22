//! BM25 full-text search via FTS5

use super::{parse_metadata_filters, SearchOptions, SearchResult, SearchSource};
use crate::db::{docid_from_hash, Database};
use crate::error::Result;

impl Database {
    /// Perform BM25 full-text search
    pub fn search_fts(&self, query: &str, options: &SearchOptions) -> Result<Vec<SearchResult>> {
        // Parse metadata filters from query or use provided filters
        let (clean_query, mut filters) = parse_metadata_filters(query);

        // Merge with filters from options (options take precedence)
        filters.extend(options.metadata_filters.clone());

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
                1.0 / (1.0 + (-1.0 * bm25(documents_fts, 
                    1.0,   -- filepath
                    10.0,  -- title
                    5.0,   -- body
                    8.0,   -- llm_summary (high weight)
                    10.0,  -- llm_title (high weight)
                    15.0,  -- llm_keywords (very high weight)
                    7.0,   -- llm_intent (high weight)
                    12.0,  -- llm_concepts (very high weight)
                    20.0,  -- user_metadata (highest weight)
                    0.1    -- modified_at (very low)
                ))) as score,
                d.llm_summary,
                d.llm_title,
                d.llm_keywords,
                d.llm_category,
                d.llm_difficulty,
                d.user_metadata
            FROM documents_fts fts
            JOIN documents d ON d.id = fts.rowid
            JOIN content c ON c.hash = d.hash
            JOIN collections coll ON coll.name = d.collection
            WHERE documents_fts MATCH ?1 AND d.active = 1
        "#,
        );

        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(clean_query.clone())];

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

        // Apply metadata filters
        for (field, value) in filters {
            match field.as_str() {
                "category" => {
                    sql.push_str(&format!(" AND d.llm_category = ?{}", params_vec.len() + 1));
                    params_vec.push(Box::new(value));
                }
                "difficulty" => {
                    sql.push_str(&format!(
                        " AND d.llm_difficulty = ?{}",
                        params_vec.len() + 1
                    ));
                    params_vec.push(Box::new(value));
                }
                "tag" | "keyword" => {
                    sql.push_str(&format!(
                        " AND d.llm_keywords LIKE ?{}",
                        params_vec.len() + 1
                    ));
                    params_vec.push(Box::new(format!("%{}%", value)));
                }
                _ => {} // Ignore unknown filters
            }
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

                    let user_metadata_json: Option<String> = row.get(14)?;
                    let user_metadata = user_metadata_json
                        .and_then(|json| crate::db::UserMetadata::from_json(&json).ok());

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
                        user_metadata,
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
