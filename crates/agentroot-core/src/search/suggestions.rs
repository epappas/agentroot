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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::SearchSource;

    fn make_result(filepath: &str, keywords: Option<Vec<&str>>) -> SearchResult {
        SearchResult {
            filepath: filepath.to_string(),
            display_path: filepath.to_string(),
            title: "".to_string(),
            hash: "".to_string(),
            collection_name: "test".to_string(),
            modified_at: "".to_string(),
            body: None,
            body_length: 0,
            docid: "".to_string(),
            context: None,
            score: 0.5,
            source: SearchSource::Bm25,
            chunk_pos: None,
            llm_summary: None,
            llm_title: None,
            llm_keywords: keywords.map(|v| v.into_iter().map(String::from).collect()),
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
    fn test_compute_suggestions_directories() {
        let db = Database::open_in_memory().unwrap();
        db.initialize().unwrap();

        let results = vec![
            make_result("agentroot://test/src/auth/login.rs", None),
            make_result("agentroot://test/src/auth/jwt.rs", None),
            make_result("agentroot://test/src/db/query.rs", None),
        ];

        let suggestions = compute_suggestions(&db, &results, "auth", None).unwrap();

        // Directories are deduplicated
        let dirs: HashSet<&str> = suggestions
            .related_directories
            .iter()
            .map(|s| s.as_str())
            .collect();
        assert!(dirs.contains("agentroot://test/src/auth"));
        assert!(dirs.contains("agentroot://test/src/db"));
        assert_eq!(dirs.len(), 2);
    }

    #[test]
    fn test_compute_suggestions_concepts() {
        let db = Database::open_in_memory().unwrap();
        db.initialize().unwrap();

        let results = vec![
            make_result("agentroot://test/a.rs", Some(vec!["rust", "search"])),
            make_result("agentroot://test/b.rs", Some(vec!["search", "index"])),
        ];

        let suggestions = compute_suggestions(&db, &results, "find", None).unwrap();

        let concepts: HashSet<&str> = suggestions
            .related_concepts
            .iter()
            .map(|s| s.as_str())
            .collect();
        assert!(concepts.contains("rust"));
        assert!(concepts.contains("search"));
        assert!(concepts.contains("index"));
    }

    #[test]
    fn test_generate_refinements() {
        let concepts = vec!["auth".to_string(), "jwt".to_string(), "session".to_string()];
        let refinements = generate_refinements("login flow", &concepts);

        assert_eq!(refinements.len(), 3);
        assert!(refinements.contains(&"login flow auth".to_string()));
        assert!(refinements.contains(&"login flow jwt".to_string()));
    }

    #[test]
    fn test_generate_refinements_excludes_query_terms() {
        let concepts = vec!["auth".to_string(), "login".to_string()];
        let refinements = generate_refinements("login flow", &concepts);

        // "login" is already in query, filtered out
        assert_eq!(refinements.len(), 1);
        assert_eq!(refinements[0], "login flow auth");
    }

    #[test]
    fn test_generate_refinements_empty() {
        let refinements = generate_refinements("test", &[]);
        assert!(refinements.is_empty());
    }
}
