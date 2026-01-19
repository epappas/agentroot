//! XML output formatter

use super::FormatOptions;
use agentroot_core::SearchResult;

pub fn format_results(results: &[SearchResult], _options: &FormatOptions) -> String {
    let mut output = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<results>\n");

    for r in results {
        output.push_str("  <result>\n");
        output.push_str(&format!("    <docid>{}</docid>\n", escape_xml(&r.docid)));
        output.push_str(&format!("    <score>{}</score>\n", r.score));
        output.push_str(&format!(
            "    <file>{}</file>\n",
            escape_xml(&r.display_path)
        ));
        output.push_str(&format!("    <title>{}</title>\n", escape_xml(&r.title)));
        output.push_str(&format!(
            "    <collection>{}</collection>\n",
            escape_xml(&r.collection_name)
        ));
        output.push_str("  </result>\n");
    }

    output.push_str("</results>\n");
    output
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
