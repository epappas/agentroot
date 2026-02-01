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
