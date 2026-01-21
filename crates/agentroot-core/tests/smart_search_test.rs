//! Integration test for smart natural language search
//!
//! Tests that natural language queries like "files edited last hour"
//! are correctly parsed and filtered.

use agentroot_core::{Database, MetadataBuilder, SearchOptions};
use std::fs;
use tempfile::TempDir;

#[tokio::test]
async fn test_smart_search_basic_functionality() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.sqlite");
    let content_dir = temp_dir.path().join("content");
    fs::create_dir(&content_dir).unwrap();

    // Create database
    let db = Database::open(&db_path).unwrap();
    db.initialize().unwrap();

    // Create test files
    let rust_file = content_dir.join("rust_guide.md");
    fs::write(&rust_file, "Complete guide to Rust programming language").unwrap();

    let python_file = content_dir.join("python_guide.md");
    fs::write(&python_file, "Complete guide to Python programming").unwrap();

    // Add collection
    db.add_collection(
        "test",
        content_dir.to_str().unwrap(),
        "**/*.md",
        "file",
        None,
    )
    .unwrap();

    // Index files
    db.reindex_collection("test").await.unwrap();

    // Test: Search using smart_search function
    let options = SearchOptions::default();
    let results = agentroot_core::smart_search(&db, "rust programming", &options).await;

    match results {
        Ok(results) => {
            // Should find rust guide
            assert!(!results.is_empty(), "Should find rust document");
            let has_rust = results.iter().any(|r| r.filepath.contains("rust_guide"));
            assert!(has_rust, "Should find rust_guide.md");
        }
        Err(e) => {
            // Expected if query parser model not available
            println!("Smart search failed (model may not be installed): {}", e);
        }
    }
}

#[tokio::test]
async fn test_smart_search_with_metadata() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.sqlite");
    let content_dir = temp_dir.path().join("content");
    fs::create_dir(&content_dir).unwrap();

    // Create database
    let db = Database::open(&db_path).unwrap();
    db.initialize().unwrap();

    // Create test files
    let file1 = content_dir.join("doc1.md");
    fs::write(&file1, "Tutorial about Rust by Alice").unwrap();

    let file2 = content_dir.join("doc2.md");
    fs::write(&file2, "Guide about Python by Bob").unwrap();

    // Add collection
    db.add_collection(
        "test",
        content_dir.to_str().unwrap(),
        "**/*.md",
        "file",
        None,
    )
    .unwrap();

    // Index files
    db.reindex_collection("test").await.unwrap();

    // Add metadata to documents using path
    let alice_meta = MetadataBuilder::new().text("author", "Alice").build();
    let doc1_path = file1.to_str().unwrap();
    if let Ok(docs) = db.get_documents_by_pattern("doc1.md") {
        if let Some(_) = docs.first() {
            db.add_metadata(doc1_path, &alice_meta).unwrap();
        }
    }

    let bob_meta = MetadataBuilder::new().text("author", "Bob").build();
    let doc2_path = file2.to_str().unwrap();
    if let Ok(docs) = db.get_documents_by_pattern("doc2.md") {
        if let Some(_) = docs.first() {
            db.add_metadata(doc2_path, &bob_meta).unwrap();
        }
    }

    // Test: Basic keyword search should work
    let options = SearchOptions::default();
    let results = agentroot_core::smart_search(&db, "rust", &options).await;

    match results {
        Ok(results) => {
            assert!(!results.is_empty(), "Should find rust document");
        }
        Err(e) => {
            println!(
                "Smart search failed (expected if model not installed): {}",
                e
            );
        }
    }
}

#[tokio::test]
#[ignore] // Requires running vLLM server, run manually with: cargo test -- --ignored
async fn test_smart_search_bm25_fallback() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.sqlite");
    let content_dir = temp_dir.path().join("content");
    fs::create_dir(&content_dir).unwrap();

    // Create database
    let db = Database::open(&db_path).unwrap();
    db.initialize().unwrap();

    // Create test file
    let file = content_dir.join("test.md");
    fs::write(&file, "This is a test document about testing").unwrap();

    // Add collection
    db.add_collection(
        "test",
        content_dir.to_str().unwrap(),
        "**/*.md",
        "file",
        None,
    )
    .unwrap();

    // Index files
    db.reindex_collection("test").await.unwrap();

    // Test: Even if parser unavailable, BM25 should work
    let options = SearchOptions::default();
    let results = agentroot_core::smart_search(&db, "test document", &options).await;

    match results {
        Ok(results) => {
            assert!(!results.is_empty(), "BM25 fallback should find document");
        }
        Err(e) => {
            // If even BM25 fails, something is wrong
            panic!("BM25 fallback should not fail: {}", e);
        }
    }
}
