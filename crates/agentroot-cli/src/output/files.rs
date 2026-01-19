//! Files list output formatter

use agentroot_core::SearchResult;

pub fn format_results(results: &[SearchResult]) -> String {
    results.iter()
        .map(|r| r.filepath.clone())
        .collect::<Vec<_>>()
        .join("\n")
}
