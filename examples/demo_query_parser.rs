//! Demo showing query parser extracting temporal and metadata filters

use agentroot_core::{MetadataFilterHint, ParsedQuery, SearchType, TemporalFilter};
use chrono::Utc;

fn main() {
    println!("=== Natural Language Query Parser Demo ===\n");

    let queries = vec![
        "files edited last hour",
        "documents from yesterday",
        "rust tutorials by Alice",
        "author:Bob python guides",
        "code modified this week",
        "async programming tagged python",
        "rust ownership",
    ];

    for query in queries {
        println!("Input: \"{}\"", query);
        let parsed = parse_query_rules(query);

        println!("  → Search terms: \"{}\"", parsed.search_terms);

        if let Some(temporal) = &parsed.temporal_filter {
            println!("  → Temporal: {}", temporal.description);
            if let Some(start) = &temporal.start {
                println!("     Start: {}", &start[..19]);
            }
            if let Some(end) = &temporal.end {
                println!("     End: {}", &end[..19]);
            }
        }

        if !parsed.metadata_filters.is_empty() {
            for filter in &parsed.metadata_filters {
                println!(
                    "  → Metadata: {}:{}={}",
                    filter.field, filter.operator, filter.value
                );
            }
        }

        println!("  → Type: {:?}\n", parsed.search_type);
    }
}

fn parse_query_rules(query: &str) -> ParsedQuery {
    let mut search_terms = query.to_string();
    let mut temporal_filter = None;
    let mut metadata_filters = Vec::new();

    // Temporal: "last hour"
    if query.contains("last hour") {
        let now = Utc::now();
        let one_hour_ago = now - chrono::Duration::hours(1);
        temporal_filter = Some(TemporalFilter {
            start: Some(one_hour_ago.to_rfc3339()),
            end: Some(now.to_rfc3339()),
            description: "Last hour".to_string(),
        });
        search_terms = search_terms.replace("last hour", "").trim().to_string();
    }

    // Temporal: "yesterday"
    if query.contains("yesterday") {
        let now = Utc::now();
        let yesterday = now - chrono::Duration::days(1);
        temporal_filter = Some(TemporalFilter {
            start: Some(
                yesterday
                    .date_naive()
                    .and_hms_opt(0, 0, 0)
                    .unwrap()
                    .and_utc()
                    .to_rfc3339(),
            ),
            end: Some(
                yesterday
                    .date_naive()
                    .and_hms_opt(23, 59, 59)
                    .unwrap()
                    .and_utc()
                    .to_rfc3339(),
            ),
            description: "Yesterday".to_string(),
        });
        search_terms = search_terms.replace("yesterday", "").trim().to_string();
    }

    // Temporal: "this week"
    if query.contains("this week") {
        let now = Utc::now();
        let week_start = now - chrono::Duration::days(7);
        temporal_filter = Some(TemporalFilter {
            start: Some(week_start.to_rfc3339()),
            end: Some(now.to_rfc3339()),
            description: "This week".to_string(),
        });
        search_terms = search_terms.replace("this week", "").trim().to_string();
    }

    // Metadata: "by Author"
    if let Some(by_pos) = query.find(" by ") {
        let after_by = &query[by_pos + 4..];
        let author = after_by.split_whitespace().next().unwrap_or("");
        if !author.is_empty() {
            metadata_filters.push(MetadataFilterHint {
                field: "author".to_string(),
                operator: "contains".to_string(),
                value: author.to_string(),
            });
            search_terms = search_terms
                .replace(&format!(" by {}", author), "")
                .trim()
                .to_string();
        }
    }

    // Metadata: "author:Value"
    if let Some(author_pos) = query.find("author:") {
        let after_colon = &query[author_pos + 7..];
        let author = after_colon.split_whitespace().next().unwrap_or("");
        if !author.is_empty() {
            metadata_filters.push(MetadataFilterHint {
                field: "author".to_string(),
                operator: "eq".to_string(),
                value: author.to_string(),
            });
            search_terms = search_terms
                .replace(&format!("author:{}", author), "")
                .trim()
                .to_string();
        }
    }

    // Metadata: "tagged Value"
    if let Some(tagged_pos) = query.find("tagged ") {
        let after_tagged = &query[tagged_pos + 7..];
        let tag = after_tagged.split_whitespace().next().unwrap_or("");
        if !tag.is_empty() {
            metadata_filters.push(MetadataFilterHint {
                field: "tags".to_string(),
                operator: "contains".to_string(),
                value: tag.to_string(),
            });
            search_terms = search_terms
                .replace(&format!("tagged {}", tag), "")
                .trim()
                .to_string();
        }
    }

    // Clean up
    search_terms = search_terms
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if search_terms.is_empty() {
        search_terms = query.to_string();
    }

    ParsedQuery {
        search_terms,
        temporal_filter,
        metadata_filters,
        search_type: SearchType::Bm25,
        confidence: 0.8,
    }
}
