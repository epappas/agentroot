//! Metadata Generation Example
//!
//! Demonstrates LLM-powered automatic metadata generation for documents
//! using an external HTTP service.
//!
//! Requirements:
//! - Set AGENTROOT_LLM_URL environment variable
//! - Set AGENTROOT_LLM_MODEL environment variable
//! - Running external LLM service (vLLM, Basilica, OpenAI, etc.)
//!
//! Example:
//!   export AGENTROOT_LLM_URL="https://your-service.com/v1"
//!   export AGENTROOT_LLM_MODEL="Qwen/Qwen2.5-7B-Instruct"
//!   cargo run --example metadata_generation
//!
//! This example shows:
//! 1. Creating a test document
//! 2. Generating metadata using HttpMetadataGenerator
//! 3. Storing metadata in the database
//! 4. Searching with metadata-enhanced results

use agentroot_core::{
    Database, DocumentMetadata, HttpMetadataGenerator, MetadataContext, MetadataGenerator,
};
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Agentroot Metadata Generation Example ===\n");

    // Step 1: Create test content
    let test_content = r#"
# Getting Started with Rust

Rust is a systems programming language that runs blazingly fast, prevents segfaults,
and guarantees thread safety. It accomplishes these goals without garbage collection,
making it a useful language for a number of use cases other languages aren't good at.

## Key Features

- Zero-cost abstractions
- Move semantics
- Guaranteed memory safety
- Threads without data races
- Trait-based generics
- Pattern matching
- Type inference
- Minimal runtime

## Installation

To install Rust, use rustup:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

## First Program

Here's a simple "Hello, World!" program:

```rust
fn main() {
    println!("Hello, world!");
}
```

Compile and run with:

```bash
rustc main.rs
./main
```
"#;

    println!("📄 Test Document Created ({} bytes)\n", test_content.len());

    // Step 2: Create metadata context (environmental signals)
    let context = MetadataContext::new("file".to_string(), "test-docs".to_string())
        .with_extension("md".to_string())
        .with_timestamps(
            "2024-01-01T00:00:00Z".to_string(),
            "2024-01-15T00:00:00Z".to_string(),
        );

    println!("🔧 Metadata Context:");
    println!("  - Source Type: {}", context.source_type);
    println!("  - Collection: {}", context.collection_name);
    println!(
        "  - Extension: {}",
        context.file_extension.as_ref().unwrap()
    );
    println!();

    // Step 3: Try to generate metadata with LLM (will fall back to heuristics if model unavailable)
    println!("🤖 Attempting metadata generation...");

    let generator_result = HttpMetadataGenerator::from_env();

    let metadata: DocumentMetadata = match generator_result {
        Ok(generator) => {
            println!("✅ Using LLM: {}", generator.model_name());
            println!("   (This may take 10-30 seconds for first run)\n");

            match generator.generate_metadata(test_content, &context).await {
                Ok(meta) => {
                    println!("✅ LLM metadata generation successful!\n");
                    meta
                }
                Err(e) => {
                    println!("⚠️  LLM generation failed: {}", e);
                    println!("   Using fallback heuristics instead...\n");
                    generate_fallback_metadata(test_content)
                }
            }
        }
        Err(e) => {
            println!("⚠️  LLM model not available: {}", e);
            println!("   Using fallback heuristics instead...\n");
            generate_fallback_metadata(test_content)
        }
    };

    // Step 4: Display generated metadata
    println!("📊 Generated Metadata:");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();
    println!("📝 Semantic Title:");
    println!("   {}", metadata.semantic_title);
    println!();
    println!("📂 Category: {}", metadata.category);
    println!("🎯 Difficulty: {}", metadata.difficulty);
    println!();
    println!("📋 Summary:");
    println!("   {}", metadata.summary);
    println!();
    println!("🔑 Keywords:");
    println!("   {}", metadata.keywords.join(", "));
    println!();
    println!("💡 Concepts:");
    println!("   {}", metadata.concepts.join(", "));
    println!();
    println!("🎯 Intent:");
    println!("   {}", metadata.intent);
    println!();
    println!("🔍 Suggested Queries:");
    for query in &metadata.suggested_queries {
        println!("   - {}", query);
    }
    println!();

    // Step 5: Test database integration
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("💾 Testing Database Integration...\n");

    let db = Database::open_in_memory()?;
    db.initialize()?;

    // Add collection
    db.add_collection("test-docs", "/tmp/test", "**/*.md", "file", None)?;
    println!("✅ Collection created");

    // Compute hash
    let hash = agentroot_core::db::hash_content(test_content);

    // Store content
    db.insert_content(&hash, test_content)?;
    println!("✅ Content stored (hash: {})", &hash[..8]);

    // Store document with metadata using internal method
    // (In production, use reindex_collection_with_metadata instead)
    use agentroot_core::db::DocumentInsert;

    let now = chrono::Utc::now().to_rfc3339();
    let keywords_json = serde_json::to_string(&metadata.keywords)?;
    let concepts_json = serde_json::to_string(&metadata.concepts)?;
    let queries_json = serde_json::to_string(&metadata.suggested_queries)?;

    let doc_insert = DocumentInsert::new(
        "test-docs",
        "getting-started.md",
        "Getting Started with Rust",
        &hash,
        &now,
        &now,
    )
    .with_source_type("file")
    .with_llm_metadata_strings(
        &metadata.summary,
        &metadata.semantic_title,
        &keywords_json,
        &metadata.category,
        &metadata.intent,
        &concepts_json,
        &metadata.difficulty,
        &queries_json,
        "metadata-generator-example",
        &now,
    );

    db.insert_doc(&doc_insert)?;
    println!("✅ Document stored with metadata");

    // Verify metadata was stored
    let stats = db.get_stats()?;
    println!("✅ Database stats:");
    println!("   - Documents: {}", stats.document_count);
    println!("   - With metadata: {}", stats.metadata_count);
    println!();

    // Step 6: Test search integration
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("🔍 Testing Search Integration...\n");

    // Search by keyword
    let search_opts = agentroot_core::SearchOptions {
        limit: 10,
        min_score: 0.0,
        collection: None,
        provider: None,
        full_content: false,
        metadata_filters: Vec::new(),
    };

    let results = db.search_fts("Rust programming", &search_opts)?;
    println!(
        "✅ BM25 search for 'Rust programming': {} results",
        results.len()
    );

    if let Some(result) = results.first() {
        println!("   Top result:");
        println!("   - Title: {}", result.title);
        println!("   - Score: {:.4}", result.score);
        if let Some(ref summary) = result.llm_summary {
            println!("   - Summary: {}...", &summary[..80.min(summary.len())]);
        }
        if let Some(ref keywords) = result.llm_keywords {
            println!("   - Keywords: {}", keywords.join(", "));
        }
    }
    println!();

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("✅ Example Complete!");
    println!();
    println!("📝 Summary:");
    println!("   - Metadata generated successfully");
    println!("   - All 8 metadata fields populated");
    println!("   - Stored in database with schema v4");
    println!("   - FTS search includes metadata fields");
    println!("   - System gracefully handles LLM unavailability");
    println!();

    Ok(())
}

