//! Indexing performance benchmarks
//!
//! Measures performance of:
//! - Document insertion
//! - Content hashing
//! - AST chunking
//! - Full indexing pipeline

use agentroot_core::db::hash_content;
use agentroot_core::index::{chunk_by_chars, chunk_semantic};
use agentroot_core::Database;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::path::Path;
use tempfile::TempDir;

const SMALL_DOC: &str = r#"# Small Document
This is a small test document with just a few lines.
It's used to benchmark baseline performance."#;

const MEDIUM_DOC: &str = r#"# Medium Document

## Introduction
This is a medium-sized document used for benchmarking. It contains multiple sections
and enough content to trigger the chunking algorithm meaningfully.

## Features
Here are some key features:
- Feature one with detailed description
- Feature two with even more text
- Feature three that goes on for a while

## Implementation
The implementation follows best practices and includes proper error handling.
We use async/await throughout for better performance.

## Examples
```rust
fn main() {
    println!("Hello, world!");
}
```

## Conclusion
This concludes our medium-sized test document.
"#;

fn generate_large_doc(sections: usize) -> String {
    let mut doc = String::from("# Large Document\n\n");
    for i in 0..sections {
        doc.push_str(&format!(
            "## Section {}\n\nThis is section {} with enough content to make the document large.\n\
             We want to test performance with realistic document sizes that developers might encounter.\n\
             This includes code blocks, lists, and various markdown elements.\n\n\
             ```rust\n\
             fn section_{}() {{\n\
                 let data = vec![1, 2, 3, 4, 5];\n\
                 data.iter().sum::<i32>()\n\
             }}\n\
             ```\n\n\
             - Point 1\n\
             - Point 2\n\
             - Point 3\n\n",
            i, i, i
        ));
    }
    doc
}

fn bench_content_hashing(c: &mut Criterion) {
    let mut group = c.benchmark_group("content_hashing");

    for (name, content) in &[
        ("small", SMALL_DOC),
        ("medium", MEDIUM_DOC),
        ("large", &generate_large_doc(50)),
    ] {
        group.throughput(Throughput::Bytes(content.len() as u64));
        group.bench_with_input(BenchmarkId::from_parameter(name), content, |b, content| {
            b.iter(|| hash_content(black_box(content)));
        });
    }

    group.finish();
}

fn bench_document_chunking(c: &mut Criterion) {
    let mut group = c.benchmark_group("document_chunking");

    for (name, content) in &[
        ("markdown_small", SMALL_DOC),
        ("markdown_medium", MEDIUM_DOC),
        ("markdown_large", &generate_large_doc(50)),
    ] {
        group.throughput(Throughput::Bytes(content.len() as u64));
        group.bench_with_input(BenchmarkId::from_parameter(name), content, |b, content| {
            b.iter(|| chunk_by_chars(black_box(content), 3200, 480));
        });
    }

    group.finish();
}

fn bench_rust_ast_chunking(c: &mut Criterion) {
    let rust_code = r#"
//! Module documentation

use std::collections::HashMap;

/// A sample struct
pub struct MyStruct {
    pub field: String,
}

impl MyStruct {
    /// Create a new instance
    pub fn new(field: String) -> Self {
        Self { field }
    }

    /// Get the field value
    pub fn get_field(&self) -> &str {
        &self.field
    }
}

/// A sample function
pub fn process_data(data: &[i32]) -> i32 {
    data.iter().sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_struct() {
        let s = MyStruct::new("test".to_string());
        assert_eq!(s.get_field(), "test");
    }
}
"#;

    c.bench_function("ast_chunking/rust", |b| {
        b.iter(|| chunk_semantic(black_box(rust_code), Path::new("test.rs")).ok());
    });
}

fn bench_database_insertion(c: &mut Criterion) {
    let mut group = c.benchmark_group("database_insertion");
    group.sample_size(50);

    group.bench_function("single_document", |b| {
        b.iter_batched(
            || {
                let temp = TempDir::new().unwrap();
                let db_path = temp.path().join("bench.db");
                let db = Database::open(&db_path).unwrap();
                db.initialize().unwrap();
                (db, temp)
            },
            |(db, _temp)| {
                let hash = hash_content(MEDIUM_DOC);
                db.insert_content(&hash, MEDIUM_DOC).unwrap();
                db.insert_document(
                    "bench",
                    "doc.md",
                    "Benchmark Doc",
                    &hash,
                    "2024-01-01T00:00:00Z",
                    "2024-01-01T00:00:00Z",
                    "file",
                    None,
                )
                .unwrap();
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.bench_function("batch_10_documents", |b| {
        b.iter_batched(
            || {
                let temp = TempDir::new().unwrap();
                let db_path = temp.path().join("bench.db");
                let db = Database::open(&db_path).unwrap();
                db.initialize().unwrap();
                (db, temp)
            },
            |(db, _temp)| {
                for i in 0..10 {
                    let content = format!("{}\n\nDocument {}", MEDIUM_DOC, i);
                    let hash = hash_content(&content);
                    db.insert_content(&hash, &content).unwrap();
                    db.insert_document(
                        "bench",
                        &format!("doc{}.md", i),
                        &format!("Benchmark Doc {}", i),
                        &hash,
                        "2024-01-01T00:00:00Z",
                        "2024-01-01T00:00:00Z",
                        "file",
                        None,
                    )
                    .unwrap();
                }
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.finish();
}

fn bench_full_indexing_pipeline(c: &mut Criterion) {
    let mut group = c.benchmark_group("full_pipeline");
    group.sample_size(20);

    group.bench_function("index_medium_document", |b| {
        b.iter_batched(
            || {
                let temp = TempDir::new().unwrap();
                let db_path = temp.path().join("bench.db");
                let db = Database::open(&db_path).unwrap();
                db.initialize().unwrap();
                (db, temp)
            },
            |(db, _temp)| {
                let chunks = chunk_by_chars(MEDIUM_DOC, 3200, 480);
                let hash = hash_content(MEDIUM_DOC);

                db.insert_content(&hash, MEDIUM_DOC).unwrap();
                db.insert_document(
                    "bench",
                    "doc.md",
                    "Medium Doc",
                    &hash,
                    "2024-01-01T00:00:00Z",
                    "2024-01-01T00:00:00Z",
                    "file",
                    None,
                )
                .unwrap();

                black_box(chunks);
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_content_hashing,
    bench_document_chunking,
    bench_rust_ast_chunking,
    bench_database_insertion,
    bench_full_indexing_pipeline,
);
criterion_main!(benches);
