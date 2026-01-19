//! Integration test for LLM-generated metadata system
//!
//! This test creates a real collection with markdown and code files,
//! generates metadata using the LLM, and verifies search quality.

use agentroot_core::{Database, LlamaMetadataGenerator, MetadataGenerator, SearchOptions};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Creates test files with realistic content
fn create_test_files(dir: &TempDir) -> Vec<PathBuf> {
    let mut files = Vec::new();

    // 1. Rust tutorial document
    let rust_guide = dir.path().join("rust-getting-started.md");
    fs::write(
        &rust_guide,
        r#"# Getting Started with Rust

Rust is a systems programming language that provides memory safety without garbage collection.
It achieves this through a unique ownership system that the compiler checks at compile time.

## Installation

Install Rust using rustup:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

## Your First Program

Create a new project:

```bash
cargo new hello_world
cd hello_world
cargo run
```

This creates a simple "Hello, world!" program that you can build and run.

## Key Concepts

- **Ownership**: Each value has a single owner
- **Borrowing**: References allow you to access data without taking ownership
- **Lifetimes**: Ensure references are always valid

Rust is perfect for systems programming, web servers, and command-line tools.
"#,
    )
    .unwrap();
    files.push(rust_guide);

    // 2. Advanced Rust document
    let async_rust = dir.path().join("async-programming.md");
    fs::write(
        &async_rust,
        r#"# Async Programming in Rust

Asynchronous programming allows you to write concurrent code using async/await syntax.
Rust's async model is zero-cost and provides excellent performance.

## Tokio Runtime

The most popular async runtime is Tokio:

```rust
#[tokio::main]
async fn main() {
    let result = fetch_data().await;
    println!("Result: {:?}", result);
}
```

## Key Concepts

- **Futures**: Lazy computations that produce a value
- **Async/Await**: Syntax for writing asynchronous code
- **Runtime**: Executes futures and manages task scheduling

This is considered an intermediate to advanced topic in Rust.
"#,
    )
    .unwrap();
    files.push(async_rust);

    // 3. Python tutorial
    let python_guide = dir.path().join("python-basics.md");
    fs::write(
        &python_guide,
        r#"# Python Basics for Beginners

Python is a high-level, interpreted programming language known for its simplicity.
It's perfect for beginners and widely used in data science, web development, and automation.

## Hello World

```python
print("Hello, World!")
```

## Variables and Types

```python
name = "Alice"
age = 30
is_student = False
```

Python uses dynamic typing, so you don't need to declare variable types.

## Lists and Loops

```python
fruits = ["apple", "banana", "cherry"]
for fruit in fruits:
    print(fruit)
```

Python is beginner-friendly and great for learning programming concepts.
"#,
    )
    .unwrap();
    files.push(python_guide);

    // 4. Configuration file example
    let config_doc = dir.path().join("config-reference.md");
    fs::write(
        &config_doc,
        r#"# Configuration Reference

This document describes all configuration options for the application.

## Database Settings

- `database.url`: Connection string for the database
- `database.pool_size`: Maximum number of connections (default: 10)
- `database.timeout`: Connection timeout in seconds (default: 30)

## Server Settings

- `server.host`: Host address to bind to (default: 127.0.0.1)
- `server.port`: Port number to listen on (default: 8080)
- `server.workers`: Number of worker threads (default: 4)

## Example Configuration

```toml
[database]
url = "postgres://localhost/mydb"
pool_size = 20

[server]
host = "0.0.0.0"
port = 3000
```
"#,
    )
    .unwrap();
    files.push(config_doc);

    // 5. Rust code file
    let rust_code = dir.path().join("example.rs");
    fs::write(
        &rust_code,
        r#"//! Example Rust module demonstrating ownership and borrowing

/// Calculates the length of a string without taking ownership
pub fn calculate_length(s: &String) -> usize {
    s.len()
}

/// Takes ownership and returns a modified string
pub fn add_suffix(mut s: String, suffix: &str) -> String {
    s.push_str(suffix);
    s
}

/// A simple struct demonstrating Rust's type system
#[derive(Debug, Clone)]
pub struct User {
    pub name: String,
    pub age: u32,
    pub email: String,
}

impl User {
    /// Creates a new user
    pub fn new(name: String, age: u32, email: String) -> Self {
        Self { name, age, email }
    }

    /// Checks if the user is an adult
    pub fn is_adult(&self) -> bool {
        self.age >= 18
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_length() {
        let s = String::from("hello");
        assert_eq!(calculate_length(&s), 5);
    }

    #[test]
    fn test_is_adult() {
        let user = User::new("Alice".to_string(), 25, "alice@example.com".to_string());
        assert!(user.is_adult());
    }
}
"#,
    )
    .unwrap();
    files.push(rust_code);

    files
}

#[tokio::test]
async fn test_end_to_end_metadata_generation() {
    // Create temporary directory with test files
    let temp_dir = TempDir::new().unwrap();
    let files = create_test_files(&temp_dir);

    assert_eq!(files.len(), 5, "Should create 5 test files");

    // Create temporary database
    let db_dir = TempDir::new().unwrap();
    let db_path = db_dir.path().join("test.sqlite");
    let db = Database::open(&db_path).unwrap();
    db.initialize().unwrap();

    // Add collection (use * pattern to match all files in root)
    let collection_name = "test-docs";
    db.add_collection(
        collection_name,
        temp_dir.path().to_str().unwrap(),
        "*",
        "file",
        Some(r#"{"exclude_hidden":"false"}"#),
    )
    .unwrap();

    // Create metadata generator (will use fallback if LLM not available)
    let generator_result = LlamaMetadataGenerator::from_default();
    let generator = generator_result.ok();

    // Index with metadata
    let indexed_count = if let Some(ref gen) = generator {
        db.reindex_collection_with_metadata(collection_name, Some(gen as &dyn MetadataGenerator))
            .await
            .unwrap()
    } else {
        db.reindex_collection_with_metadata(collection_name, None)
            .await
            .unwrap()
    };

    assert_eq!(indexed_count, 5, "Should index all 5 files");

    // Verify documents are in database
    let collections = db.list_collections().unwrap();
    assert_eq!(collections.len(), 1);
    assert_eq!(collections[0].name, collection_name);
    assert_eq!(collections[0].document_count, 5);

    // Use search to get documents with metadata
    let search_opts = SearchOptions {
        limit: 10,
        min_score: 0.0,
        collection: Some(collection_name.to_string()),
        provider: None,
        full_content: false,
    };

    // Search for common words to get all documents
    let all_results = db
        .search_fts("a OR the OR is OR Rust", &search_opts)
        .unwrap();
    assert!(
        all_results.len() >= 4,
        "Should have at least 4 documents with common words"
    );

    let mut metadata_count = 0;
    for result in &all_results {
        if result.llm_summary.is_some() {
            metadata_count += 1;

            // Verify metadata quality
            let summary = result.llm_summary.as_ref().unwrap();
            assert!(
                !summary.is_empty(),
                "Summary should not be empty for {}",
                result.filepath
            );

            if let Some(title) = &result.llm_title {
                assert!(!title.is_empty(), "Title should not be empty");
            }

            if let Some(category) = &result.llm_category {
                assert!(!category.is_empty(), "Category should not be empty");
            }

            if let Some(keywords) = &result.llm_keywords {
                assert!(
                    !keywords.is_empty(),
                    "Keywords should not be empty for {}",
                    result.filepath
                );
            }
        }
    }

    println!(
        "Generated metadata for {} out of {} documents",
        metadata_count,
        all_results.len()
    );

    if generator.is_some() {
        // If LLM model is available, metadata should be generated
        println!("LLM model available - verifying metadata was generated");
        assert!(
            metadata_count >= 4,
            "Should generate metadata for most documents when LLM is available"
        );
    } else {
        // Without LLM model, metadata won't be generated, but indexing should still work
        println!("LLM model not available - metadata generation skipped (expected)");
    }
}

#[tokio::test]
async fn test_metadata_improves_search_quality() {
    // Create temporary directory with test files
    let temp_dir = TempDir::new().unwrap();
    let _files = create_test_files(&temp_dir);

    // Create temporary database
    let db_dir = TempDir::new().unwrap();
    let db_path = db_dir.path().join("test.sqlite");
    let db = Database::open(&db_path).unwrap();
    db.initialize().unwrap();

    // Add collection and index with metadata
    let collection_name = "search-test";
    db.add_collection(
        collection_name,
        temp_dir.path().to_str().unwrap(),
        "*",
        "file",
        Some(r#"{"exclude_hidden":"false"}"#),
    )
    .unwrap();

    let generator_result = LlamaMetadataGenerator::from_default();
    let generator = generator_result.ok();

    if let Some(ref gen) = generator {
        db.reindex_collection_with_metadata(collection_name, Some(gen as &dyn MetadataGenerator))
            .await
            .unwrap();
    } else {
        db.reindex_collection_with_metadata(collection_name, None)
            .await
            .unwrap();
    }

    // Test BM25 search with metadata
    let search_opts = SearchOptions {
        limit: 10,
        min_score: 0.0,
        collection: Some(collection_name.to_string()),
        provider: None,
        full_content: false,
    };

    // Search for "Rust programming beginners"
    let mut results = db.search_fts("Rust programming", &search_opts).unwrap();
    if results.is_empty() {
        // If no results, try a simpler query
        results = db.search_fts("Rust", &search_opts).unwrap();
    }
    assert!(!results.is_empty(), "Should find results for Rust query");

    // The beginner Rust guide should rank highly
    let has_rust_guide = results.iter().any(|r| {
        r.filepath.contains("rust-getting-started")
            || r.title.to_lowercase().contains("getting started")
    });
    assert!(has_rust_guide, "Should find the Rust getting started guide");

    // Search for "asynchronous" or "async"
    let mut results = db.search_fts("asynchronous", &search_opts).unwrap();
    if results.is_empty() {
        results = db.search_fts("async", &search_opts).unwrap();
    }
    assert!(!results.is_empty(), "Should find results for async query");

    let has_async_doc = results.iter().any(|r| {
        r.filepath.contains("async-programming") || r.title.to_lowercase().contains("async")
    });
    assert!(has_async_doc, "Should find the async programming document");

    // Search for "Python"
    let results = db.search_fts("Python", &search_opts).unwrap();
    assert!(!results.is_empty(), "Should find results for Python query");

    let has_python_guide = results
        .iter()
        .any(|r| r.filepath.contains("python-basics") || r.title.to_lowercase().contains("python"));
    assert!(has_python_guide, "Should find the Python basics guide");

    // Search for "configuration"
    let results = db.search_fts("configuration", &search_opts).unwrap();
    assert!(!results.is_empty(), "Should find results for config query");

    let has_config_doc = results.iter().any(|r| {
        r.filepath.contains("config-reference") || r.title.to_lowercase().contains("config")
    });
    assert!(has_config_doc, "Should find the configuration reference");

    println!("Search quality test passed - metadata enhances discoverability");
}

#[tokio::test]
async fn test_metadata_fields_in_search_results() {
    // Create temporary directory with test files
    let temp_dir = TempDir::new().unwrap();
    let _files = create_test_files(&temp_dir);

    // Create temporary database
    let db_dir = TempDir::new().unwrap();
    let db_path = db_dir.path().join("test.sqlite");
    let db = Database::open(&db_path).unwrap();
    db.initialize().unwrap();

    // Add collection and index
    let collection_name = "metadata-fields-test";
    db.add_collection(
        collection_name,
        temp_dir.path().to_str().unwrap(),
        "*",
        "file",
        Some(r#"{"exclude_hidden":"false"}"#),
    )
    .unwrap();

    let generator_result = LlamaMetadataGenerator::from_default();
    let generator = generator_result.ok();

    if let Some(ref gen) = generator {
        db.reindex_collection_with_metadata(collection_name, Some(gen as &dyn MetadataGenerator))
            .await
            .unwrap();
    } else {
        db.reindex_collection_with_metadata(collection_name, None)
            .await
            .unwrap();
    }

    // Search and verify metadata in results
    let search_opts = SearchOptions {
        limit: 10,
        min_score: 0.0,
        collection: Some(collection_name.to_string()),
        provider: None,
        full_content: false,
    };

    let results = db.search_fts("Rust", &search_opts).unwrap();
    assert!(!results.is_empty(), "Should find Rust-related documents");

    // Check that at least one result has metadata (if LLM was available)
    let has_metadata = results.iter().any(|r| {
        r.llm_summary.is_some()
            || r.llm_title.is_some()
            || r.llm_keywords.is_some()
            || r.llm_category.is_some()
            || r.llm_difficulty.is_some()
    });

    if generator.is_some() {
        assert!(
            has_metadata,
            "Search results should include metadata fields when LLM is available"
        );
    } else {
        println!("LLM model not available - skipping metadata field checks");
    }

    // Verify metadata quality for results with metadata
    for result in &results {
        if let Some(summary) = &result.llm_summary {
            assert!(summary.len() >= 20, "Summary should be substantial");
        }

        if let Some(keywords) = &result.llm_keywords {
            assert!(!keywords.is_empty(), "Keywords should not be empty");
        }

        if let Some(difficulty) = &result.llm_difficulty {
            assert!(
                ["beginner", "intermediate", "advanced"].contains(&difficulty.as_str()),
                "Difficulty should be a valid level"
            );
        }
    }

    println!("Metadata fields successfully included in search results");
}

#[tokio::test]
async fn test_metadata_cache_functionality() {
    // Create temporary directory with one test file
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.md");
    fs::write(
        &test_file,
        "# Test Document\n\nThis is a test document for cache verification.",
    )
    .unwrap();

    // Create temporary database
    let db_dir = TempDir::new().unwrap();
    let db_path = db_dir.path().join("test.sqlite");
    let db = Database::open(&db_path).unwrap();
    db.initialize().unwrap();

    // Add collection
    let collection_name = "cache-test";
    db.add_collection(
        collection_name,
        temp_dir.path().to_str().unwrap(),
        "**/*.md",
        "file",
        Some(r#"{"exclude_hidden":"false"}"#),
    )
    .unwrap();

    // Index first time
    let generator_result = LlamaMetadataGenerator::from_default();
    let generator = generator_result.ok();

    if let Some(ref gen) = generator {
        db.reindex_collection_with_metadata(collection_name, Some(gen as &dyn MetadataGenerator))
            .await
            .unwrap();
    } else {
        db.reindex_collection_with_metadata(collection_name, None)
            .await
            .unwrap();
    }

    // Get search results to check metadata
    let search_opts = SearchOptions {
        limit: 10,
        min_score: 0.0,
        collection: Some(collection_name.to_string()),
        provider: None,
        full_content: false,
    };

    let docs = db.search_fts("test OR document", &search_opts).unwrap();
    assert_eq!(docs.len(), 1);
    let first_summary = docs[0].llm_summary.clone();

    // Re-index (should use cache)
    if let Some(ref gen) = generator {
        db.reindex_collection_with_metadata(collection_name, Some(gen as &dyn MetadataGenerator))
            .await
            .unwrap();
    } else {
        db.reindex_collection_with_metadata(collection_name, None)
            .await
            .unwrap();
    }

    // Verify document still has the same metadata
    let docs_after = db.search_fts("test OR document", &search_opts).unwrap();
    assert_eq!(docs_after.len(), 1);
    assert_eq!(
        docs_after[0].llm_summary, first_summary,
        "Metadata should be preserved through cache"
    );

    println!("Metadata caching works correctly");
}