/// Fallback metadata generation using heuristics
fn generate_fallback_metadata(content: &str) -> DocumentMetadata {
    // Extract title from first heading
    let title = content
        .lines()
        .find(|line| line.starts_with('#'))
        .map(|line| line.trim_start_matches('#').trim())
        .unwrap_or("Untitled")
        .to_string();

    // Extract summary from first paragraph
    let summary = content
        .lines()
        .skip_while(|line| line.trim().is_empty() || line.starts_with('#'))
        .take_while(|line| !line.trim().is_empty())
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(200)
        .collect::<String>();

    // Extract keywords from content (simple word frequency)
    let words: Vec<&str> = content
        .split_whitespace()
        .filter(|w| w.len() > 4 && w.chars().all(|c| c.is_alphanumeric()))
        .take(50)
        .collect();

    let mut word_counts = std::collections::HashMap::new();
    for word in words {
        *word_counts.entry(word.to_lowercase()).or_insert(0) += 1;
    }

    let mut keywords: Vec<String> = word_counts
        .into_iter()
        .filter(|(_, count)| *count > 1)
        .map(|(word, _)| word)
        .take(8)
        .collect();

    if keywords.is_empty() {
        keywords = vec!["programming".to_string(), "tutorial".to_string()];
    }

    // Extract concepts (capitalized words)
    let concepts: Vec<String> = content
        .split_whitespace()
        .filter(|w| w.len() > 2 && w.chars().next().map_or(false, |c| c.is_uppercase()))
        .map(|w| w.to_string())
        .take(10)
        .collect();

    DocumentMetadata {
        summary,
        semantic_title: title.clone(),
        keywords,
        category: "documentation".to_string(),
        intent: "Educational content for learning programming concepts".to_string(),
        concepts,
        difficulty: "beginner".to_string(),
        suggested_queries: vec![
            title.to_lowercase(),
            "getting started tutorial".to_string(),
            "programming guide".to_string(),
        ],
        extracted_concepts: vec![],
    }
}
