//! Comprehensive benchmark to demonstrate glossary effectiveness
//!
//! Methodology:
//! 1. Index larger corpus (10+ documents)
//! 2. Test with abstract queries where glossary should help
//! 3. Compare: Baseline (BM25) vs With Glossary (BM25 + GlossarySearch)
//! 4. Measure: Recall@10, Precision, Query Expansion Success
//!
//! Usage:
//!   export AGENTROOT_LLM_URL="https://your-endpoint"
//!   export AGENTROOT_LLM_MODEL="Qwen/Qwen2.5-7B-Instruct"
//!   cargo run --release --example glossary_benchmark

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

struct BenchmarkQuery {
    query: &'static str,
    description: &'static str,
    expected_docs: Vec<&'static str>,
    query_type: &'static str,
}

struct BenchmarkResults {
    query: String,
    query_type: String,
    baseline_recall: f64,
    glossary_recall: f64,
    baseline_docs: usize,
    glossary_docs: usize,
    new_docs_found: usize,
    improvement: f64,
}

#[tokio::main]
async fn main() {
    println!("\n╔════════════════════════════════════════════════════════════╗");
    println!("║          Intelligent Glossary Benchmark                   ║");
    println!("║   Measuring Search Quality Improvement                    ║");
    println!("╚════════════════════════════════════════════════════════════╝\n");

    let llm_url = match env::var("AGENTROOT_LLM_URL") {
        Ok(url) => url,
        Err(_) => {
            eprintln!("❌ AGENTROOT_LLM_URL not set. Using mock mode.");
            eprintln!("   Set environment variables for real LLM testing:\n");
            eprintln!("   export AGENTROOT_LLM_URL=\"https://your-endpoint\"");
            eprintln!("   export AGENTROOT_LLM_MODEL=\"Qwen/Qwen2.5-7B-Instruct\"\n");
            std::process::exit(1);
        }
    };

    let llm_model = env::var("AGENTROOT_LLM_MODEL")
        .unwrap_or_else(|_| "Qwen/Qwen2.5-7B-Instruct".to_string());

    println!("📋 Configuration:");
    println!("  LLM: {}", llm_model);
    println!("  Endpoint: {}\n", llm_url);

    let config = LLMServiceConfig {
        url: llm_url,
        model: llm_model.clone(),
        embedding_url: None,
        embedding_model: "intfloat/e5-mistral-7b-instruct".to_string(),
        embedding_dimensions: Some(4096),
        api_key: None,
        timeout_secs: 60,
    };

    let client = VLLMClient::new(config).unwrap();
    let generator = agentroot_core::llm::HttpMetadataGenerator::new(Arc::new(client));

    println!("▶ Phase 1: Indexing Document Corpus");
    println!("  Scanning for markdown files...");

    let db = Database::open_in_memory().unwrap();
    db.initialize().unwrap();
    db.ensure_vec_table(128).unwrap();
    db.add_collection("agentroot", ".", "**/*.md", "file", None).unwrap();

    let mut docs_to_index = Vec::new();
    
    // Add core documentation
    for doc in &["README.md", "AGENTS.md", "CHANGELOG.md", "CONTRIBUTING.md"] {
        if Path::new(doc).exists() {
            docs_to_index.push((*doc, format!("Root: {}", doc)));
        }
    }

    // Add examples directory docs
    if Path::new("examples/README.md").exists() {
        docs_to_index.push(("examples/README.md", "Examples documentation".to_string()));
    }

    // Add docs directory
    for entry in fs::read_dir("docs").ok().into_iter().flatten() {
        if let Ok(entry) = entry {
            if let Some(ext) = entry.path().extension() {
                if ext == "md" {
                    let path = entry.path().display().to_string();
                    docs_to_index.push((
                        Box::leak(path.clone().into_boxed_str()) as &'static str,
                        format!("Doc: {}", entry.file_name().to_string_lossy())
                    ));
                }
            }
        }
    }

    println!("  Found {} documents to index\n", docs_to_index.len());

    let mut indexed = 0;
    let mut total_concepts = 0;

    for (path, desc) in &docs_to_index {
        print!("  • Indexing {} ... ", desc);
        std::io::Write::flush(&mut std::io::stdout()).unwrap();

        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => {
                println!("⚠ Failed to read");
                continue;
            }
        };

        if content.trim().is_empty() {
            println!("⚠ Empty");
            continue;
        }

        let truncated = if content.len() > 50000 {
            content.chars().take(50000).collect::<String>()
        } else {
            content.clone()
        };

        let hash = hash_content(&content);
        db.insert_content(&hash, &content).unwrap();

        let context = MetadataContext::new("file".to_string(), "agentroot".to_string())
            .with_extension("md".to_string())
            .with_timestamps(chrono::Utc::now().to_rfc3339(), chrono::Utc::now().to_rfc3339());

        let metadata = match generator.generate_metadata(&truncated, &context).await {
            Ok(m) => m,
            Err(e) => {
                println!("⚠ LLM failed: {}", e);
                continue;
            }
        };

        let concepts_count = metadata.extracted_concepts.len();
        total_concepts += concepts_count;
        println!("✓ ({} concepts)", concepts_count);

        db.insert_doc(
            &agentroot_core::db::DocumentInsert::new(
                "agentroot", path, &metadata.semantic_title, &hash,
                &chrono::Utc::now().to_rfc3339(), &chrono::Utc::now().to_rfc3339()
            ).with_llm_metadata_strings(
                &metadata.summary, &metadata.semantic_title,
                &serde_json::to_string(&metadata.keywords).unwrap(),
                &metadata.category, &metadata.intent,
                &serde_json::to_string(&metadata.concepts).unwrap(),
                &metadata.difficulty,
                &serde_json::to_string(&metadata.suggested_queries).unwrap(),
                &llm_model, &chrono::Utc::now().to_rfc3339()
            )
        ).unwrap();

        let chunk_hash = blake3::hash(content.as_bytes()).to_hex().to_string();
        db.insert_chunk_embedding(&hash, 0, 0, &chunk_hash, "test", &vec![0.1; 128]).unwrap();

        for concept in &metadata.extracted_concepts {
            let cid = db.upsert_concept(&concept.term).unwrap();
            db.link_concept_to_chunk(cid, &chunk_hash, &hash, &concept.snippet).unwrap();
            db.update_concept_stats(cid).unwrap();
        }

        indexed += 1;
    }

    let (unique_concepts, concept_links) = db.get_concept_stats().unwrap();
    println!("\n  ✓ Indexed {} documents", indexed);
    println!("  ✓ Extracted {} unique concepts", unique_concepts);
    println!("  ✓ Created {} concept links", concept_links);

    if indexed < 5 {
        eprintln!("\n⚠ Warning: Only {} documents indexed. Need 10+ for meaningful results.\n", indexed);
    }

    println!("\n▶ Phase 2: Benchmark Queries");
    println!("  Testing abstract queries where glossary should provide value\n");

    let benchmark_queries = vec![
        BenchmarkQuery {
            query: "search",
            description: "Generic term mapping to 'semantic search', 'hybrid search'",
            expected_docs: vec!["README.md", "AGENTS.md"],
            query_type: "Broad Term",
        },
        BenchmarkQuery {
            query: "chunking",
            description: "Technical term mapping to 'AST-aware chunking'",
            expected_docs: vec!["README.md", "docs/semantic-chunking.md"],
            query_type: "Technical Term",
        },
        BenchmarkQuery {
            query: "provider",
            description: "Architecture term mapping to 'provider system', 'URLProvider', 'PDFProvider'",
            expected_docs: vec!["AGENTS.md", "CHANGELOG.md", "docs/providers.md"],
            query_type: "Architecture Term",
        },
        BenchmarkQuery {
            query: "glossary",
            description: "Feature term mapping to 'intelligent glossary'",
            expected_docs: vec!["CHANGELOG.md"],
            query_type: "Feature Term",
        },
        BenchmarkQuery {
            query: "BM25",
            description: "Exact algorithm name (should work with both baseline and glossary)",
            expected_docs: vec!["README.md", "AGENTS.md"],
            query_type: "Specific Term",
        },
    ];

    let mut all_results = Vec::new();

    for test in &benchmark_queries {
        println!("  Query: \"{}\"", test.query);
        println!("    Type: {}", test.query_type);
        println!("    {}", test.description);

        let baseline_workflow = Workflow {
            steps: vec![WorkflowStep::Bm25Search {
                query: test.query.to_string(),
                limit: 10,
            }],
            reasoning: "Baseline".to_string(),
            expected_results: 10,
            complexity: "simple".to_string(),
        };

        let glossary_workflow = Workflow {
            steps: vec![
                WorkflowStep::Bm25Search { query: test.query.to_string(), limit: 10 },
                WorkflowStep::GlossarySearch { query: test.query.to_string(), limit: 10, min_confidence: 0.3 },
                WorkflowStep::Merge { strategy: MergeStrategy::Append },
                WorkflowStep::Deduplicate,
            ],
            reasoning: "With Glossary".to_string(),
            expected_results: 10,
            complexity: "simple".to_string(),
        };

        let options = SearchOptions::default();
        
        let baseline_results = execute_workflow(&db, &baseline_workflow, test.query, &options).await.unwrap();
        let glossary_results = execute_workflow(&db, &glossary_workflow, test.query, &options).await.unwrap();

        let baseline_docs: HashSet<_> = baseline_results.iter().map(|r| r.filepath.as_str()).collect();
        let glossary_docs: HashSet<_> = glossary_results.iter().map(|r| r.filepath.as_str()).collect();
        let expected: HashSet<_> = test.expected_docs.iter().copied().collect();

        let baseline_hits = baseline_docs.intersection(&expected).count();
        let glossary_hits = glossary_docs.intersection(&expected).count();

        let baseline_recall = baseline_hits as f64 / test.expected_docs.len() as f64;
        let glossary_recall = glossary_hits as f64 / test.expected_docs.len() as f64;

        let new_docs = glossary_docs.difference(&baseline_docs).count();
        let improvement = ((glossary_recall - baseline_recall) / baseline_recall.max(0.01)) * 100.0;

        println!("    Baseline: {}/{} docs (recall: {:.1}%)",
            baseline_results.len(), baseline_hits, baseline_recall * 100.0);
        println!("    With Glossary: {}/{} docs (recall: {:.1}%)",
            glossary_results.len(), glossary_hits, glossary_recall * 100.0);
        println!("    New docs discovered: {}", new_docs);
        println!("    Improvement: {:.1}%\n", improvement);

        all_results.push(BenchmarkResults {
            query: test.query.to_string(),
            query_type: test.query_type.to_string(),
            baseline_recall,
            glossary_recall,
            baseline_docs: baseline_results.len(),
            glossary_docs: glossary_results.len(),
            new_docs_found: new_docs,
            improvement,
        });
    }

    println!("\n▶ Phase 3: Aggregate Results");
    println!("\n╔═══════════════════════════════════════════════════════════════╗");
    println!("║                    BENCHMARK SUMMARY                          ║");
    println!("╚═══════════════════════════════════════════════════════════════╝\n");

    let avg_baseline_recall = all_results.iter().map(|r| r.baseline_recall).sum::<f64>() / all_results.len() as f64;
    let avg_glossary_recall = all_results.iter().map(|r| r.glossary_recall).sum::<f64>() / all_results.len() as f64;
    let total_new_docs = all_results.iter().map(|r| r.new_docs_found).sum::<usize>();
    let avg_improvement = all_results.iter().map(|r| r.improvement).sum::<f64>() / all_results.len() as f64;

    println!("  📊 Overall Metrics:");
    println!("    Documents Indexed: {}", indexed);
    println!("    Unique Concepts: {}", unique_concepts);
    println!("    Queries Tested: {}\n", all_results.len());

    println!("  📈 Recall Performance:");
    println!("    Baseline (BM25 only): {:.1}%", avg_baseline_recall * 100.0);
    println!("    With Glossary: {:.1}%", avg_glossary_recall * 100.0);
    println!("    Absolute Gain: +{:.1} percentage points\n", (avg_glossary_recall - avg_baseline_recall) * 100.0);

    println!("  🔍 Discovery:");
    println!("    Total new documents found: {}", total_new_docs);
    println!("    Avg new docs per query: {:.1}\n", total_new_docs as f64 / all_results.len() as f64);

    println!("  💯 Improvement:");
    println!("    Average improvement: {:.1}%\n", avg_improvement);

    println!("  📋 Per-Query Breakdown:");
    println!("  ┌─────────────────────────┬──────────┬──────────┬──────────┐");
    println!("  │ Query                   │ Baseline │ Glossary │ Improve  │");
    println!("  ├─────────────────────────┼──────────┼──────────┼──────────┤");
    for result in &all_results {
        println!("  │ {:<23} │ {:>6.1}%  │ {:>6.1}%  │ {:>6.1}% │",
            &result.query[..result.query.len().min(23)],
            result.baseline_recall * 100.0,
            result.glossary_recall * 100.0,
            result.improvement
        );
    }
    println!("  └─────────────────────────┴──────────┴──────────┴──────────┘\n");

    println!("  🎯 Conclusion:");
    if avg_improvement > 20.0 {
        println!("    ✅ EXCELLENT: Glossary provides significant improvement");
        println!("       Semantic concept mapping is highly effective");
    } else if avg_improvement > 10.0 {
        println!("    ✓ GOOD: Glossary provides measurable improvement");
        println!("       Concept extraction working as designed");
    } else if avg_improvement > 0.0 {
        println!("    ○ MARGINAL: Small improvement detected");
        println!("       Consider: more documents, better concepts, tuned confidence");
    } else {
        println!("    ⚠ NO IMPROVEMENT: Glossary not providing value");
        println!("       Possible reasons:");
        println!("       - Corpus too small (need 10-20+ documents)");
        println!("       - Queries too specific (need more abstract terms)");
        println!("       - Concepts don't match query semantics");
    }

    println!("\n╔═══════════════════════════════════════════════════════════════╗");
    println!("║                  BENCHMARK COMPLETE                           ║");
    println!("╚═══════════════════════════════════════════════════════════════╝\n");
}
