//! CSV output formatter

use super::FormatOptions;
use agentroot_core::SearchResult;

pub fn format_results(results: &[SearchResult], _options: &FormatOptions) -> String {
    let mut output = String::from("docid,score,file,title,collection\n");

    for r in results {
        let escaped_file = escape_csv(&r.display_path);
        let escaped_title = escape_csv(&r.title);
        let escaped_collection = escape_csv(&r.collection_name);

        output.push_str(&format!(
            "{},{},{},{},{}\n",
            r.docid, r.score, escaped_file, escaped_title, escaped_collection
        ));
    }

    output
}

fn escape_csv(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}
