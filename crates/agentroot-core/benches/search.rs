//! Search performance benchmarks
//!
//! Measures performance of:
//! - BM25 full-text search
//! - Query parsing and execution
//! - Result ranking
//! - Database query performance

use agentroot_core::db::hash_content;
use agentroot_core::{Database, SearchOptions};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use tempfile::TempDir;

const SAMPLE_DOCS: &[(&str, &str)] = &[
    (
        "rust-intro.md",
        "# Introduction to Rust\n\nRust is a systems programming language focused on safety, speed, and concurrency.",
    ),
    (
        "rust-ownership.md",
        "# Ownership in Rust\n\nOwnership is Rust's most unique feature. It enables Rust to make memory safety guarantees.",
    ),
    (
        "rust-async.md",
        "# Async Programming\n\nRust has first-class support for async programming with async/await syntax.",
    ),
    (
        "python-intro.md",
        "# Python Basics\n\nPython is a high-level programming language known for its simplicity and readability.",
    ),
    (
        "python-async.md",
        "# Python Asyncio\n\nPython's asyncio provides infrastructure for writing concurrent code using async/await.",
    ),
    (
        "javascript-intro.md",
        "# JavaScript Overview\n\nJavaScript is the programming language of the web, essential for frontend development.",
    ),
    (
        "javascript-async.md",
        "# JavaScript Promises\n\nPromises and async/await make asynchronous programming in JavaScript much easier.",
    ),
    (
        "go-intro.md",
        "# Go Programming\n\nGo is a statically typed, compiled language designed for simplicity and efficiency.",
    ),
    (
        "go-concurrency.md",
        "# Go Goroutines\n\nGo's goroutines make concurrent programming simple and efficient with built-in support.",
    ),
    (
        "typescript-intro.md",
        "# TypeScript\n\nTypeScript adds static typing to JavaScript, making large codebases more maintainable.",
    ),
];

fn setup_test_db() -> (Database, TempDir) {
    let temp = TempDir::new().unwrap();
    let db_path = temp.path().join("search_bench.db");
    let db = Database::open(&db_path).unwrap();
    db.initialize().unwrap();

    db.add_collection("docs", "/tmp/docs", "**/*.md", "file", None)
        .unwrap();

    for (i, (filename, content)) in SAMPLE_DOCS.iter().enumerate() {
        let hash = hash_content(content);
        db.insert_content(&hash, content).unwrap();
        db.insert_document(
            "docs",
            filename,
            &format!("Document {}", i),
            &hash,
            "2024-01-01T00:00:00Z",
            "2024-01-01T00:00:00Z",
            "file",
            None,
        )
        .unwrap();
    }

    (db, temp)
}

fn bench_bm25_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("bm25_search");

    let queries = vec![
        ("single_word", "Rust"),
        ("two_words", "async programming"),
        ("phrase", "memory safety guarantees"),
        ("common_word", "programming"),
    ];

    for (name, query) in queries {
        group.bench_with_input(BenchmarkId::from_parameter(name), &query, |b, query| {
            let (db, _temp) = setup_test_db();
            let options = SearchOptions {
                limit: 10,
                min_score: 0.0,
                collection: None,
                provider: None,
                full_content: false,,
        metadata_filters: Vec::new()
            };

            b.iter(|| {
                db.search_fts(black_box(query), black_box(&options))
                    .unwrap()
            });
        });
    }

    group.finish();
}

fn bench_search_with_limits(c: &mut Criterion) {
    let mut group = c.benchmark_group("search_limits");
    let (db, _temp) = setup_test_db();
    let query = "async";

    for limit in [5, 10, 20, 50] {
        group.bench_with_input(BenchmarkId::from_parameter(limit), &limit, |b, &limit| {
            let options = SearchOptions {
                limit,
                min_score: 0.0,
                collection: None,
                provider: None,
                full_content: false,,
        metadata_filters: Vec::new()
            };

            b.iter(|| {
                db.search_fts(black_box(query), black_box(&options))
                    .unwrap()
            });
        });
    }

    group.finish();
}

fn bench_search_with_collection_filter(c: &mut Criterion) {
    let (db, _temp) = setup_test_db();

    c.bench_function("search_with_filter", |b| {
        let options = SearchOptions {
            limit: 10,
            min_score: 0.0,
            collection: Some("docs".to_string()),
            provider: None,
            full_content: false,,
        metadata_filters: Vec::new()
        };

        b.iter(|| {
            db.search_fts(black_box("programming"), black_box(&options))
                .unwrap()
        });
    });

    c.bench_function("search_without_filter", |b| {
        let options = SearchOptions {
            limit: 10,
            min_score: 0.0,
            collection: None,
            provider: None,
            full_content: false,,
        metadata_filters: Vec::new()
        };

        b.iter(|| {
            db.search_fts(black_box("programming"), black_box(&options))
                .unwrap()
        });
    });
}

fn bench_search_with_min_score(c: &mut Criterion) {
    let mut group = c.benchmark_group("search_min_score");
    let (db, _temp) = setup_test_db();
    let query = "Rust";

    for min_score in [0.0, 0.1, 0.3, 0.5] {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{:.1}", min_score)),
            &min_score,
            |b, &min_score| {
                let options = SearchOptions {
                    limit: 10,
                    min_score,
                    collection: None,
                    provider: None,
                    full_content: false,,
        metadata_filters: Vec::new()
                };

                b.iter(|| {
                    db.search_fts(black_box(query), black_box(&options))
                        .unwrap()
                });
            },
        );
    }

    group.finish();
}

fn bench_search_full_content(c: &mut Criterion) {
    let (db, _temp) = setup_test_db();

    c.bench_function("search_metadata_only", |b| {
        let options = SearchOptions {
            limit: 10,
            min_score: 0.0,
            collection: None,
            provider: None,
            full_content: false,,
        metadata_filters: Vec::new()
        };

        b.iter(|| {
            db.search_fts(black_box("programming"), black_box(&options))
                .unwrap()
        });
    });

    c.bench_function("search_with_content", |b| {
        let options = SearchOptions {
            limit: 10,
            min_score: 0.0,
            collection: None,
            provider: None,
            full_content: true,,
        metadata_filters: Vec::new()
        };

        b.iter(|| {
            db.search_fts(black_box("programming"), black_box(&options))
                .unwrap()
        });
    });
}

criterion_group!(
    benches,
    bench_bm25_search,
    bench_search_with_limits,
    bench_search_with_collection_filter,
    bench_search_with_min_score,
    bench_search_full_content,
);
criterion_main!(benches);
