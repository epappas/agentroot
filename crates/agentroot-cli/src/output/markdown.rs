//! Markdown output formatter

use super::FormatOptions;
use agentroot_core::SearchResult;

pub fn format_results(results: &[SearchResult], _options: &FormatOptions) -> String {
    let mut output = String::from("# Search Results\n\n");

    for (i, r) in results.iter().enumerate() {
        output.push_str(&format!(
            "## {}. {} (Score: {:.2})\n\n",
            i + 1,
            r.title,
            r.score
        ));
        output.push_str(&format!("- **File**: `{}`\n", r.display_path));
        output.push_str(&format!("- **Collection**: {}\n", r.collection_name));
        output.push_str(&format!("- **DocID**: `{}`\n", r.docid));
        output.push_str("\n---\n\n");
    }

    if results.is_empty() {
        output.push_str("*No results found*\n");
    }

    output
}
