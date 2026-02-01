//! Search suggestion computation for agentic workflows

use super::SearchResult;
use crate::db::Database;
use crate::error::Result;
use std::collections::HashSet;

#[derive(Debug, Clone, serde::Serialize)]
pub struct SearchSuggestions {
    pub related_directories: Vec<String>,
    pub related_concepts: Vec<String>,
    pub refinement_queries: Vec<String>,
    pub unseen_related: usize,
}

pub fn compute_suggestions(
    db: &Database,
    results: &[SearchResult],
    query: &str,
    session_id: Option<&str>,
) -> Result<SearchSuggestions> {
    let related_directories: Vec<String> = results
        .iter()
        .filter_map(|r| r.filepath.rsplit_once('/').map(|(dir, _)| dir.to_string()))
        .collect::<HashSet<_>>()
        .into_iter()
        .take(5)
        .collect();

    let related_concepts: Vec<String> = results
        .iter()
        .filter_map(|r| r.llm_keywords.as_ref())
        .flatten()
        .cloned()
        .collect::<HashSet<_>>()
        .into_iter()
        .take(10)
        .collect();

    let refinement_queries = generate_refinements(query, &related_concepts);

    let unseen_related = match session_id {
        Some(sid) => count_unseen_in_directories(db, sid, &related_directories)?,
        None => 0,
    };

    Ok(SearchSuggestions {
        related_directories,
        related_concepts,
        refinement_queries,
        unseen_related,
    })
}

fn generate_refinements(query: &str, concepts: &[String]) -> Vec<String> {
    concepts
        .iter()
        .take(3)
        .filter(|c| !query.to_lowercase().contains(&c.to_lowercase()))
        .map(|c| format!("{} {}", query, c))
        .collect()
}

fn count_unseen_in_directories(
    db: &Database,
    session_id: &str,
    directories: &[String],
) -> Result<usize> {
    let seen = db.get_seen_hashes(session_id)?;
    let mut count = 0;

    for dir in directories {
        // Strip agentroot:// prefix to get collection/path pattern
        let pattern = dir.strip_prefix("agentroot://").unwrap_or(dir);

        let docs = db.find_documents_by_path_prefix(pattern)?;
        for doc_hash in docs {
            if !seen.contains(&doc_hash) {
                count += 1;
            }
        }
    }

    Ok(count)
}
