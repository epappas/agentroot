//! JSON output formatter

use super::FormatOptions;
use agentroot_core::SearchResult;

pub fn format_results(results: &[SearchResult], _options: &FormatOptions) -> String {
    let output: Vec<serde_json::Value> = results
        .iter()
        .map(|r| {
            serde_json::json!({
                "docid": r.docid,
                "score": r.score,
                "file": r.display_path,
                "title": r.title,
                "collection": r.collection_name,
            })
        })
        .collect();

    serde_json::to_string_pretty(&output).unwrap_or_else(|_| "[]".to_string()) + "\n"
}
