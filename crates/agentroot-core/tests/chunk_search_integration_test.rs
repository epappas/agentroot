//! End-to-end integration test for chunk-level search
//!
//! Tests:
//! 1. Chunk creation during indexing
//! 2. Chunk-level BM25 search
//! 3. Chunk-level vector search (if embeddings exist)
//! 4. Chunk metadata quality
//! 5. Label filtering
//! 6. Chunk navigation
//! 7. Comparison: chunk vs document search

use agentroot_core::{Database, SearchOptions};

#[tokio::test]
async fn test_chunks_created_for_documents() {
    let db = Database::default_path();
    let db = Database::open(&db).unwrap();
    db.initialize().unwrap();

    // Get a sample document from agentroot-src collection
    let docs = db.get_documents_in_collection("agentroot-src").unwrap();

    if docs.is_empty() {
        println!("⚠️  No documents in agentroot-src collection. Run indexing first.");
        return;
    }

    println!("📊 Testing chunk creation for {} documents", docs.len());

    let mut total_chunks = 0;
    let mut docs_with_chunks = 0;
    let mut chunks_with_metadata = 0;

    for doc in docs.iter().take(10) {
        let chunks = db.get_chunks_for_document(&doc.hash).unwrap();

        if !chunks.is_empty() {
            docs_with_chunks += 1;
            total_chunks += chunks.len();

            // Count chunks with LLM metadata
            for chunk in &chunks {
                if chunk.llm_summary.is_some() || chunk.llm_purpose.is_some() {
                    chunks_with_metadata += 1;
                }
            }

            println!(
                "  ✓ Document {} has {} chunks ({})",
                &doc.hash[..8],
                chunks.len(),
                doc.path
            );

            // Show first chunk details
            if let Some(first_chunk) = chunks.first() {
                println!(
                    "    First chunk: {} ({})",
                    first_chunk
                        .breadcrumb
                        .as_ref()
                        .unwrap_or(&"<no breadcrumb>".to_string()),
                    first_chunk
                        .chunk_type
                        .as_ref()
                        .unwrap_or(&"<no type>".to_string())
                );
                if let Some(summary) = &first_chunk.llm_summary {
                    println!("    Summary: {}", summary);
                }
                if !first_chunk.llm_labels.is_empty() {
                    println!("    Labels: {:?}", first_chunk.llm_labels);
                }
            }
        }
    }

    println!("\n📈 Chunk Statistics:");
    println!(
        "  Documents with chunks: {}/{}",
        docs_with_chunks,
        docs.len().min(10)
    );
    println!("  Total chunks: {}", total_chunks);
    println!("  Chunks with metadata: {}", chunks_with_metadata);

    assert!(
        docs_with_chunks > 0,
        "At least some documents should have chunks"
    );
}

#[tokio::test]
async fn test_chunk_search_quality() {
    let db = Database::default_path();
    let db = Database::open(&db).unwrap();
    db.initialize().unwrap();

    let test_queries = vec!["search", "database", "embed", "metadata", "collection"];

    println!("\n🔍 Testing Chunk Search Quality\n");

    for query in test_queries {
        let options = SearchOptions {
            limit: 5,
            min_score: 0.0,
            collection: Some("agentroot-src".to_string()),
            provider: None,
            metadata_filters: Vec::new(),
            detail: agentroot_core::DetailLevel::L2,
            ..Default::default()
        };

        let chunk_results = db.search_chunks_bm25(query, &options).unwrap();

        println!("Query: \"{}\"", query);
        println!("  Chunks found: {}", chunk_results.len());

        for (i, result) in chunk_results.iter().take(3).enumerate() {
            println!(
                "  {}. {} (score: {:.2})",
                i + 1,
                result
                    .chunk_breadcrumb
                    .as_ref()
                    .unwrap_or(&"<no breadcrumb>".to_string()),
                result.score
            );
            println!(
                "     {} @ {}:{}",
                result.display_path,
                result.chunk_start_line.unwrap_or(0),
                result.chunk_end_line.unwrap_or(0)
            );
            if let Some(summary) = &result.chunk_summary {
                println!("     Summary: {}", summary);
            }
        }
        println!();
    }
}

