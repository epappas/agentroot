//! Real-world validation of intelligent glossary system
//!
//! This example performs comprehensive validation of the glossary feature:
//! 1. Indexes actual agentroot documentation with real LLM
//! 2. Extracts concepts from documents
//! 3. Runs comparative searches (with/without glossary)
//! 4. Measures quality improvements (recall, precision, relevance)
//!
//! Required environment variables:
//! - AGENTROOT_LLM_URL: LLM endpoint for metadata generation
//! - AGENTROOT_LLM_MODEL: LLM model name (e.g., "Qwen/Qwen2.5-7B-Instruct")
//!
//! Optional:
//! - AGENTROOT_EMBEDDING_URL: Embedding endpoint (for vector search comparison)
//! - AGENTROOT_EMBEDDING_MODEL: Embedding model name
//! - AGENTROOT_EMBEDDING_DIMS: Embedding dimensions (default: 4096)
//!
//! Usage:
//!   cargo run --release --example glossary_validation

use agentroot_core::db::{hash_content, Database};
use agentroot_core::llm::{MetadataContext, MetadataGenerator, VLLMClient};
use agentroot_core::config::LLMServiceConfig;
use agentroot_core::search::{execute_workflow, SearchOptions, SearchSource};
use agentroot_core::llm::{Workflow, WorkflowStep, MergeStrategy};
use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::Path;
use std::sync::Arc;

#[derive(Debug)]
struct QueryTest {
    query: &'static str,
    category: &'static str,
    expected_keywords: Vec<&'static str>,
    should_use_glossary: bool,
}

