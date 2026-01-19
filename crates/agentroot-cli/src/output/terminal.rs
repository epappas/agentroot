//! Terminal output formatter

use super::FormatOptions;
use agentroot_core::SearchResult;

pub fn format_results(results: &[SearchResult], options: &FormatOptions) -> String {
    if results.is_empty() {
        return String::new();
    }

    let mut output = String::new();

    for result in results {
        let score_pct = (result.score * 100.0) as u32;
        output.push_str(&format!(
            "{:>3}% {} #{}\n",
            score_pct, result.display_path, result.docid
        ));

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
