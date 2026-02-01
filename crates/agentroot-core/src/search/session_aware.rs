//! Session-aware search post-processing

use super::SearchResult;
use crate::db::Database;
use crate::error::Result;

/// Demote results the agent has already seen in this session.
/// Seen results get score *= 0.3 (demoted, not removed).
pub fn apply_session_awareness(
    db: &Database,
    results: &mut Vec<SearchResult>,
    session_id: &str,
) -> Result<()> {
    let seen = db.get_seen_hashes(session_id)?;
    if seen.is_empty() {
        return Ok(());
    }

    for result in results.iter_mut() {
        let hash = result.chunk_hash.as_deref().unwrap_or(&result.hash);
        if seen.contains(hash) {
            result.score *= 0.3;
        }
    }

    results.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    Ok(())
}

/// Log search results to session and mark top results as seen.
pub fn log_session_results(
    db: &Database,
    session_id: &str,
    query: &str,
    results: &[SearchResult],
    detail_level: &str,
) -> Result<()> {
    db.touch_session(session_id)?;
    db.log_session_query(session_id, query, results)?;

    for result in results.iter().take(10) {
        let chunk_hash = result.chunk_hash.as_deref();
        db.mark_seen(session_id, &result.hash, chunk_hash, detail_level)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::SearchSource;

    fn make_result(hash: &str, score: f64) -> SearchResult {
        SearchResult {
            filepath: format!("agentroot://test/{}.rs", hash),
            display_path: format!("test/{}.rs", hash),
            title: hash.to_string(),
            hash: hash.to_string(),
            collection_name: "test".to_string(),
            modified_at: "".to_string(),
            body: None,
            body_length: 0,
            docid: hash[..6.min(hash.len())].to_string(),
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
    fn test_apply_session_awareness_demotes_seen() {
        let db = Database::open_in_memory().unwrap();
        db.initialize().unwrap();

        let sid = db.create_session(Some(3600)).unwrap();
        db.mark_seen(&sid, "hash_a", None, "L1").unwrap();

        let mut results = vec![make_result("hash_a", 0.9), make_result("hash_b", 0.5)];

        apply_session_awareness(&db, &mut results, &sid).unwrap();

        let a = results.iter().find(|r| r.hash == "hash_a").unwrap();
        assert!((a.score - 0.27).abs() < 0.001);

        let b = results.iter().find(|r| r.hash == "hash_b").unwrap();
        assert!((b.score - 0.5).abs() < 0.001);

        // Re-sorted: hash_b (0.5) before hash_a (0.27)
        assert_eq!(results[0].hash, "hash_b");
        assert_eq!(results[1].hash, "hash_a");
    }

    #[test]
    fn test_apply_session_awareness_no_seen() {
        let db = Database::open_in_memory().unwrap();
        db.initialize().unwrap();

        let sid = db.create_session(Some(3600)).unwrap();

        let mut results = vec![make_result("hash_a", 0.9), make_result("hash_b", 0.5)];

        apply_session_awareness(&db, &mut results, &sid).unwrap();

        assert!((results[0].score - 0.9).abs() < 0.001);
        assert!((results[1].score - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_apply_session_awareness_uses_chunk_hash() {
        let db = Database::open_in_memory().unwrap();
        db.initialize().unwrap();

        let sid = db.create_session(Some(3600)).unwrap();
        db.mark_seen(&sid, "doc_hash", Some("chunk_abc"), "L1")
            .unwrap();

        let mut r = make_result("doc_hash", 0.8);
        r.chunk_hash = Some("chunk_abc".to_string());

        let mut results = vec![r];
        apply_session_awareness(&db, &mut results, &sid).unwrap();

        assert!((results[0].score - 0.24).abs() < 0.001);
    }

    #[test]
    fn test_log_session_results() {
        let db = Database::open_in_memory().unwrap();
        db.initialize().unwrap();

        let sid = db.create_session(Some(3600)).unwrap();

        let results: Vec<SearchResult> = (0..12)
            .map(|i| make_result(&format!("hash_{:02}", i), 0.9 - i as f64 * 0.05))
            .collect();

        log_session_results(&db, &sid, "test query", &results, "L1").unwrap();

        let queries = db.get_session_queries(&sid).unwrap();
        assert_eq!(queries.len(), 1);
        assert_eq!(queries[0].query, "test query");
        assert_eq!(queries[0].result_count, 12);

        // Only top 10 marked as seen
        let seen = db.get_seen_hashes(&sid).unwrap();
        assert!(seen.contains("hash_00"));
        assert!(seen.contains("hash_09"));
        assert!(!seen.contains("hash_10"));
        assert!(!seen.contains("hash_11"));
    }
}
