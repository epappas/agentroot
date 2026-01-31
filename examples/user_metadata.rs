//! Rich metadata system demo
//!
//! Demonstrates MetadataBuilder, MetadataFilter, find_by_metadata,
//! get_metadata, merge, and removal.
//!
//! Usage:
//!   cargo run --example user_metadata -p agentroot-core

use agentroot_core::db::{hash_content, Database};
use agentroot_core::{MetadataBuilder, MetadataFilter, SearchOptions};
use chrono::Utc;

fn main() -> agentroot_core::Result<()> {
    println!("=== User Metadata System Demo ===\n");

    let db = Database::open_in_memory()?;
    db.initialize()?;
    db.add_collection("articles", ".", "**/*.md", "file", None)?;

    let now = Utc::now().to_rfc3339();

    let articles: Vec<(&str, &str, &str)> = vec![
        ("rust_ownership.md", "Rust Ownership Explained", "Ownership is Rust's most unique feature. It enables memory safety without a garbage collector."),
        ("async_patterns.md", "Async Patterns in Rust", "Async/await in Rust provides zero-cost futures. Tokio is the most popular runtime."),
        ("wasm_guide.md", "WebAssembly with Rust", "Compile Rust to WebAssembly for near-native performance in the browser."),
        ("error_handling.md", "Error Handling Best Practices", "Use Result and the ? operator for propagating errors. Custom error types improve ergonomics."),
        ("testing_strategies.md", "Testing Strategies for Rust", "Unit tests, integration tests, and property-based testing with proptest."),
        ("perf_tuning.md", "Performance Tuning Guide", "Profile with flamegraph, optimize hot paths, use SIMD intrinsics for data-parallel workloads."),
    ];

    let mut hashes: Vec<String> = Vec::new();
    for (path, title, content) in &articles {
        let hash = hash_content(content);
        db.insert_content(&hash, content)?;
        db.insert_document("articles", path, title, &hash, &now, &now, "file", None)?;
        hashes.push(hash);
    }
    println!("Inserted {} documents\n", articles.len());

    // Attach rich metadata to each document
    let docids: Vec<String> = hashes.iter().map(|h| h.chars().take(6).collect()).collect();

    let meta0 = MetadataBuilder::new()
        .text("author", "Alice")
        .tags("topics", vec!["memory", "ownership", "borrowing"])
        .integer("difficulty", 2)
        .boolean("has_code_samples", true)
        .enum_value("level", "intermediate", vec!["beginner".into(), "intermediate".into(), "advanced".into()])?
        .quantitative("read_minutes", 8.0, "min")
        .datetime_now("indexed_at")
        .json("extra", serde_json::json!({"series": "core-concepts"}))
        .build();

    let meta1 = MetadataBuilder::new()
        .text("author", "Bob")
        .tags("topics", vec!["async", "concurrency", "tokio"])
        .integer("difficulty", 3)
        .boolean("has_code_samples", true)
        .qualitative("complexity", "high", vec!["low".into(), "medium".into(), "high".into()])?
        .build();

    let meta2 = MetadataBuilder::new()
        .text("author", "Alice")
        .tags("topics", vec!["wasm", "frontend", "performance"])
        .integer("difficulty", 2)
        .boolean("has_code_samples", true)
        .build();

    let meta3 = MetadataBuilder::new()
        .text("author", "Charlie")
        .tags("topics", vec!["errors", "result", "anyhow"])
        .integer("difficulty", 1)
        .boolean("has_code_samples", true)
        .build();

    let meta4 = MetadataBuilder::new()
        .text("author", "Bob")
        .tags("topics", vec!["testing", "proptest", "ci"])
        .integer("difficulty", 2)
        .boolean("has_code_samples", false)
        .build();

    let meta5 = MetadataBuilder::new()
        .text("author", "Charlie")
        .tags("topics", vec!["performance", "simd", "profiling"])
        .integer("difficulty", 4)
        .boolean("has_code_samples", true)
        .build();

    let metas = [&meta0, &meta1, &meta2, &meta3, &meta4, &meta5];
    for (docid, meta) in docids.iter().zip(metas.iter()) {
        db.add_metadata(docid, meta)?;
    }
    println!("Attached metadata to all documents\n");

    // Demonstrate get_metadata
    println!("--- get_metadata for first document ---");
    if let Some(retrieved) = db.get_metadata(&docids[0])? {
        println!("  author: {:?}", retrieved.get("author"));
        println!("  topics: {:?}", retrieved.get("topics"));
        println!("  difficulty: {:?}", retrieved.get("difficulty"));
        println!("  has_code_samples: {:?}", retrieved.get("has_code_samples"));
    }

    // Filter: difficulty > 2
    println!("\n--- Filter: difficulty > 2 ---");
    let hard = MetadataFilter::IntegerGt("difficulty".into(), 2);
    let results = db.find_by_metadata(&hard, 10)?;
    println!("  Found {} docs: {:?}", results.len(), results);

    // Filter: topics contain "performance"
    println!("\n--- Filter: topics contain 'performance' ---");
    let perf = MetadataFilter::TagsContain("topics".into(), "performance".into());
    let results = db.find_by_metadata(&perf, 10)?;
    println!("  Found {} docs: {:?}", results.len(), results);

    // Filter: has_code_samples == true
    println!("\n--- Filter: has_code_samples == true ---");
    let coded = MetadataFilter::BooleanEq("has_code_samples".into(), true);
    let results = db.find_by_metadata(&coded, 10)?;
    println!("  Found {} docs with code samples", results.len());

    // Compound: (difficulty > 2) AND (has_code_samples == true)
    println!("\n--- Filter: hard AND has code samples ---");
    let compound = MetadataFilter::And(vec![
        MetadataFilter::IntegerGt("difficulty".into(), 2),
        MetadataFilter::BooleanEq("has_code_samples".into(), true),
    ]);
    let results = db.find_by_metadata(&compound, 10)?;
    println!("  Found {} docs: {:?}", results.len(), results);

    // Or: authored by Alice OR difficulty == 1
    println!("\n--- Filter: author='Alice' OR difficulty=1 ---");
    let either = MetadataFilter::Or(vec![
        MetadataFilter::TextEq("author".into(), "Alice".into()),
        MetadataFilter::IntegerEq("difficulty".into(), 1),
    ]);
    let results = db.find_by_metadata(&either, 10)?;
    println!("  Found {} docs: {:?}", results.len(), results);

    // Not: NOT authored by Bob
    println!("\n--- Filter: NOT author='Bob' ---");
    let not_bob = MetadataFilter::Not(Box::new(MetadataFilter::TextEq(
        "author".into(),
        "Bob".into(),
    )));
    let results = db.find_by_metadata(&not_bob, 10)?;
    println!("  Found {} docs: {:?}", results.len(), results);

    // Demonstrate merge
    println!("\n--- Metadata merge ---");
    let patch = MetadataBuilder::new()
        .integer("difficulty", 3)
        .text("reviewer", "Diana")
        .build();
    db.add_metadata(&docids[0], &patch)?;

    if let Some(merged) = db.get_metadata(&docids[0])? {
        println!("  After merge: difficulty={:?}, reviewer={:?}, author={:?}",
            merged.get("difficulty"), merged.get("reviewer"), merged.get("author"));
    }

    // Demonstrate removal
    println!("\n--- Metadata field removal ---");
    db.remove_metadata_fields(&docids[0], &["reviewer".into()])?;
    if let Some(after) = db.get_metadata(&docids[0])? {
        println!("  reviewer present after removal: {}", after.contains("reviewer"));
        println!("  author still present: {}", after.contains("author"));
    }

    // Search with metadata context
    println!("\n--- BM25 search showing metadata integration ---");
    let opts = SearchOptions {
        limit: 3,
        min_score: 0.0,
        collection: Some("articles".into()),
        full_content: false,
        provider: None,
        metadata_filters: vec![],
    };
    let results = db.search_fts("performance", &opts)?;
    for r in &results {
        println!("  {} (score: {:.3})", r.display_path, r.score);
    }

    println!("\nDone.");
    Ok(())
}