#[tokio::test]
async fn test_chunk_vs_document_search() {
    let db = Database::default_path();
    let db = Database::open(&db).unwrap();
    db.initialize().unwrap();

    let query = "search database";
    let options = SearchOptions {
        limit: 10,
        min_score: 0.0,
        collection: Some("agentroot-src".to_string()),
        provider: None,
        metadata_filters: Vec::new(),
        ..Default::default()
    };

    println!("\n⚖️  Comparing Chunk vs Document Search\n");
    println!("Query: \"{}\"", query);

    // Document-level search
    let doc_results = db.search_fts(query, &options).unwrap();
    println!("\n📄 Document-level search:");
    println!("  Results: {}", doc_results.len());
    for (i, result) in doc_results.iter().take(3).enumerate() {
        println!(
            "  {}. {} (score: {:.2})",
            i + 1,
            result.display_path,
            result.score
        );
    }

    // Chunk-level search
    let chunk_results = db.search_chunks_bm25(query, &options).unwrap();
    println!("\n🧩 Chunk-level search:");
    println!("  Results: {}", chunk_results.len());
    for (i, result) in chunk_results.iter().take(3).enumerate() {
        println!(
            "  {}. {} @ {} (score: {:.2})",
            i + 1,
            result
                .chunk_breadcrumb
                .as_ref()
                .unwrap_or(&"<no breadcrumb>".to_string()),
            result.display_path,
            result.score
        );
        println!(
            "     Lines: {}-{}",
            result.chunk_start_line.unwrap_or(0),
            result.chunk_end_line.unwrap_or(0)
        );
    }

    println!("\n💡 Analysis:");
    println!("  Document results: {}", doc_results.len());
    println!("  Chunk results: {}", chunk_results.len());
    println!(
        "  Precision improvement: {}x more granular",
        if doc_results.is_empty() {
            1.0
        } else {
            chunk_results.len() as f64 / doc_results.len() as f64
        }
    );
}

#[tokio::test]
async fn test_chunk_metadata_quality() {
    let db = Database::default_path();
    let db = Database::open(&db).unwrap();
    db.initialize().unwrap();

    let options = SearchOptions {
        limit: 10,
        min_score: 0.0,
        collection: Some("agentroot-src".to_string()),
        provider: None,
        metadata_filters: Vec::new(),
        detail: agentroot_core::DetailLevel::L2,
        ..Default::default()
    };

    let results = db.search_chunks_bm25("Database", &options).unwrap();

    println!("\n🎯 Testing Chunk Metadata Quality\n");
    println!("Found {} chunks mentioning 'Database'\n", results.len());

    let mut with_summary = 0;
    let mut with_purpose = 0;
    let mut with_concepts = 0;
    let mut with_labels = 0;

    for result in results.iter().take(5) {
        println!(
            "Chunk: {}",
            result
                .chunk_breadcrumb
                .as_ref()
                .unwrap_or(&"<no breadcrumb>".to_string())
        );

        if let Some(summary) = &result.chunk_summary {
            with_summary += 1;
            println!("  Summary: {}", summary);
        }

        if let Some(purpose) = &result.chunk_purpose {
            with_purpose += 1;
            println!("  Purpose: {}", purpose);
        }

        if !result.chunk_concepts.is_empty() {
            with_concepts += 1;
            println!("  Concepts: {:?}", result.chunk_concepts);
        }

        if !result.chunk_labels.is_empty() {
            with_labels += 1;
            println!("  Labels: {:?}", result.chunk_labels);
        }

        println!();
    }

    println!("📊 Metadata Coverage:");
    println!("  With summary: {}/{}", with_summary, results.len().min(5));
    println!("  With purpose: {}/{}", with_purpose, results.len().min(5));
    println!(
        "  With concepts: {}/{}",
        with_concepts,
        results.len().min(5)
    );
    println!("  With labels: {}/{}", with_labels, results.len().min(5));
}

#[tokio::test]
async fn test_label_filtering() {
    let db = Database::default_path();
    let db = Database::open(&db).unwrap();
    db.initialize().unwrap();

    println!("\n🏷️  Testing Label Filtering\n");

    // First, find what labels exist
    let all_chunks_options = SearchOptions {
        limit: 50,
        min_score: 0.0,
        collection: Some("agentroot-src".to_string()),
        provider: None,
        metadata_filters: Vec::new(),
        ..Default::default()
    };

    let all_results = db.search_chunks_bm25("fn", &all_chunks_options).unwrap();

    let mut label_counts = std::collections::HashMap::new();
    for result in &all_results {
        for (key, value) in &result.chunk_labels {
            let label = format!("{}:{}", key, value);
            *label_counts.entry(label).or_insert(0) += 1;
        }
    }

    println!("Available labels:");
    for (label, count) in label_counts.iter().take(10) {
        println!("  {} ({})", label, count);
    }

    // Test filtering by a specific label if any exist
    if let Some((label, _)) = label_counts.iter().next() {
        let mut filtered_options = all_chunks_options.clone();
        filtered_options
            .metadata_filters
            .push(("label".to_string(), label.clone()));
        filtered_options.limit = 5;

        let filtered_results = db.search_chunks_bm25("", &filtered_options).unwrap();

        println!("\nFiltering by label '{}':", label);
        println!("  Results: {}", filtered_results.len());

        for result in filtered_results.iter().take(3) {
            println!(
                "  - {} @ {}",
                result
                    .chunk_breadcrumb
                    .as_ref()
                    .unwrap_or(&"<no breadcrumb>".to_string()),
                result.display_path
            );
        }
    } else {
        println!("⚠️  No labels found in chunks. LLM metadata may not be generated yet.");
    }
}

