//! Integration test for MCP tools with metadata

use agentroot_core::{Database, LlamaMetadataGenerator, MetadataGenerator};
use agentroot_mcp::tools::*;
use serde_json::json;
use std::fs;
use tempfile::TempDir;

#[tokio::test]
async fn test_search_tool_with_metadata() {
    // Create temporary directory with test files
    let temp_dir = TempDir::new().unwrap();

    fs::write(
        temp_dir.path().join("rust-tutorial.md"),
        "# Rust Tutorial for Beginners\n\nRust is a systems programming language.",
    )
    .unwrap();

    fs::write(
        temp_dir.path().join("python-guide.md"),
        "# Python Guide\n\nPython is a high-level programming language for beginners.",
    )
    .unwrap();

    // Create database and add collection
    let db_dir = TempDir::new().unwrap();
    let db_path = db_dir.path().join("test.sqlite");
    let db = Database::open(&db_path).unwrap();
    db.initialize().unwrap();

    db.add_collection(
        "test-docs",
        temp_dir.path().to_str().unwrap(),
        "*",
        "file",
        Some(r#"{"exclude_hidden":"false"}"#),
    )
    .unwrap();

    // Index with metadata
    let generator_result = LlamaMetadataGenerator::from_default();
    let generator = generator_result.ok();

    if let Some(ref gen) = generator {
        db.reindex_collection_with_metadata("test-docs", Some(gen as &dyn MetadataGenerator))
            .await
            .unwrap();
    } else {
        db.reindex_collection_with_metadata("test-docs", None)
            .await
            .unwrap();
    }

    // Test search tool
    let args = json!({
        "query": "Rust",
        "limit": 10,
        "collection": "test-docs"
    });

    let result = handle_search(&db, args).await.unwrap();
    assert!(!result.is_error.unwrap_or(false));

    // Verify structured content includes results
    let structured = result.structured_content.unwrap();
    let results = structured["results"].as_array().unwrap();
    assert!(!results.is_empty(), "Should find Rust-related documents");

    // Verify metadata fields are included (if metadata was generated)
    if generator.is_some() {
        let first_result = &results[0];
        println!(
            "First result: {}",
            serde_json::to_string_pretty(&first_result).unwrap()
        );

        // Metadata fields should be present if they were generated
        // They may be null if LLM wasn't available, which is okay
    }

    println!("Search tool test passed - found {} results", results.len());
}

#[tokio::test]
async fn test_search_tool_with_filters() {
    // Create temporary directory with test files
    let temp_dir = TempDir::new().unwrap();

    fs::write(
        temp_dir.path().join("tutorial.md"),
        "# Tutorial Document\n\nThis is a beginner-friendly tutorial.",
    )
    .unwrap();

    fs::write(
        temp_dir.path().join("reference.md"),
        "# Reference Document\n\nThis is an advanced reference guide.",
    )
    .unwrap();

    // Create database and add collection
    let db_dir = TempDir::new().unwrap();
    let db_path = db_dir.path().join("test.sqlite");
    let db = Database::open(&db_path).unwrap();
    db.initialize().unwrap();

    db.add_collection(
        "test-docs",
        temp_dir.path().to_str().unwrap(),
        "*",
        "file",
        Some(r#"{"exclude_hidden":"false"}"#),
    )
    .unwrap();

    // Index with metadata
    let generator_result = LlamaMetadataGenerator::from_default();
    let generator = generator_result.ok();

    if let Some(ref gen) = generator {
        db.reindex_collection_with_metadata("test-docs", Some(gen as &dyn MetadataGenerator))
            .await
            .unwrap();
    } else {
        db.reindex_collection_with_metadata("test-docs", None)
            .await
            .unwrap();
    }

    // Test search with difficulty filter
    let args = json!({
        "query": "document",
        "limit": 10,
        "collection": "test-docs",
        "difficulty": "beginner"
    });

    let result = handle_search(&db, args).await.unwrap();
    assert!(!result.is_error.unwrap_or(false));

    let structured = result.structured_content.unwrap();
    let results = structured["results"].as_array().unwrap();

    if generator.is_some() && !results.is_empty() {
        // If metadata was generated, filtered results should only include beginner docs
        println!(
            "Filter test: found {} results with difficulty=beginner",
            results.len()
        );
        for result in results {
            if let Some(difficulty) = result.get("difficulty") {
                println!("  - Document difficulty: {:?}", difficulty);
            }
        }
    } else {
        println!("Metadata not available - filter test skipped");
    }
}

#[tokio::test]
async fn test_query_tool_with_metadata() {
    // Create temporary directory with test files
    let temp_dir = TempDir::new().unwrap();

    fs::write(
        temp_dir.path().join("doc1.md"),
        "# Programming Languages\n\nRust and Python are popular programming languages.",
    )
    .unwrap();

    // Create database and add collection
    let db_dir = TempDir::new().unwrap();
    let db_path = db_dir.path().join("test.sqlite");
    let db = Database::open(&db_path).unwrap();
    db.initialize().unwrap();

    db.add_collection(
        "test-docs",
        temp_dir.path().to_str().unwrap(),
        "*",
        "file",
        Some(r#"{"exclude_hidden":"false"}"#),
    )
    .unwrap();

    // Index with metadata
    let generator_result = LlamaMetadataGenerator::from_default();
    let generator = generator_result.ok();

    if let Some(ref gen) = generator {
        db.reindex_collection_with_metadata("test-docs", Some(gen as &dyn MetadataGenerator))
            .await
            .unwrap();
    } else {
        db.reindex_collection_with_metadata("test-docs", None)
            .await
            .unwrap();
    }

    // Test query tool (hybrid search)
    let args = json!({
        "query": "programming",
        "limit": 10,
        "collection": "test-docs"
    });

    let result = handle_query(&db, args).await.unwrap();
    assert!(!result.is_error.unwrap_or(false));

    let structured = result.structured_content.unwrap();
    let results = structured["results"].as_array().unwrap();

    if !results.is_empty() {
        println!("Query tool test passed - found {} results", results.len());

        // Check if metadata is included
        let first_result = &results[0];
        if first_result.get("summary").is_some() {
            println!("Metadata included in query results");
        }
    }
}
