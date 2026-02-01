//! BM25 full-text search via FTS5

use super::{extract_snippet, parse_metadata_filters, SearchOptions, SearchResult, SearchSource};
use crate::db::{docid_from_hash, Database};
use crate::error::Result;

impl Database {
    /// Perform BM25 full-text search
    pub fn search_fts(&self, query: &str, options: &SearchOptions) -> Result<Vec<SearchResult>> {
        // Parse metadata filters from query or use provided filters
        let (clean_query, mut filters) = parse_metadata_filters(query);

        // Preprocess query for FTS5 compatibility (handle :: and other special chars)
        let clean_query = preprocess_fts_query(&clean_query);

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
                (1.0 / (1.0 + (-1.0 * bm25(documents_fts, 
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
                )))) * COALESCE(d.importance_score, 1.0) as score,
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

                    // Extract snippet from document body
                    let body: String = row.get(6)?;
                    let snippet = extract_snippet(&body, &clean_query, Some(150), None);

                    Ok(SearchResult {
                        filepath: row.get(0)?,
                        display_path: row.get(1)?,
                        title: row.get(2)?,
                        hash: row.get(3)?,
                        collection_name: row.get(4)?,
                        modified_at: row.get(5)?,
                        body: if options.detail.is_full_content() {
                            Some(body)
                        } else {
                            None
                        },
                        body_length: row.get(7)?,
                        docid: docid_from_hash(&row.get::<_, String>(3)?),
                        context: Some(snippet.snippet),
                        score,
                        source: SearchSource::Bm25,
                        chunk_pos: None,
                        llm_summary: row.get(9)?,
                        llm_title: row.get(10)?,
                        llm_keywords: keywords,
                        llm_category: row.get(12)?,
                        llm_difficulty: row.get(13)?,
                        user_metadata,
                        // Chunk fields (not populated for document-level search)
                        is_chunk: false,
                        chunk_hash: None,
                        chunk_type: None,
                        chunk_breadcrumb: None,
                        chunk_start_line: None,
                        chunk_end_line: None,
                        chunk_language: None,
                        chunk_summary: None,
                        chunk_purpose: None,
                        chunk_concepts: Vec::new(),
                        chunk_labels: std::collections::HashMap::new(),
                    })
                },
            )?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        let filtered: Vec<SearchResult> = results
            .into_iter()
            .filter(|r| r.score >= options.min_score)
            .collect();

        Ok(filtered)
    }

    /// Perform BM25 full-text search on chunks (returns SearchResult)
    pub fn search_chunks_bm25(
        &self,
        query: &str,
        options: &SearchOptions,
    ) -> Result<Vec<SearchResult>> {
        let (clean_query, filters) = parse_metadata_filters(query);

        let use_fts = !clean_query.is_empty();
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        let mut sql = if use_fts {
            params_vec.push(Box::new(clean_query.clone()));
            String::from(
                r#"
                SELECT
                    'agentroot://' || d.collection || '/' || d.path as filepath,
                    d.collection || '/' || d.path as display_path,
                    d.title as doc_title,
                    d.hash as doc_hash,
                    d.collection,
                    d.modified_at,
                    ch.content as chunk_content,
                    LENGTH(ch.content) as chunk_length,
                    1.0 / (1.0 + (-1.0 * bm25(chunks_fts,
                        1.0,   -- content
                        5.0,   -- breadcrumb
                        8.0,   -- llm_summary
                        7.0    -- llm_purpose
                    ))) as score,
                    ch.hash as chunk_hash,
                    ch.chunk_type,
                    ch.breadcrumb,
                    ch.start_line,
                    ch.end_line,
                    ch.language,
                    ch.llm_summary as chunk_summary,
                    ch.llm_purpose as chunk_purpose,
                    ch.llm_concepts,
                    ch.llm_labels
                FROM chunks_fts cf
                JOIN chunks ch ON ch.rowid = cf.rowid
                JOIN documents d ON d.hash = ch.document_hash
                JOIN collections coll ON coll.name = d.collection
                WHERE chunks_fts MATCH ?1 AND d.active = 1
            "#,
            )
        } else {
            String::from(
                r#"
                SELECT
                    'agentroot://' || d.collection || '/' || d.path as filepath,
                    d.collection || '/' || d.path as display_path,
                    d.title as doc_title,
                    d.hash as doc_hash,
                    d.collection,
                    d.modified_at,
                    ch.content as chunk_content,
                    LENGTH(ch.content) as chunk_length,
                    0.5 as score,
                    ch.hash as chunk_hash,
                    ch.chunk_type,
                    ch.breadcrumb,
                    ch.start_line,
                    ch.end_line,
                    ch.language,
                    ch.llm_summary as chunk_summary,
                    ch.llm_purpose as chunk_purpose,
                    ch.llm_concepts,
                    ch.llm_labels
                FROM chunks ch
                JOIN documents d ON d.hash = ch.document_hash
                JOIN collections coll ON coll.name = d.collection
                WHERE d.active = 1
            "#,
            )
        };

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

        // Apply chunk-level label filters
        for (field, value) in filters {
            if field != "label" {
                continue;
            }
            // Split label into key:value
            if let Some(colon_pos) = value.find(':') {
                let label_key = &value[..colon_pos];
                let label_value = &value[colon_pos + 1..];
                sql.push_str(&format!(
                    " AND ch.hash IN (SELECT chunk_hash FROM chunk_labels WHERE key = ?{} AND value = ?{})",
                    params_vec.len() + 1,
                    params_vec.len() + 2
                ));
                params_vec.push(Box::new(label_key.to_string()));
                params_vec.push(Box::new(label_value.to_string()));
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
                    let chunk_hash: String = row.get(9)?;
                    let doc_hash: String = row.get(3)?;

                    let concepts_json: Option<String> = row.get(17)?;
                    let concepts = concepts_json
                        .and_then(|json| serde_json::from_str::<Vec<String>>(&json).ok())
                        .unwrap_or_default();

                    let labels_json: Option<String> = row.get(18)?;
                    let labels = labels_json
                        .and_then(|json| {
                            serde_json::from_str::<std::collections::HashMap<String, String>>(&json)
                                .ok()
                        })
                        .unwrap_or_default();

                    // Extract snippet from chunk body
                    let body: String = row.get(6)?;
                    let snippet = extract_snippet(&body, &clean_query, Some(150), None);

                    Ok(SearchResult {
                        filepath: row.get(0)?,
                        display_path: row.get(1)?,
                        title: row.get(2)?,
                        hash: doc_hash.clone(),
                        collection_name: row.get(4)?,
                        modified_at: row.get(5)?,
                        body: if options.detail.is_full_content() {
                            Some(body)
                        } else {
                            None
                        },
                        body_length: row.get(7)?,
                        docid: docid_from_hash(&doc_hash),
                        context: Some(snippet.snippet),
                        score,
                        source: SearchSource::Bm25,
                        chunk_pos: None,
                        llm_summary: None,
                        llm_title: None,
                        llm_keywords: None,
                        llm_category: None,
                        llm_difficulty: None,
                        user_metadata: None,
                        // Chunk fields
                        is_chunk: true,
                        chunk_hash: Some(chunk_hash),
                        chunk_type: row.get(10)?,
                        chunk_breadcrumb: row.get(11)?,
                        chunk_start_line: row.get(12)?,
                        chunk_end_line: row.get(13)?,
                        chunk_language: row.get(14)?,
                        chunk_summary: row.get(15)?,
                        chunk_purpose: row.get(16)?,
                        chunk_concepts: concepts,
                        chunk_labels: labels,
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

/// Preprocess query for FTS5 compatibility
/// Handles special characters that FTS5 can't tokenize properly
fn preprocess_fts_query(query: &str) -> String {
    query
        // Replace :: with space (Rust path separator)
        .replace("::", " ")
        // Replace -> with space (function return type)
        .replace("->", " ")
        // Replace < and > with spaces (generics)
        .replace('<', " ")
        .replace('>', " ")
        // Preserve other characters that FTS5 handles well
        .to_string()
}
