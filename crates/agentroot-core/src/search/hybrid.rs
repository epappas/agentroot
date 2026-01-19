//! Hybrid search with Reciprocal Rank Fusion

use std::collections::HashMap;
use crate::db::Database;
use crate::llm::{Embedder, QueryExpander, Reranker, RerankDocument};
use crate::error::Result;
use super::{SearchOptions, SearchResult, SearchSource};

/// RRF constant (standard value)
const RRF_K: f64 = 60.0;

/// Maximum documents to send to reranker
const MAX_RERANK_DOCS: usize = 40;

/// Strong signal threshold
const STRONG_SIGNAL_SCORE: f64 = 0.85;
const STRONG_SIGNAL_GAP: f64 = 0.15;

/// Check if top BM25 result is a strong signal (skip expansion)
pub fn has_strong_signal(results: &[SearchResult]) -> bool {
    if results.len() < 2 {
        return results.first().map(|r| r.score >= STRONG_SIGNAL_SCORE).unwrap_or(false);
    }

    let top_score = results[0].score;
    let second_score = results[1].score;
    let gap = top_score - second_score;

    top_score >= STRONG_SIGNAL_SCORE && gap >= STRONG_SIGNAL_GAP
}

/// Cap results for reranking
pub fn cap_for_reranking(results: Vec<SearchResult>) -> Vec<SearchResult> {
    results.into_iter().take(MAX_RERANK_DOCS).collect()
}

/// Position-aware score blending
pub fn blend_scores(rrf_rank: usize, rrf_score: f64, rerank_score: f64) -> f64 {
    let rrf_weight = if rrf_rank <= 3 {
        0.75  // Trust retrieval for top results
    } else if rrf_rank <= 10 {
        0.60
    } else {
        0.40  // Trust reranker for lower-ranked
    };

    rrf_weight * rrf_score + (1.0 - rrf_weight) * rerank_score
}

/// Reciprocal Rank Fusion
pub fn rrf_fusion(
    bm25_results: &[SearchResult],
    vec_results: &[SearchResult],
) -> Vec<SearchResult> {
    let mut scores: HashMap<String, (f64, SearchResult)> = HashMap::new();

    // Process BM25 results (weight 2x)
    for (rank, result) in bm25_results.iter().enumerate() {
        let rrf_score = 2.0 / (RRF_K + (rank + 1) as f64);
        // Bonus for appearing in top 3
        let bonus = if rank < 3 { 0.05 } else if rank < 10 { 0.02 } else { 0.0 };

        let entry = scores.entry(result.hash.clone()).or_insert((0.0, result.clone()));
        entry.0 += rrf_score + bonus;
    }

    // Process vector results
    for (rank, result) in vec_results.iter().enumerate() {
        let rrf_score = 1.0 / (RRF_K + (rank + 1) as f64);
        let bonus = if rank < 3 { 0.05 } else if rank < 10 { 0.02 } else { 0.0 };

        let entry = scores.entry(result.hash.clone()).or_insert((0.0, result.clone()));
        entry.0 += rrf_score + bonus;
    }

    // Sort by score
    let mut results: Vec<(f64, SearchResult)> = scores.into_values().collect();
    results.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    results.into_iter().map(|(score, mut r)| {
        r.score = score;
        r.source = SearchSource::Hybrid;
        r
    }).collect()
}

/// Full hybrid search pipeline
pub async fn hybrid_search(
    db: &Database,
    query: &str,
    options: &SearchOptions,
    embedder: &dyn Embedder,
    expander: Option<&dyn QueryExpander>,
    reranker: Option<&dyn Reranker>,
) -> Result<Vec<SearchResult>> {
    // 1. Initial BM25 search
    let bm25_results = db.search_fts(query, options)?;

    // 2. Check for strong signal
    if has_strong_signal(&bm25_results) {
        return Ok(bm25_results);
    }

    // 3. Vector search
    let vec_results = db.search_vec(query, embedder, options).await?;

    // 4. Query expansion (if available and not skipped)
    let mut all_bm25 = bm25_results.clone();
    let mut all_vec = vec_results.clone();

    if let Some(exp) = expander {
        let expanded = exp.expand(query, None).await?;

        // Run lexical variations
        for lex_query in &expanded.lexical {
            let results = db.search_fts(lex_query, options)?;
            all_bm25.extend(results);
        }

        // Run semantic variations
        for vec_query in &expanded.semantic {
            let results = db.search_vec(vec_query, embedder, options).await?;
            all_vec.extend(results);
        }

        // Run HyDE if present
        if let Some(ref hyde) = expanded.hyde {
            let results = db.search_vec(hyde, embedder, options).await?;
            all_vec.extend(results);
        }
    }

    // 5. RRF fusion
    let mut fused = rrf_fusion(&all_bm25, &all_vec);

    // 6. Cap for reranking
    fused = cap_for_reranking(fused);

    // 7. Rerank (if available)
    if let Some(rr) = reranker {
        let docs: Vec<RerankDocument> = fused.iter().map(|r| RerankDocument {
            id: r.hash.clone(),
            text: r.body.clone().unwrap_or_default(),
        }).collect();

        let reranked = rr.rerank(query, &docs).await?;

        // Build hash -> rerank score map
        let rerank_scores: HashMap<String, f64> = reranked.iter()
            .map(|r| (r.id.clone(), r.score))
            .collect();

        // Blend scores
        for (rrf_rank, result) in fused.iter_mut().enumerate() {
            if let Some(&rerank_score) = rerank_scores.get(&result.hash) {
                let rrf_score = result.score;
                result.score = blend_scores(rrf_rank + 1, rrf_score, rerank_score);
            }
        }

        // Re-sort by blended score
        fused.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    }

    // 8. Apply final limit and min_score
    let final_results: Vec<SearchResult> = fused
        .into_iter()
        .filter(|r| r.score >= options.min_score)
        .take(options.limit)
        .collect();

    Ok(final_results)
}

impl Database {
    /// Synchronous vector search (placeholder for CLI - needs runtime)
    pub fn search_vec_sync(&self, _query: &str, options: &SearchOptions) -> Result<Vec<SearchResult>> {
        // Placeholder: In production, this would use a runtime
        // For now, fall back to BM25
        eprintln!("Warning: Vector search requires embeddings, falling back to BM25");
        self.search_fts(_query, options)
    }

    /// Synchronous hybrid search (placeholder for CLI - needs runtime)
    pub fn search_hybrid_sync(&self, query: &str, options: &SearchOptions) -> Result<Vec<SearchResult>> {
        // Placeholder: In production, this would use full hybrid pipeline
        // For now, just run BM25
        self.search_fts(query, options)
    }
}
