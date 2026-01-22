//! Performance benchmarks for metadata generation system
//!
//! Measures:
//! - Metadata generation timing (with and without LLM)
//! - Cache hit rates
//! - Search relevance impact
//! - Memory usage patterns

use agentroot_core::{Database, SearchOptions};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use std::fs;
use std::time::{Duration, Instant};
use tempfile::TempDir;

fn create_test_documents(dir: &TempDir, count: usize) {
    for i in 0..count {
        let content = format!(
            "# Document {}\n\n\
            This is test document number {}. It contains information about \
            programming concepts, software development, and technical topics.\n\n\
            ## Section 1\n\
            Lorem ipsum dolor sit amet, consectetur adipiscing elit. \
            Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.\n\n\
            ## Section 2\n\
            Ut enim ad minim veniam, quis nostrud exercitation ullamco \
            laboris nisi ut aliquip ex ea commodo consequat.\n\n\
            ```rust\n\
            fn example() {{\n\
                println!(\"Hello, world!\");\n\
            }}\n\
            ```\n",
            i, i
        );

        fs::write(dir.path().join(format!("doc{}.md", i)), content).unwrap();
    }
}

fn bench_metadata_generation_fallback(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    create_test_documents(&temp_dir, 10);

    let db_dir = TempDir::new().unwrap();
    let db_path = db_dir.path().join("bench.sqlite");
    let db = Database::open(&db_path).unwrap();
    db.initialize().unwrap();

    db.add_collection(
        "bench",
        temp_dir.path().to_str().unwrap(),
        "*",
        "file",
        Some(r#"{"exclude_hidden":"false"}"#),
    )
    .unwrap();

    let runtime = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("metadata_generation_fallback_10docs", |b| {
        b.iter(|| {
            runtime.block_on(async {
                black_box(
                    db.reindex_collection_with_metadata("bench", None)
                        .await
                        .unwrap(),
                )
            })
        })
    });
}

fn bench_cache_performance(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    create_test_documents(&temp_dir, 5);

    let db_dir = TempDir::new().unwrap();
    let db_path = db_dir.path().join("bench.sqlite");
    let db = Database::open(&db_path).unwrap();
    db.initialize().unwrap();

    db.add_collection(
        "bench",
        temp_dir.path().to_str().unwrap(),
        "*",
        "file",
        Some(r#"{"exclude_hidden":"false"}"#),
    )
    .unwrap();

    // First indexing (cold cache)
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(async {
        db.reindex_collection_with_metadata("bench", None)
            .await
            .unwrap();
    });

    // Benchmark re-indexing with warm cache
    let runtime2 = tokio::runtime::Runtime::new().unwrap();
    c.bench_function("metadata_cache_warm_5docs", |b| {
        b.iter(|| {
            runtime2.block_on(async {
                black_box(
                    db.reindex_collection_with_metadata("bench", None)
                        .await
                        .unwrap(),
                )
            })
        })
    });
}

fn bench_search_with_metadata(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    create_test_documents(&temp_dir, 20);

    let db_dir = TempDir::new().unwrap();
    let db_path = db_dir.path().join("bench.sqlite");
    let db = Database::open(&db_path).unwrap();
    db.initialize().unwrap();

    db.add_collection(
        "bench",
        temp_dir.path().to_str().unwrap(),
        "*",
        "file",
        Some(r#"{"exclude_hidden":"false"}"#),
    )
    .unwrap();

    // Index with metadata
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(async {
        db.reindex_collection_with_metadata("bench", None)
            .await
            .unwrap();
    });

    let options = SearchOptions {
        limit: 10,
        min_score: 0.0,
        collection: Some("bench".to_string()),
        provider: None,
        full_content: false,,
        metadata_filters: Vec::new()
    };

    c.bench_function("search_with_metadata_20docs", |b| {
        b.iter(|| {
            black_box(
                db.search_fts("programming software development", &options)
                    .unwrap(),
            )
        });
    });
}

fn bench_metadata_generation_by_doc_size(c: &mut Criterion) {
    let mut group = c.benchmark_group("metadata_by_doc_size");

    for size in [100, 500, 1000, 5000].iter() {
        let temp_dir = TempDir::new().unwrap();

        // Create a document with specified character count
        let content =
            "# Test Document\n\n".to_string() + &"Lorem ipsum dolor sit amet. ".repeat(size / 28);
        fs::write(temp_dir.path().join("doc.md"), &content).unwrap();

        let db_dir = TempDir::new().unwrap();
        let db_path = db_dir.path().join("bench.sqlite");
        let db = Database::open(&db_path).unwrap();
        db.initialize().unwrap();

        db.add_collection(
            "bench",
            temp_dir.path().to_str().unwrap(),
            "*",
            "file",
            Some(r#"{"exclude_hidden":"false"}"#),
        )
        .unwrap();

        let runtime = tokio::runtime::Runtime::new().unwrap();
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _size| {
            b.iter(|| {
                runtime.block_on(async {
                    black_box(
                        db.reindex_collection_with_metadata("bench", None)
                            .await
                            .unwrap(),
                    )
                })
            })
        });
    }

    group.finish();
}

fn bench_cache_hit_rate_measurement(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    create_test_documents(&temp_dir, 10);

    let db_dir = TempDir::new().unwrap();
    let db_path = db_dir.path().join("bench.sqlite");
    let db = Database::open(&db_path).unwrap();
    db.initialize().unwrap();

    db.add_collection(
        "bench",
        temp_dir.path().to_str().unwrap(),
        "*",
        "file",
        Some(r#"{"exclude_hidden":"false"}"#),
    )
    .unwrap();

    let runtime = tokio::runtime::Runtime::new().unwrap();

    // Measure cold cache (first indexing)
    let cold_start = Instant::now();
    runtime.block_on(async {
        db.reindex_collection_with_metadata("bench", None)
            .await
            .unwrap();
    });
    let cold_duration = cold_start.elapsed();

    // Measure warm cache (re-indexing)
    let warm_start = Instant::now();
    runtime.block_on(async {
        db.reindex_collection_with_metadata("bench", None)
            .await
            .unwrap();
    });
    let warm_duration = warm_start.elapsed();

    // Calculate speedup
    let speedup = cold_duration.as_millis() as f64 / warm_duration.as_millis().max(1) as f64;

    println!("\n=== Cache Performance Metrics ===");
    println!("Cold cache (10 docs): {:.2}ms", cold_duration.as_millis());
    println!("Warm cache (10 docs): {:.2}ms", warm_duration.as_millis());
    println!("Speedup factor: {:.2}x", speedup);
    println!(
        "Cache hit rate (estimated): {:.1}%",
        ((speedup - 1.0) / speedup) * 100.0
    );

    c.bench_function("cache_hit_rate_report", |b| {
        b.iter(|| black_box(()));
    });
}

fn bench_memory_usage_metadata(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    create_test_documents(&temp_dir, 50);

    let db_dir = TempDir::new().unwrap();
    let db_path = db_dir.path().join("bench.sqlite");
    let db = Database::open(&db_path).unwrap();
    db.initialize().unwrap();

    db.add_collection(
        "bench",
        temp_dir.path().to_str().unwrap(),
        "*",
        "file",
        Some(r#"{"exclude_hidden":"false"}"#),
    )
    .unwrap();

    // Measure database size before metadata
    let size_before = fs::metadata(&db_path).unwrap().len();

    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(async {
        db.reindex_collection_with_metadata("bench", None)
            .await
            .unwrap();
    });

    // Measure database size after metadata
    let size_after = fs::metadata(&db_path).unwrap().len();
    let metadata_overhead = size_after - size_before;
    let per_doc_overhead = metadata_overhead as f64 / 50.0;

    println!("\n=== Memory Usage Metrics ===");
    println!("DB size before metadata: {} bytes", size_before);
    println!("DB size after metadata: {} bytes", size_after);
    println!("Metadata overhead: {} bytes", metadata_overhead);
    println!("Per-document overhead: {:.2} bytes", per_doc_overhead);

    c.bench_function("memory_usage_report", |b| {
        b.iter(|| black_box(()));
    });
}

fn bench_search_relevance_impact(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();

    // Create documents with specific topics
    fs::write(
        temp_dir.path().join("rust-tutorial.md"),
        "# Rust Tutorial\n\nLearn Rust programming for beginners. \
         This tutorial covers ownership, borrowing, and lifetimes.",
    )
    .unwrap();

    fs::write(
        temp_dir.path().join("python-guide.md"),
        "# Python Guide\n\nPython programming guide for data science. \
         Learn pandas, numpy, and machine learning basics.",
    )
    .unwrap();

    fs::write(
        temp_dir.path().join("javascript-ref.md"),
        "# JavaScript Reference\n\nJavaScript language reference for web development. \
         Covers ES6+, async/await, and modern frameworks.",
    )
    .unwrap();

    let db_dir = TempDir::new().unwrap();
    let db_path = db_dir.path().join("bench.sqlite");
    let db = Database::open(&db_path).unwrap();
    db.initialize().unwrap();

    db.add_collection(
        "bench",
        temp_dir.path().to_str().unwrap(),
        "*",
        "file",
        Some(r#"{"exclude_hidden":"false"}"#),
    )
    .unwrap();

    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(async {
        db.reindex_collection_with_metadata("bench", None)
            .await
            .unwrap();
    });

    let options = SearchOptions {
        limit: 10,
        min_score: 0.0,
        collection: Some("bench".to_string()),
        provider: None,
        full_content: false,,
        metadata_filters: Vec::new()
    };

    // Benchmark search quality
    let results = db.search_fts("Rust beginners", &options).unwrap();
    let rust_rank = results
        .iter()
        .position(|r| r.filepath.contains("rust"))
        .map(|p| p + 1)
        .unwrap_or(0);

    println!("\n=== Search Relevance Metrics ===");
    println!("Query: 'Rust beginners'");
    println!("Results found: {}", results.len());
    println!("Rust tutorial rank: {}", rust_rank);
    if rust_rank > 0 {
        println!("Top result score: {:.2}", results[0].score);
    }

    c.bench_function("search_relevance_report", |b| {
        b.iter(|| black_box(()));
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .measurement_time(Duration::from_secs(10))
        .sample_size(10);
    targets =
        bench_metadata_generation_fallback,
        bench_cache_performance,
        bench_search_with_metadata,
        bench_metadata_generation_by_doc_size,
        bench_cache_hit_rate_measurement,
        bench_memory_usage_metadata,
        bench_search_relevance_impact
}

criterion_main!(benches);
