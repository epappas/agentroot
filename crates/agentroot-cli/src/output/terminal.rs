//! Terminal output formatter

use super::FormatOptions;
use agentroot_core::{SearchResult, SearchSource};

pub fn format_results(results: &[SearchResult], options: &FormatOptions) -> String {
    if results.is_empty() {
        return String::new();
    }

    let mut output = String::new();

    for result in results {
        let score_pct = (result.score * 100.0) as u32;

        // Match type indicator
        let match_type = match result.source {
            SearchSource::Bm25 => "[BM25]",
            SearchSource::Vector => "[VECTOR]",
            SearchSource::Hybrid => "[HYBRID]",
            SearchSource::Glossary => "[GLOSSARY]",
        };

        // Check if this is a chunk result
        if result.is_chunk {
            // Format chunk result with additional details
            let breadcrumb = result
                .chunk_breadcrumb
                .as_deref()
                .unwrap_or("<no breadcrumb>");
            let chunk_type = result
                .chunk_type
                .as_ref()
                .map(|t| format!(" ({})", t))
                .unwrap_or_default();

            output.push_str(&format!(
                "{} {:>3}% {} {} [Lines {}-{}] #{}\n",
                match_type,
                score_pct,
                breadcrumb,
                chunk_type,
                result.chunk_start_line.unwrap_or(0),
                result.chunk_end_line.unwrap_or(0),
                result.docid
            ));

            output.push_str(&format!("  File: {}\n", result.display_path));

            // Show summary if available
            if let Some(ref summary) = result.chunk_summary {
                output.push_str(&format!("  Summary: {}\n", summary));
            }

            // Show labels if available
            if !result.chunk_labels.is_empty() {
                let labels: Vec<String> = result
                    .chunk_labels
                    .iter()
                    .map(|(k, v)| format!("{}:{}", k, v))
                    .collect();
                output.push_str(&format!("  Labels: {}\n", labels.join(", ")));
            }

            // Show context snippet if available
            if let Some(ref context) = result.context {
                output.push_str(&format!("  \"{}\"\n", context.trim()));
            } else if let Some(ref body) = result.body {
                // Fallback to body preview if no context
                let preview = body.chars().take(150).collect::<String>();
                let preview_clean = preview.replace('\n', " ");
                output.push_str(&format!("  \"{}...\"\n", preview_clean.trim()));
            }
        } else {
            // Format document result with match type and snippet
            output.push_str(&format!(
                "{} {:>3}% {} #{}\n",
                match_type, score_pct, result.display_path, result.docid
            ));

            // Show context snippet for all document results
            if let Some(ref context) = result.context {
                // Clean up snippet: replace newlines with spaces and collapse whitespace
                // Using split_whitespace + join is simple and efficient for this use case
                let cleaned = context
                    .replace(['\n', '\r'], " ")
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ");

                // Truncate if needed (using char boundaries for UTF-8 safety)
                let display = if cleaned.len() > 150 {
                    // Find safe truncation point at char boundary
                    let truncate_pos = cleaned
                        .char_indices()
                        .take_while(|(pos, _)| *pos <= 150)
                        .last()
                        .map(|(pos, _)| pos)
                        .unwrap_or(0);
                    format!("{}...", &cleaned[..truncate_pos])
                } else {
                    cleaned
                };
                output.push_str(&format!("  {}\n", display));
            }
        }

        if options.full {
            if let Some(ref body) = result.body {
                let lines: Vec<&str> = body.lines().take(5).collect();
                for (i, line) in lines.iter().enumerate() {
                    if options.line_numbers {
                        output.push_str(&format!("  {:>4} {}\n", i + 1, line));
                    } else {
                        output.push_str(&format!("  {}\n", line));
                    }
                }
                if body.lines().count() > 5 {
                    output.push_str("  ...\n");
                }
            }
        }
    }

    output
}