#[tokio::test]
async fn test_chunk_navigation() {
    let db = Database::default_path();
    let db = Database::open(&db).unwrap();
    db.initialize().unwrap();

    println!("\n🧭 Testing Chunk Navigation\n");

    // Get a document with multiple chunks
    let docs = db.get_documents_in_collection("agentroot-src").unwrap();

    for doc in docs.iter().take(5) {
        let chunks = db.get_chunks_for_document(&doc.hash).unwrap();

        if chunks.len() >= 3 {
            println!("Document: {} ({} chunks)", doc.path, chunks.len());

            // Test navigation from middle chunk
            if let Some(middle_chunk) = chunks.get(1) {
                let (prev, next) = db.get_surrounding_chunks(&middle_chunk.hash).unwrap();

                if let Some(ref prev_chunk) = prev {
                    println!(
                        "  Previous: {}",
                        prev_chunk
                            .breadcrumb
                            .as_ref()
                            .unwrap_or(&"<no breadcrumb>".to_string())
                    );
                }

                println!(
                    "  Current:  {}",
                    middle_chunk
                        .breadcrumb
                        .as_ref()
                        .unwrap_or(&"<no breadcrumb>".to_string())
                );

                if let Some(ref next_chunk) = next {
                    println!(
                        "  Next:     {}",
                        next_chunk
                            .breadcrumb
                            .as_ref()
                            .unwrap_or(&"<no breadcrumb>".to_string())
                    );
                }

                assert!(prev.is_some(), "Should have previous chunk");
                assert!(next.is_some(), "Should have next chunk");

                println!("  ✓ Navigation working correctly\n");
                break;
            }
        }
    }
}

#[tokio::test]
async fn test_performance_metrics() {
    let db = Database::default_path();
    let db = Database::open(&db).unwrap();
    db.initialize().unwrap();

    println!("\n⚡ Performance Metrics\n");

    let query = "search database collection";
    let options = SearchOptions {
        limit: 20,
        min_score: 0.0,
        collection: Some("agentroot-src".to_string()),
        provider: None,
        metadata_filters: Vec::new(),
        ..Default::default()
    };

    // Document search timing
    let start = std::time::Instant::now();
    let doc_results = db.search_fts(query, &options).unwrap();
    let doc_duration = start.elapsed();

    // Chunk search timing
    let start = std::time::Instant::now();
    let chunk_results = db.search_chunks_bm25(query, &options).unwrap();
    let chunk_duration = start.elapsed();

    println!("Document search:");
    println!("  Results: {}", doc_results.len());
    println!("  Time: {:?}", doc_duration);

    println!("\nChunk search:");
    println!("  Results: {}", chunk_results.len());
    println!("  Time: {:?}", chunk_duration);

    println!("\nPerformance comparison:");
    println!(
        "  Speedup: {:.2}x",
        doc_duration.as_secs_f64() / chunk_duration.as_secs_f64().max(0.001)
    );
    println!(
        "  Granularity: {:.2}x more results",
        chunk_results.len() as f64 / doc_results.len().max(1) as f64
    );
}

#[tokio::test]
async fn test_concept_linking() {
    let db = Database::default_path();
    let db = Database::open(&db).unwrap();
    db.initialize().unwrap();

    println!("\n🔗 Testing Concept Linking\n");

    // Check if concepts are linked to chunks
    let concepts = db.list_concepts().unwrap();

    if concepts.is_empty() {
        println!("⚠️  No concepts found. Glossary may not be generated yet.");
        return;
    }

    println!("Found {} concepts in glossary", concepts.len());

    // Test a few concepts
    for concept in concepts.iter().take(5) {
        let chunk_links = db.get_chunks_for_concept(concept.id).unwrap();

        if !chunk_links.is_empty() {
            println!("\nConcept: \"{}\"", concept.term);
            println!("  Linked to {} chunks", chunk_links.len());

            for link in chunk_links.iter().take(3) {
                println!("  - Chunk: {}", link.chunk_hash);
                println!("    Snippet: {}", link.snippet);
            }
        }
    }
}