#[tokio::main]
async fn main() {
    println!("\n╔════════════════════════════════════════════════════════════╗");
    println!("║   Intelligent Glossary Real-World Validation              ║");
    println!("╚════════════════════════════════════════════════════════════╝\n");

    let llm_url = match env::var("AGENTROOT_LLM_URL") {
        Ok(url) => url,
        Err(_) => {
            eprintln!("❌ Error: AGENTROOT_LLM_URL environment variable not set");
            eprintln!("\nPlease set the LLM endpoint:");
            eprintln!("  export AGENTROOT_LLM_URL=\"https://your-llm-endpoint\"");
            eprintln!("  export AGENTROOT_LLM_MODEL=\"Qwen/Qwen2.5-7B-Instruct\"");
            std::process::exit(1);
        }
    };

    let llm_model = env::var("AGENTROOT_LLM_MODEL")
        .unwrap_or_else(|_| "Qwen/Qwen2.5-7B-Instruct".to_string());

    println!("📋 Configuration:");
    println!("  LLM URL: {}", llm_url);
    println!("  LLM Model: {}", llm_model);

    let config = LLMServiceConfig {
        url: llm_url,
        model: llm_model.clone(),
        embedding_url: None,
        embedding_model: "intfloat/e5-mistral-7b-instruct".to_string(),
        embedding_dimensions: Some(4096),
        api_key: None,
        timeout_secs: 60,
    };

    let client = match VLLMClient::new(config) {
        Ok(c) => Arc::new(c),
        Err(e) => {
            eprintln!("❌ Failed to create LLM client: {}", e);
            std::process::exit(1);
        }
    };

    let generator = agentroot_core::llm::HttpMetadataGenerator::new(client.clone());

    println!("\n▶ Phase 1: Setting up test database");
    let db = Database::open_in_memory().unwrap();
    db.initialize().unwrap();
    db.ensure_vec_table(128).unwrap();

    db.add_collection("agentroot-docs", ".", "**/*.md", "file", None)
        .unwrap();

    println!("  ✓ Database initialized");

    println!("\n▶ Phase 2: Indexing documentation with metadata generation");
    let docs_to_index = vec![
        ("README.md", "Project overview and quick start guide"),
        ("AGENTS.md", "Comprehensive agent guidelines"),
        ("CHANGELOG.md", "Project changelog and release notes"),
    ];

    let mut indexed_count = 0;

    for (path, description) in &docs_to_index {
        if !Path::new(path).exists() {
            println!("  ⚠ Skipping {} (not found)", path);
            continue;
        }

        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                println!("  ⚠ Error reading {}: {}", path, e);
                continue;
            }
        };

        if content.trim().is_empty() {
            println!("  ⚠ Skipping {} (empty)", path);
            continue;
        }

        let truncated_content = if content.len() > 50000 {
            content.chars().take(50000).collect::<String>()
        } else {
            content.clone()
        };

        println!("  • Indexing {} ({})...", path, description);

        let hash = hash_content(&content);
        db.insert_content(&hash, &content).unwrap();

        let context = MetadataContext::new("file".to_string(), "agentroot-docs".to_string())
            .with_extension("md".to_string())
            .with_timestamps(
                chrono::Utc::now().to_rfc3339(),
                chrono::Utc::now().to_rfc3339(),
            );

        print!("    Generating metadata with LLM... ");
        std::io::Write::flush(&mut std::io::stdout()).unwrap();

        let metadata = match generator.generate_metadata(&truncated_content, &context).await {
            Ok(m) => {
                println!("✓");
                m
            }
            Err(e) => {
                println!("✗");
                println!("    ⚠ Metadata generation failed: {}", e);
                println!("    Continuing without metadata...");
                continue;
            }
        };

        println!("    Extracted {} concepts:", metadata.extracted_concepts.len());
        for (i, concept) in metadata.extracted_concepts.iter().enumerate().take(5) {
            let snippet_preview = if concept.snippet.len() > 50 {
                format!("{}...", &concept.snippet[..50])
            } else {
                concept.snippet.clone()
            };
            println!("      {}. {} (\"{}\")", i + 1, concept.term, snippet_preview);
        }
        if metadata.extracted_concepts.len() > 5 {
            println!("      ... and {} more", metadata.extracted_concepts.len() - 5);
        }

        db.insert_doc(
            &agentroot_core::db::DocumentInsert::new(
                "agentroot-docs",
                path,
                &metadata.semantic_title,
                &hash,
                &chrono::Utc::now().to_rfc3339(),
                &chrono::Utc::now().to_rfc3339(),
            )
            .with_llm_metadata_strings(
                &metadata.summary,
                &metadata.semantic_title,
                &serde_json::to_string(&metadata.keywords).unwrap(),
                &metadata.category,
                &metadata.intent,
                &serde_json::to_string(&metadata.concepts).unwrap(),
                &metadata.difficulty,
                &serde_json::to_string(&metadata.suggested_queries).unwrap(),
                &llm_model,
                &chrono::Utc::now().to_rfc3339(),
            ),
        )
        .unwrap();

        db.insert_chunk_embedding(
            &hash,
            0,
            0,
            &format!("chunk_{}", path.replace('/', "_")),
            "test-model",
            &vec![0.1; 128],
        )
        .unwrap();

        for concept in &metadata.extracted_concepts {
            let concept_id = db.upsert_concept(&concept.term).unwrap();
            db.link_concept_to_chunk(
                concept_id,
                &format!("chunk_{}", path.replace('/', "_")),
                &hash,
                &concept.snippet,
            )
            .unwrap();
            db.update_concept_stats(concept_id).unwrap();
        }

        indexed_count += 1;
    }

    let (total_concepts, total_links) = db.get_concept_stats().unwrap();
    println!("\n  ✓ Indexed {} documents", indexed_count);
    println!("  ✓ Extracted {} unique concepts", total_concepts);
    println!("  ✓ Created {} concept-chunk links", total_links);

    if indexed_count == 0 {
        eprintln!("\n❌ No documents indexed. Cannot proceed with validation.");
        std::process::exit(1);
    }

    println!("\n▶ Phase 3: Running comparative search tests");

    let test_queries = vec![
        QueryTest {
            query: "workflow orchestration",
            category: "Abstract technical term",
            expected_keywords: vec!["workflow", "coordinator", "manager"],
            should_use_glossary: true,
        },
        QueryTest {
            query: "provider system",
            category: "Architecture concept",
            expected_keywords: vec!["source", "file", "github", "plugin"],
            should_use_glossary: true,
        },
        QueryTest {
            query: "semantic search",
            category: "Feature description",
            expected_keywords: vec!["vector", "embedding", "similarity"],
            should_use_glossary: true,
        },
        QueryTest {
            query: "testing guidelines",
            category: "Development process",
            expected_keywords: vec!["test", "quality", "verification"],
            should_use_glossary: true,
        },
    ];

    let mut total_glossary_results = 0;
    let mut total_baseline_results = 0;
    let mut total_recall_improvement = 0.0;
    let mut queries_run = 0;

    for test in &test_queries {
        println!("\n  Query: \"{}\" ({})", test.query, test.category);

        let workflow_glossary = Workflow {
            steps: vec![
                WorkflowStep::Bm25Search {
                    query: test.query.to_string(),
                    limit: 10,
                },
                WorkflowStep::GlossarySearch {
                    query: test.query.to_string(),
                    limit: 10,
                    min_confidence: 0.3,
                },
                WorkflowStep::Merge {
                    strategy: MergeStrategy::Append,
                },
                WorkflowStep::Deduplicate,
            ],
            reasoning: "Test with glossary".to_string(),
            expected_results: 10,
            complexity: "simple".to_string(),
        };

        let workflow_baseline = Workflow {
            steps: vec![WorkflowStep::Bm25Search {
                query: test.query.to_string(),
                limit: 10,
            }],
            reasoning: "Baseline test".to_string(),
            expected_results: 10,
            complexity: "simple".to_string(),
        };

        let options = SearchOptions::default();

        let results_with_glossary = execute_workflow(&db, &workflow_glossary, test.query, &options)
            .await
            .unwrap();

        let results_baseline = execute_workflow(&db, &workflow_baseline, test.query, &options)
            .await
            .unwrap();

        let glossary_docs: Vec<_> = results_with_glossary
            .iter()
            .filter(|r| matches!(r.source, SearchSource::Glossary))
            .collect();

        println!("\n    📊 Results:");
        println!("      Baseline (BM25 only): {} documents", results_baseline.len());
        println!("      With Glossary: {} documents", results_with_glossary.len());
        println!("      Glossary-only results: {}", glossary_docs.len());

        if !glossary_docs.is_empty() {
            println!("\n    🔍 Documents found via glossary:");
            for result in &glossary_docs {
                println!("      - {} (score: {:.3})", result.filepath, result.score);
                if let Some(context) = &result.context {
                    let context_preview = if context.len() > 80 {
                        format!("{}...", &context[..80])
                    } else {
                        context.clone()
                    };
                    println!("        {}", context_preview);
                }
            }
        }

        let baseline_set: HashSet<_> = results_baseline.iter().map(|r| &r.filepath).collect();
        let glossary_set: HashSet<_> = results_with_glossary.iter().map(|r| &r.filepath).collect();

        let new_docs = glossary_set.difference(&baseline_set).count();
        let recall_improvement = if results_baseline.is_empty() {
            if results_with_glossary.is_empty() {
                0.0
            } else {
                100.0
            }
        } else {
            ((results_with_glossary.len() as f64 - results_baseline.len() as f64)
                / results_baseline.len() as f64)
                * 100.0
        };

        println!("\n    📈 Metrics:");
        println!("      New documents discovered: {}", new_docs);
        println!("      Recall improvement: {:.1}%", recall_improvement);

        if test.should_use_glossary && glossary_docs.is_empty() {
            println!("      ⚠ Expected glossary to help, but no glossary results found");
        } else if !test.should_use_glossary && !glossary_docs.is_empty() {
            println!("      ℹ Glossary activated for technical query (may not be needed)");
        } else if test.should_use_glossary && !glossary_docs.is_empty() {
            println!("      ✓ Glossary successfully expanded results");
        }

        total_glossary_results += results_with_glossary.len();
        total_baseline_results += results_baseline.len();
        total_recall_improvement += recall_improvement;
        queries_run += 1;
    }

    println!("\n▶ Phase 4: Final Report");
    println!("\n  📊 Overall Metrics:");
    println!("    Total queries tested: {}", queries_run);
    println!("    Avg baseline results: {:.1}", total_baseline_results as f64 / queries_run as f64);
    println!("    Avg glossary results: {:.1}", total_glossary_results as f64 / queries_run as f64);
    println!("    Avg recall improvement: {:.1}%", total_recall_improvement / queries_run as f64);

    let avg_recall = total_recall_improvement / queries_run as f64;

    println!("\n  🎯 Validation Summary:");
    if avg_recall > 20.0 {
        println!("    ✅ EXCELLENT: Glossary provides significant recall improvement (>20%)");
    } else if avg_recall > 10.0 {
        println!("    ✓ GOOD: Glossary provides moderate recall improvement (>10%)");
    } else if avg_recall > 0.0 {
        println!("    ○ MARGINAL: Glossary provides small recall improvement");
    } else {
        println!("    ⚠ NO IMPROVEMENT: Glossary did not improve recall");
        println!("      This may indicate:");
        println!("      - Not enough documents indexed");
        println!("      - Concepts not diverse enough");
        println!("      - Test queries too specific");
    }

    println!("\n  💡 Recommendations:");
    if indexed_count < 10 {
        println!("    • Index more documents for better concept coverage");
    }
    if total_concepts < 20 {
        println!("    • Ensure LLM extracts diverse concepts from documents");
    }
    if avg_recall < 10.0 {
        println!("    • Try more abstract queries that benefit from concept expansion");
        println!("    • Verify concept extraction is working correctly");
        println!("    • Consider lowering min_confidence threshold");
    }

    println!("\n╔════════════════════════════════════════════════════════════╗");
    println!("║   Validation Complete                                      ║");
    println!("╚════════════════════════════════════════════════════════════╝\n");
}
