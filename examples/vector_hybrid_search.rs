//! Vector and hybrid search demo
//!
//! Demonstrates ensure_vec_table, insert_embedding, get_all_embeddings,
//! cosine_similarity, and BM25+vector fusion -- all without an external
//! embedding service using synthetic TF-IDF-style vectors.
//!
//! Usage:
//!   cargo run --example vector_hybrid_search -p agentroot-core

use agentroot_core::db::{hash_content, vectors::cosine_similarity, Database};
use agentroot_core::search::{SearchOptions, SearchResult};
use chrono::Utc;

// 15-term vocabulary for synthetic embeddings
const VOCAB: &[&str] = &[
    "rust",
    "python",
    "async",
    "web",
    "database",
    "search",
    "vector",
    "error",
    "test",
    "performance",
    "memory",
    "concurrent",
    "api",
    "query",
    "index",
];
const DIM: usize = 15;

fn term_vector(text: &str) -> Vec<f32> {
    let lower = text.to_lowercase();
    let mut vec = vec![0.0f32; DIM];
    for (i, term) in VOCAB.iter().enumerate() {
        vec[i] = lower.matches(term).count() as f32;
    }
    // L2 normalize
    let norm: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        vec.iter_mut().for_each(|x| *x /= norm);
    }
    vec
}

fn main() -> agentroot_core::Result<()> {
    println!("=== Vector & Hybrid Search Demo ===\n");

    let db = Database::open_in_memory()?;
    db.initialize()?;
    db.add_collection("kb", ".", "**/*.md", "file", None)?;
    db.ensure_vec_table(DIM)?;

    let now = Utc::now().to_rfc3339();

    let docs: Vec<(&str, &str, &str)> = vec![
        ("rust_async.md", "Async Programming in Rust",
         "Rust async concurrent programming with tokio runtime for high-performance web api servers."),
        ("python_ml.md", "Python Machine Learning",
         "Python is great for machine learning. Use vector embeddings for semantic search and query."),
        ("db_indexing.md", "Database Indexing Strategies",
         "Database index optimization for search query performance. B-tree and inverted index."),
        ("error_handling.md", "Error Handling Patterns",
         "Rust error handling with Result and the question-mark operator for robust error propagation."),
        ("web_testing.md", "Web API Testing",
         "Test web api endpoints with integration test suites. Performance test for concurrent load."),
        ("memory_safety.md", "Memory Safety in Systems Programming",
         "Rust memory safety guarantees prevent data races in concurrent programs without garbage collection."),
    ];

    let mut hashes: Vec<String> = Vec::new();
    for (path, title, content) in &docs {
        let hash = hash_content(content);
        db.insert_content(&hash, content)?;
        db.insert_document("kb", path, title, &hash, &now, &now, "file", None)?;

        let embedding = term_vector(content);
        db.insert_embedding(&hash, 0, 0, "tfidf-synthetic", &embedding)?;
        hashes.push(hash);
    }
    println!("Inserted {} documents with embeddings\n", docs.len());

    let options = SearchOptions {
        limit: 6,
        min_score: 0.0,
        collection: Some("kb".into()),

        provider: None,
        metadata_filters: vec![],
        ..Default::default()
    };

    // 1. Pure BM25 search
    println!("--- BM25 search: 'concurrent async performance' ---");
    let bm25_results = db.search_fts("concurrent async performance", &options)?;
    print_ranked("BM25", &bm25_results);

    // 2. Manual vector similarity search
    println!("--- Vector search: 'concurrent async performance' ---");
    let query_vec = term_vector("concurrent async performance");
    let all_embeddings = db.get_all_embeddings()?;

    let mut scored: Vec<(f32, &str, &str)> = Vec::new();
    for (hash_seq, emb) in &all_embeddings {
        let hash = hash_seq.split('_').next().unwrap_or("");
        let sim = cosine_similarity(&query_vec, emb);
        // Find matching doc
        if let Some(idx) = hashes.iter().position(|h| h == hash) {
            scored.push((sim, docs[idx].0, docs[idx].1));
        }
    }
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
    println!("  Vector results:");
    for (i, (sim, path, title)) in scored.iter().enumerate() {
        println!("  {}. {} -- {} (sim: {:.4})", i + 1, path, title, sim);
    }
    println!();

    // 3. Hybrid fusion (manual RRF)
    println!("--- Hybrid fusion (RRF) ---");
    let hybrid = rrf_fuse(&bm25_results, &scored);
    println!("  Hybrid results:");
    for (i, (score, path)) in hybrid.iter().enumerate() {
        println!("  {}. {} (fused: {:.4})", i + 1, path, score);
    }
    println!();

    // 4. Demonstrate case where vector finds what BM25 misses
    println!("--- Semantic gap: 'safe systems language' ---");
    println!("  (no exact keyword overlap with 'memory safety')");
    let gap_bm25 = db.search_fts("safe systems language", &options)?;
    print_ranked("BM25", &gap_bm25);

    let gap_vec = term_vector("memory safety concurrent rust performance");
    let mut gap_scored: Vec<(f32, &str, &str)> = Vec::new();
    for (hash_seq, emb) in &all_embeddings {
        let hash = hash_seq.split('_').next().unwrap_or("");
        let sim = cosine_similarity(&gap_vec, emb);
        if let Some(idx) = hashes.iter().position(|h| h == hash) {
            gap_scored.push((sim, docs[idx].0, docs[idx].1));
        }
    }
    gap_scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
    println!("  Vector results (semantic expansion):");
    for (i, (sim, path, title)) in gap_scored.iter().take(3).enumerate() {
        println!("  {}. {} -- {} (sim: {:.4})", i + 1, path, title, sim);
    }

    println!("\nDone.");
    Ok(())
}

fn print_ranked(label: &str, results: &[SearchResult]) {
    println!("  {} results:", label);
    if results.is_empty() {
        println!("  (none)\n");
        return;
    }
    for (i, r) in results.iter().enumerate() {
        println!(
            "  {}. {} -- {} (score: {:.4})",
            i + 1,
            r.display_path,
            r.title,
            r.score
        );
    }
    println!();
}

const RRF_K: f64 = 60.0;

fn rrf_fuse<'a>(bm25: &[SearchResult], vec_scored: &[(f32, &'a str, &str)]) -> Vec<(f64, &'a str)> {
    let mut scores: std::collections::HashMap<&str, f64> = std::collections::HashMap::new();

    // BM25 results: match on the raw path portion of display_path
    for (rank, r) in bm25.iter().enumerate() {
        let path = r.display_path.split('/').last().unwrap_or(&r.display_path);
        // Find matching vec entry to use same key
        if let Some((_, vpath, _)) = vec_scored.iter().find(|(_, p, _)| *p == path) {
            *scores.entry(vpath).or_default() += 1.0 / (RRF_K + (rank + 1) as f64);
        }
    }
    for (rank, (_sim, path, _title)) in vec_scored.iter().enumerate() {
        *scores.entry(path).or_default() += 1.0 / (RRF_K + (rank + 1) as f64);
    }

    let mut results: Vec<(f64, &str)> = scores.into_iter().map(|(p, s)| (s, p)).collect();
    results.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
    results
}
