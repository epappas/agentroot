//! Multi-step search workflow demo
//!
//! Demonstrates Workflow/WorkflowStep construction, execute_workflow,
//! fallback_workflow heuristic, and execution traces via WorkflowContext.
//!
//! Usage:
//!   cargo run --example workflow_search -p agentroot-core

use agentroot_core::db::{hash_content, Database};
use agentroot_core::llm::{fallback_workflow, MergeStrategy, Workflow, WorkflowStep};
use agentroot_core::search::{execute_workflow, SearchOptions};
use chrono::Utc;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> agentroot_core::Result<()> {
    println!("=== Workflow Search Demo ===\n");

    let db = Database::open_in_memory()?;
    db.initialize()?;
    db.add_collection("docs", ".", "**/*.md", "file", None)?;

    let now = Utc::now().to_rfc3339();

    let documents: Vec<(&str, &str, &str, &str, &str)> = vec![
        ("tutorial/rust_basics.md", "Rust Basics Tutorial", "tutorial", "beginner",
         "Learn Rust fundamentals: variables, functions, ownership, and basic error handling with Result."),
        ("tutorial/async_rust.md", "Async Rust Tutorial", "tutorial", "advanced",
         "Master async/await patterns in Rust using Tokio runtime for concurrent network programming."),
        ("reference/bm25_scoring.md", "BM25 Scoring Reference", "reference", "intermediate",
         "BM25 is a probabilistic ranking function used in information retrieval for full-text search."),
        ("reference/vector_search.md", "Vector Similarity Search", "reference", "advanced",
         "Vector search uses embeddings and cosine similarity for semantic document retrieval."),
        ("guide/error_patterns.md", "Error Handling Patterns", "guide", "intermediate",
         "Comprehensive guide to error handling in Rust: Result, Option, anyhow, thiserror crates."),
        ("guide/testing_guide.md", "Testing Guide", "guide", "beginner",
         "Write effective unit tests, integration tests, and use property-based testing with proptest."),
        ("tutorial/wasm_intro.md", "WebAssembly Introduction", "tutorial", "intermediate",
         "Compile Rust to WebAssembly and run it in the browser with wasm-bindgen and wasm-pack."),
        ("reference/search_api.md", "Search API Reference", "reference", "advanced",
         "Full-text search API supporting BM25, vector similarity, and hybrid reciprocal rank fusion."),
    ];

    for (path, title, category, difficulty, content) in &documents {
        let hash = hash_content(content);
        db.insert_content(&hash, content)?;
        db.insert_doc(
            &agentroot_core::db::DocumentInsert::new("docs", path, title, &hash, &now, &now)
                .with_llm_metadata_strings(
                    content, title, "[]", category, "", "[]", difficulty, "[]", "", &now,
                ),
        )?;
    }
    println!("Inserted {} documents\n", documents.len());

    // Insert chunks for two documents to enable chunk search
    let chunk_data: Vec<(&str, &str, Vec<(&str, &str)>)> = vec![
        ("BM25 uses term frequency and inverse document frequency for scoring.",
         "bm25_scoring", vec![("layer", "algorithm"), ("topic", "ranking")]),
        ("Cosine similarity measures the angle between embedding vectors.",
         "vector_math", vec![("layer", "algorithm"), ("topic", "similarity")]),
        ("Reciprocal Rank Fusion merges results from multiple retrieval systems.",
         "rrf_fusion", vec![("layer", "algorithm"), ("topic", "fusion")]),
    ];

    let ref_hash = hash_content(documents[2].4);
    for (seq, (content, breadcrumb, labels)) in chunk_data.iter().enumerate() {
        let chunk_hash = hash_content(content);
        let label_map: HashMap<String, String> = labels.iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        db.insert_chunk(
            &chunk_hash, &ref_hash, seq as i32, 0,
            content, Some("Paragraph"), Some(breadcrumb),
            (seq as i32) * 5 + 1, (seq as i32) * 5 + 4, None,
            Some(content), None, &[], &label_map, &[],
            None, None, &now,
        )?;
    }
    println!("Inserted {} chunks for reference docs\n", chunk_data.len());

    let options = SearchOptions::default();

    // Workflow 1: Simple BM25
    println!("--- Workflow 1: Simple BM25 ---");
    let wf1 = Workflow {
        steps: vec![WorkflowStep::Bm25Search {
            query: "error handling Result".into(),
            limit: 5,
        }],
        reasoning: "Direct keyword match for error handling".into(),
        expected_results: 5,
        complexity: "simple".into(),
    };
    let results = execute_workflow(&db, &wf1, "error handling Result", &options).await?;
    print_results("error handling Result", &results);

    // Workflow 2: Doc BM25 + Chunk BM25, merged with RRF + dedup
    println!("--- Workflow 2: Doc + Chunk merged (RRF) ---");
    let wf2 = Workflow {
        steps: vec![
            WorkflowStep::Bm25Search { query: "search ranking algorithm".into(), limit: 10 },
            WorkflowStep::Bm25ChunkSearch { query: "search ranking algorithm".into(), limit: 10 },
            WorkflowStep::Merge { strategy: MergeStrategy::Rrf },
            WorkflowStep::Deduplicate,
            WorkflowStep::Limit { count: 5 },
        ],
        reasoning: "Combine document and chunk results via RRF for broader recall".into(),
        expected_results: 5,
        complexity: "moderate".into(),
    };
    let results = execute_workflow(&db, &wf2, "search ranking algorithm", &options).await?;
    print_results("search ranking algorithm", &results);

    // Workflow 3: Filtered search by metadata
    println!("--- Workflow 3: Filtered by category + difficulty ---");
    let wf3 = Workflow {
        steps: vec![
            WorkflowStep::Bm25Search { query: "Rust programming".into(), limit: 10 },
            WorkflowStep::FilterMetadata {
                category: Some("tutorial".into()),
                difficulty: None,
                tags: None,
                exclude_category: None,
                exclude_difficulty: Some("advanced".into()),
            },
        ],
        reasoning: "Find beginner-friendly tutorials about Rust".into(),
        expected_results: 5,
        complexity: "moderate".into(),
    };
    let results = execute_workflow(&db, &wf3, "Rust programming", &options).await?;
    print_results("Rust programming (tutorials, not advanced)", &results);

    // Demonstrate fallback_workflow heuristic
    println!("--- fallback_workflow heuristic ---");
    let queries = vec![
        "fn validate",
        "how does BM25 work",
        "Config::new",
        "testing strategies",
    ];
    for q in queries {
        let wf = fallback_workflow(q, false);
        let step_names: Vec<String> = wf.steps.iter().map(|s| format!("{:?}", s)).collect();
        println!("  \"{}\" -> complexity={}, steps={}",
            q, wf.complexity, step_names.len());
        for s in &step_names {
            let short = if s.len() > 60 { &s[..60] } else { s };
            println!("    {}", short);
        }
    }

    println!("\nDone.");
    Ok(())
}

fn print_results(query: &str, results: &[agentroot_core::SearchResult]) {
    println!("  Query: \"{}\"", query);
    if results.is_empty() {
        println!("  No results.\n");
        return;
    }
    for (i, r) in results.iter().enumerate() {
        let kind = if r.is_chunk { "chunk" } else { "doc" };
        println!("  {}. [{}] {} (score: {:.3}) -- {}",
            i + 1, kind, r.display_path, r.score,
            r.llm_category.as_deref().unwrap_or("-"));
    }
    println!();
}
