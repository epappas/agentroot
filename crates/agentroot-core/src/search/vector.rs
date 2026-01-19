//! Vector similarity search
//!
//! Computes cosine similarity between query embedding and stored embeddings.

use super::{SearchOptions, SearchResult, SearchSource};
use crate::db::vectors::cosine_similarity;
use crate::db::{docid_from_hash, Database};
use crate::error::Result;
use crate::llm::Embedder;
use std::collections::HashMap;

impl Database {
    /// Perform vector similarity search
    pub async fn search_vec(
        &self,
        query: &str,
        embedder: &dyn Embedder,
        options: &SearchOptions,
    ) -> Result<Vec<SearchResult>> {
        // Get query embedding
        let query_embedding = embedder.embed(&format_query_for_embedding(query)).await?;

        // Get all stored embeddings (optionally filtered by collection)
        let stored_embeddings = if let Some(ref coll) = options.collection {
            self.get_embeddings_for_collection(coll)?
        } else {
            self.get_all_embeddings()?
        };

        // Compute similarities
        let mut similarities: Vec<(String, f32)> = stored_embeddings
            .iter()
            .map(|(hash_seq, embedding)| {
                let sim = cosine_similarity(&query_embedding, embedding);
                (hash_seq.clone(), sim)
            })
            .collect();

        // Sort by similarity (descending)
        similarities.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Take top candidates (3x limit for deduplication)
        let fetch_limit = options.limit * 3;
        let top_candidates: Vec<_> = similarities.into_iter().take(fetch_limit).collect();

        // Fetch document details for top candidates
        let mut results = Vec::new();
        for (hash_seq, score) in top_candidates {
            if let Some(result) = self.get_search_result_for_hash_seq(&hash_seq, score, options)? {
                results.push(result);
            }
        }

        // Deduplicate: keep best chunk per document
        let mut best_by_hash: HashMap<String, SearchResult> = HashMap::new();
        for result in results {
            let existing = best_by_hash.get(&result.hash);
            if existing.is_none() || existing.unwrap().score < result.score {
                best_by_hash.insert(result.hash.clone(), result);
            }
        }

        let mut final_results: Vec<SearchResult> = best_by_hash.into_values().collect();
        final_results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Filter by min_score and limit
        let filtered: Vec<SearchResult> = final_results
            .into_iter()
            .filter(|r| r.score >= options.min_score)
            .take(options.limit)
            .collect();

        Ok(filtered)
    }

    /// Get search result for a hash_seq
    fn get_search_result_for_hash_seq(
        &self,
        hash_seq: &str,
        score: f32,
        options: &SearchOptions,
    ) -> Result<Option<SearchResult>> {
        // Parse hash_seq (format: "hash_seq")
        let parts: Vec<&str> = hash_seq.rsplitn(2, '_').collect();
        if parts.len() != 2 {
            return Ok(None);
        }
        let hash = parts[1];

        let result = self.conn.query_row(
            "SELECT
                'agentroot://' || d.collection || '/' || d.path as filepath,
                d.collection || '/' || d.path as display_path,
                d.title,
                d.hash,
                d.collection,
                d.modified_at,
                c.doc,
                LENGTH(c.doc),
                cv.pos,
                d.llm_summary,
                d.llm_title,
                d.llm_keywords,
                d.llm_category,
                d.llm_difficulty
             FROM documents d
             JOIN content c ON c.hash = d.hash
             JOIN content_vectors cv ON cv.hash = d.hash
             WHERE d.hash = ?1 AND d.active = 1
             LIMIT 1",
            rusqlite::params![hash],
            |row| {
                let keywords_json: Option<String> = row.get(11)?;
                let keywords =
                    keywords_json.and_then(|json| serde_json::from_str::<Vec<String>>(&json).ok());

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
                    score: score as f64,
                    source: SearchSource::Vector,
                    chunk_pos: Some(row.get(8)?),
                    llm_summary: row.get(9)?,
                    llm_title: row.get(10)?,
                    llm_keywords: keywords,
                    llm_category: row.get(12)?,
                    llm_difficulty: row.get(13)?,
                })
            },
        );

        match result {
            Ok(r) => Ok(Some(r)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
}

/// Format query for embedding (matches document format)
fn format_query_for_embedding(query: &str) -> String {
    format!("search_query: {}", query)
}
