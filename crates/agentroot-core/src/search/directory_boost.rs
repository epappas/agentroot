//! Directory-aware structural boosting for search results

use super::SearchResult;

/// Boost results that share a directory with top-scoring results.
/// Applied as post-processing after initial scoring.
pub fn apply_directory_boost(results: &mut [SearchResult]) {
    if results.len() < 2 {
        return;
    }

    let top_dirs: Vec<String> = results
        .iter()
        .take(3)
        .filter_map(|r| parent_dir(&r.filepath))
        .collect();

    if top_dirs.is_empty() {
        return;
    }

    let mut boosted = false;
    for result in results.iter_mut().skip(3) {
        if let Some(dir) = parent_dir(&result.filepath) {
            if top_dirs.contains(&dir) {
                result.score = (result.score * 1.15).min(1.0);
                boosted = true;
            }
        }
    }

    if boosted {
        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }
}

fn parent_dir(filepath: &str) -> Option<String> {
    filepath.rsplit_once('/').map(|(dir, _)| dir.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::{SearchResult, SearchSource};

    fn make_result(filepath: &str, score: f64) -> SearchResult {
        SearchResult {
            filepath: filepath.to_string(),
            display_path: filepath.to_string(),
            title: "".to_string(),
            hash: "".to_string(),
            collection_name: "".to_string(),
            modified_at: "".to_string(),
            body: None,
            body_length: 0,
            docid: "".to_string(),
            context: None,
            score,
            source: SearchSource::Bm25,
            chunk_pos: None,
            llm_summary: None,
            llm_title: None,
            llm_keywords: None,
            llm_category: None,
            llm_difficulty: None,
            user_metadata: None,
            is_chunk: false,
            chunk_hash: None,
            chunk_type: None,
            chunk_breadcrumb: None,
            chunk_start_line: None,
            chunk_end_line: None,
            chunk_language: None,
            chunk_summary: None,
            chunk_purpose: None,
            chunk_concepts: vec![],
            chunk_labels: std::collections::HashMap::new(),
        }
    }

    #[test]
    fn test_directory_boost_applied() {
        let mut results = vec![
            make_result("agentroot://test/src/auth/login.rs", 0.9),
            make_result("agentroot://test/src/auth/jwt.rs", 0.85),
            make_result("agentroot://test/src/db/query.rs", 0.8),
            make_result("agentroot://test/src/auth/session.rs", 0.5),
            make_result("agentroot://test/src/utils/logger.rs", 0.4),
        ];

        apply_directory_boost(&mut results);

        // Boosted result should have moved up in ranking
        let auth_session = results
            .iter()
            .find(|r| r.filepath.contains("session.rs"))
            .unwrap();
        assert!(auth_session.score > 0.5);
        // Non-matching result unchanged
        let logger = results
            .iter()
            .find(|r| r.filepath.contains("logger.rs"))
            .unwrap();
        assert!((logger.score - 0.4).abs() < 0.001);
    }

    #[test]
    fn test_directory_boost_capped_at_one() {
        let mut results = vec![
            make_result("agentroot://test/src/auth/a.rs", 0.95),
            make_result("agentroot://test/src/auth/b.rs", 0.92),
            make_result("agentroot://test/src/db/c.rs", 0.8),
            make_result("agentroot://test/src/auth/d.rs", 0.9),
        ];

        apply_directory_boost(&mut results);

        for r in &results {
            assert!(r.score <= 1.0, "score {} exceeds 1.0", r.score);
        }
    }

    #[test]
    fn test_directory_boost_resorts() {
        let mut results = vec![
            make_result("agentroot://test/src/auth/a.rs", 0.9),
            make_result("agentroot://test/src/auth/b.rs", 0.85),
            make_result("agentroot://test/src/db/c.rs", 0.8),
            make_result("agentroot://test/src/auth/d.rs", 0.75),
            make_result("agentroot://test/src/other/e.rs", 0.78),
        ];

        apply_directory_boost(&mut results);

        // Results must be sorted descending by score
        for window in results.windows(2) {
            assert!(
                window[0].score >= window[1].score,
                "not sorted: {} >= {}",
                window[0].score,
                window[1].score
            );
        }
    }

    #[test]
    fn test_directory_boost_too_few_results() {
        let mut results = vec![make_result("agentroot://test/a.rs", 0.9)];
        apply_directory_boost(&mut results);
        assert!((results[0].score - 0.9).abs() < 0.001);
    }
}
