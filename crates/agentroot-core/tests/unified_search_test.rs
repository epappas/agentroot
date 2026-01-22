//! Integration tests for unified intelligent search

use agentroot_core::parse_metadata_filters;

// Note: Full unified search functionality requires external services
// (embeddings, LLM) so comprehensive integration tests are manual.
// These tests verify the core metadata filtering parsing logic.

#[test]
fn test_metadata_filter_parsing() {
    // Test basic category filter
    let (query, filters) = parse_metadata_filters("provider category:tutorial");
    assert_eq!(query, "provider");
    assert_eq!(filters.len(), 1);
    assert_eq!(filters[0], ("category".to_string(), "tutorial".to_string()));

    // Test multiple filters
    let (query, filters) =
        parse_metadata_filters("error handling category:code difficulty:beginner");
    assert_eq!(query, "error handling");
    assert_eq!(filters.len(), 2);
    assert!(filters.contains(&("category".to_string(), "code".to_string())));
    assert!(filters.contains(&("difficulty".to_string(), "beginner".to_string())));

    // Test keyword/tag filter
    let (query, filters) = parse_metadata_filters("search tag:rust");
    assert_eq!(query, "search");
    assert_eq!(filters.len(), 1);
    assert_eq!(filters[0], ("tag".to_string(), "rust".to_string()));

    // Test no filters
    let (query, filters) = parse_metadata_filters("just a regular query");
    assert_eq!(query, "just a regular query");
    assert_eq!(filters.len(), 0);

    // Test colon in non-filter context (URLs should not be parsed as filters)
    let (query, filters) = parse_metadata_filters("http://example.com test");
    assert_eq!(query, "http://example.com test");
    assert_eq!(filters.len(), 0);

    // Test mixed filters and regular terms
    let (query, filters) = parse_metadata_filters(
        "search term1 category:guide term2 difficulty:advanced tag:tutorial",
    );
    assert_eq!(query, "search term1 term2");
    assert_eq!(filters.len(), 3);
    assert!(filters.contains(&("category".to_string(), "guide".to_string())));
    assert!(filters.contains(&("difficulty".to_string(), "advanced".to_string())));
    assert!(filters.contains(&("tag".to_string(), "tutorial".to_string())));

    // Test keyword filter (alias for tag)
    let (query, filters) = parse_metadata_filters("search keyword:async");
    assert_eq!(query, "search");
    assert_eq!(filters.len(), 1);
    assert_eq!(filters[0], ("keyword".to_string(), "async".to_string()));

    // Test empty query with only filters
    let (query, filters) = parse_metadata_filters("category:code");
    assert_eq!(query, "");
    assert_eq!(filters.len(), 1);
    assert_eq!(filters[0], ("category".to_string(), "code".to_string()));
}
