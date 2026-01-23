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
            if let Some(result) = self.get_search_result_for_hash_seq(&hash_seq, score, query, options)? {
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
        let mut filtered: Vec<SearchResult> = final_results
            .into_iter()
            .filter(|r| r.score >= options.min_score)
            .take(options.limit)
            .collect();

        // Normalize scores relative to top result for better differentiation
        // Top result = 100%, others proportional
        if let Some(top_score) = filtered.first().map(|r| r.score) {
            if top_score > 0.0 {
                for result in &mut filtered {
                    result.score = (result.score / top_score) * 100.0;
                }
            }
        }

        Ok(filtered)
    }

    /// Get search result for a hash_seq
    fn get_search_result_for_hash_seq(
        &self,
        hash_seq: &str,
        score: f32,
        query: &str,
        options: &SearchOptions,
    ) -> Result<Option<SearchResult>> {
        // Parse hash_seq (format: "hash_seq")
        let parts: Vec<&str> = hash_seq.rsplitn(2, '_').collect();
        if parts.len() != 2 {
            return Ok(None);
        }
        let hash = parts[1];

        // Build SQL with metadata filters
        let mut sql = String::from(
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
                d.llm_difficulty,
                d.user_metadata,
                COALESCE(d.importance_score, 1.0) as importance_score,
                d.path
             FROM documents d
             JOIN content c ON c.hash = d.hash
             JOIN content_vectors cv ON cv.hash = d.hash
             WHERE d.hash = ?1 AND d.active = 1",
        );

        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(hash.to_string())];

        // Apply metadata filters
        for (field, value) in &options.metadata_filters {
            match field.as_str() {
                "category" => {
                    sql.push_str(&format!(" AND d.llm_category = ?{}", params_vec.len() + 1));
                    params_vec.push(Box::new(value.clone()));
                }
                "difficulty" => {
                    sql.push_str(&format!(
                        " AND d.llm_difficulty = ?{}",
                        params_vec.len() + 1
                    ));
                    params_vec.push(Box::new(value.clone()));
                }
                "tag" | "keyword" => {
                    sql.push_str(&format!(
                        " AND d.llm_keywords LIKE ?{}",
                        params_vec.len() + 1
                    ));
                    params_vec.push(Box::new(format!("%{}%", value)));
                }
                _ => {}
            }
        }

        sql.push_str(" LIMIT 1");

        let result = self.conn.query_row(
            &sql,
            rusqlite::params_from_iter(params_vec.iter().map(|p| p.as_ref())),
            |row| {
                let keywords_json: Option<String> = row.get(11)?;
                let keywords =
                    keywords_json.and_then(|json| serde_json::from_str::<Vec<String>>(&json).ok());

                let user_metadata_json: Option<String> = row.get(14)?;
                let user_metadata = user_metadata_json
                    .and_then(|json| crate::db::UserMetadata::from_json(&json).ok());

                // Get importance score and path for boosting
                let importance_score: f64 = row.get(15)?;
                let path: String = row.get(16)?;
                let collection_name: String = row.get(4)?;

                // Apply importance boost (like BM25 does)
                let mut boosted_score = score as f64 * importance_score;

                // Collection boost: prefer documentation collections over source code
                // agentroot (docs) > agentroot-src (source code with tests)
                if collection_name == "agentroot" {
                    boosted_score *= 1.5; // Boost documentation collection
                } else if collection_name.contains("-src") {
                    boosted_score *= 0.7; // Demote source code collections
                }

                // Path-based demotion: heavily penalize test files
                if path.contains("/tests/") || path.contains("/test/") {
                    boosted_score *= 0.1; // 90% penalty for test files
                }

                // Title/filename boost: strongly prefer documents with query terms in title/path
                // This helps exact keyword queries (e.g., "mcp" should rank "mcp-server.md" highly)
                let title: String = row.get(2)?;
                let query_lower = query.to_lowercase();
                let title_lower = title.to_lowercase();
                let path_lower = path.to_lowercase();
                
                // Extract query terms (split on whitespace and common delimiters)
                let query_terms: Vec<&str> = query_lower
                    .split(|c: char| c.is_whitespace() || c == '?' || c == '!')
                    .filter(|s| !s.is_empty() && s.len() >= 2) // Keep acronyms and short terms
                    .collect();
                
                // Check for title/filename matches with graduated boosting
                let mut title_boost = 1.0;
                for term in &query_terms {
                    // Extra strong boost if term appears in filename (path)
                    if path_lower.contains(term) {
                        title_boost *= 10.0; // VERY strong boost for filename match
                        break; // One match is enough for max boost
                    }
                    // Strong boost if term appears in title  
                    else if title_lower.contains(term) {
                        title_boost *= 4.0; // Strong boost for title match
                    }
                }
                
                boosted_score *= title_boost;

                Ok(SearchResult {
                    filepath: row.get(0)?,
                    display_path: row.get(1)?,
                    title: row.get(2)?,
                    hash: row.get(3)?,
                    collection_name,
                    modified_at: row.get(5)?,
                    body: if options.full_content {
                        Some(row.get(6)?)
                    } else {
                        None
                    },
                    body_length: row.get(7)?,
                    docid: docid_from_hash(&row.get::<_, String>(3)?),
                    context: None,
                    score: boosted_score,
                    source: SearchSource::Vector,
                    chunk_pos: Some(row.get(8)?),
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
        );

        match result {
            Ok(r) => Ok(Some(r)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Perform vector similarity search on chunks
    pub async fn search_chunks_vec(
        &self,
        query: &str,
        embedder: &dyn Embedder,
        options: &SearchOptions,
    ) -> Result<Vec<SearchResult>> {
        // Get query embedding
        let query_embedding = embedder.embed(&format_query_for_embedding(query)).await?;

        // Get all chunk embeddings (optionally filtered by collection)
        let chunk_embeddings = if let Some(ref coll) = options.collection {
            self.get_chunk_embeddings_for_collection(coll, embedder.model_name())?
        } else {
            self.get_all_chunk_embeddings(embedder.model_name())?
        };

        if chunk_embeddings.is_empty() {
            return Ok(Vec::new());
        }

        // Compute similarities
        let mut similarities: Vec<(String, f32)> = chunk_embeddings
            .iter()
            .map(|(chunk_hash, embedding)| {
                let sim = cosine_similarity(&query_embedding, embedding);
                (chunk_hash.clone(), sim)
            })
            .collect();

        // Sort by similarity (descending)
        similarities.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Take top candidates
        let top_candidates: Vec<_> = similarities.into_iter().take(options.limit).collect();

        // Fetch chunk details for top candidates
        let mut results = Vec::new();
        for (chunk_hash, score) in top_candidates {
            if score < options.min_score as f32 {
                continue;
            }

            if let Some(result) = self.get_chunk_search_result(&chunk_hash, score, options)? {
                results.push(result);
            }
        }

        Ok(results)
    }

    /// Get search result for a chunk
    fn get_chunk_search_result(
        &self,
        chunk_hash: &str,
        score: f32,
        options: &SearchOptions,
    ) -> Result<Option<SearchResult>> {
        let mut sql = String::from(
            "SELECT
                'agentroot://' || d.collection || '/' || d.path as filepath,
                d.collection || '/' || d.path as display_path,
                d.title as doc_title,
                d.hash as doc_hash,
                d.collection,
                d.modified_at,
                ch.content as chunk_content,
                LENGTH(ch.content) as chunk_length,
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
             WHERE ch.hash = ?1 AND d.active = 1",
        );

        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(chunk_hash.to_string())];

        if let Some(ref coll) = options.collection {
            sql.push_str(" AND d.collection = ?");
            sql.push_str(&(params_vec.len() + 1).to_string());
            params_vec.push(Box::new(coll.clone()));
        }

        sql.push_str(" LIMIT 1");

        let result = self.conn.query_row(
            &sql,
            rusqlite::params_from_iter(params_vec.iter().map(|p| p.as_ref())),
            |row| {
                let doc_hash: String = row.get(3)?;
                let chunk_hash: String = row.get(8)?;

                let concepts_json: Option<String> = row.get(16)?;
                let concepts = concepts_json
                    .and_then(|json| serde_json::from_str::<Vec<String>>(&json).ok())
                    .unwrap_or_default();

                let labels_json: Option<String> = row.get(17)?;
                let labels = labels_json
                    .and_then(|json| serde_json::from_str::<std::collections::HashMap<String, String>>(&json).ok())
                    .unwrap_or_default();

                Ok(SearchResult {
                    filepath: row.get(0)?,
                    display_path: row.get(1)?,
                    title: row.get(2)?,
                    hash: doc_hash.clone(),
                    collection_name: row.get(4)?,
                    modified_at: row.get(5)?,
                    body: if options.full_content {
                        Some(row.get(6)?)
                    } else {
                        None
                    },
                    body_length: row.get(7)?,
                    docid: docid_from_hash(&doc_hash),
                    context: None,
                    score: score as f64,
                    source: SearchSource::Vector,
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
        );

        match result {
            Ok(r) => Ok(Some(r)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Get chunk embeddings for a collection
    fn get_chunk_embeddings_for_collection(
        &self,
        collection: &str,
        model: &str,
    ) -> Result<Vec<(String, Vec<f32>)>> {
        let mut stmt = self.conn.prepare(
            "SELECT ce.chunk_hash, ce.embedding
             FROM chunk_embeddings ce
             JOIN chunks ch ON ch.hash = ce.chunk_hash
             JOIN documents d ON d.hash = ch.document_hash
             WHERE d.collection = ?1 AND d.active = 1 AND ce.model = ?2",
        )?;

        let results = stmt
            .query_map(rusqlite::params![collection, model], |row| {
                let chunk_hash: String = row.get(0)?;
                let embedding_blob: Vec<u8> = row.get(1)?;
                let embedding = crate::db::vectors::bytes_to_embedding(&embedding_blob);
                Ok((chunk_hash, embedding))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(results)
    }

    /// Get all chunk embeddings
    fn get_all_chunk_embeddings(&self, model: &str) -> Result<Vec<(String, Vec<f32>)>> {
        let mut stmt = self.conn.prepare(
            "SELECT ce.chunk_hash, ce.embedding
             FROM chunk_embeddings ce
             JOIN chunks ch ON ch.hash = ce.chunk_hash
             JOIN documents d ON d.hash = ch.document_hash
             WHERE d.active = 1 AND ce.model = ?1",
        )?;

        let results = stmt
            .query_map(rusqlite::params![model], |row| {
                let chunk_hash: String = row.get(0)?;
                let embedding_blob: Vec<u8> = row.get(1)?;
                let embedding = crate::db::vectors::bytes_to_embedding(&embedding_blob);
                Ok((chunk_hash, embedding))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(results)
    }
}

/// Format query for embedding (matches document format)
fn format_query_for_embedding(query: &str) -> String {
    format!("search_query: {}", query)
}
