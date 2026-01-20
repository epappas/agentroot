//! Direct database query to show metadata fields

use agentroot_core::{Database, LlamaMetadataGenerator, MetadataGenerator};
use std::fs;
use tempfile::TempDir;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("\n🔍 Direct Database Metadata Inspection\n");
    println!("═══════════════════════════════════════════════════════════════\n");

    // Create sample document
    let temp_dir = TempDir::new()?;
    let doc_path = temp_dir.path().join("rust-tutorial.md");
    fs::write(
        &doc_path,
        r#"# Getting Started with Rust

Rust is a systems programming language for beginners who want to learn 
safe, concurrent programming. Perfect for building web servers, command-line 
tools, and embedded systems.

## Key Features
- Memory safety without garbage collection
- Zero-cost abstractions
- Fearless concurrency

This tutorial will teach you Rust fundamentals.
"#,
    )?;

    // Create database
    let db_dir = TempDir::new()?;
    let db_path = db_dir.path().join("demo.sqlite");
    let db = Database::open(&db_path)?;
    db.initialize()?;

    db.add_collection(
        "demo",
        temp_dir.path().to_str().unwrap(),
        "*.md",
        "file",
        Some(r#"{"exclude_hidden":"false"}"#),
    )?;

    println!("📝 Indexing with fallback metadata...\n");

    // Index with metadata (fallback)
    db.reindex_collection_with_metadata("demo", None).await?;

    // Query database directly using rusqlite
    let conn = &db.conn;

    let mut stmt = conn.prepare(
        "SELECT 
            id, collection, path, title,
            llm_summary, llm_title, llm_keywords, 
            llm_category, llm_intent, llm_concepts,
            llm_difficulty, llm_queries,
            llm_metadata_generated_at, llm_model
         FROM documents 
         WHERE collection = 'demo'",
    )?;

    let docs: Vec<_> = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, Option<String>>(6)?,
                row.get::<_, Option<String>>(7)?,
                row.get::<_, Option<String>>(8)?,
                row.get::<_, Option<String>>(9)?,
                row.get::<_, Option<String>>(10)?,
                row.get::<_, Option<String>>(11)?,
                row.get::<_, Option<String>>(12)?,
                row.get::<_, Option<String>>(13)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    for (
        id,
        collection,
        path,
        title,
        summary,
        llm_title,
        keywords,
        category,
        intent,
        concepts,
        difficulty,
        queries,
        generated_at,
        model,
    ) in docs
    {
        println!("📄 Document ID: {}", id);
        println!("   Collection: {}", collection);
        println!("   Path: {}", path);
        println!("   Title: {}", title);
        println!();

        match (
            summary, llm_title, keywords, category, intent, concepts, difficulty, queries,
        ) {
            (Some(s), Some(t), Some(k), Some(c), Some(i), Some(co), Some(d), Some(q)) => {
                println!("   ✅ METADATA GENERATED:");
                println!();
                println!("   📋 Summary:");
                println!("      {}", s);
                println!();
                println!("   🏷️  Semantic Title: {}", t);
                println!("   📂 Category: {}", c);
                println!("   📊 Difficulty: {}", d);
                println!();
                println!("   💡 Intent:");
                println!("      {}", i);
                println!();
                println!("   🔑 Keywords:");
                if let Ok(kw_array) = serde_json::from_str::<Vec<String>>(&k) {
                    for keyword in kw_array {
                        println!("      • {}", keyword);
                    }
                } else {
                    println!("      {}", k);
                }
                println!();
                println!("   🧠 Concepts:");
                if let Ok(concept_array) = serde_json::from_str::<Vec<String>>(&co) {
                    for concept in concept_array {
                        println!("      • {}", concept);
                    }
                } else {
                    println!("      {}", co);
                }
                println!();
                println!("   🔍 Suggested Queries:");
                if let Ok(query_array) = serde_json::from_str::<Vec<String>>(&q) {
                    for query in query_array {
                        println!("      • \"{}\"", query);
                    }
                } else {
                    println!("      {}", q);
                }
                println!();
                if let Some(gen) = generated_at {
                    println!("   🕐 Generated: {}", gen);
                }
                if let Some(m) = model {
                    println!("   🤖 Model: {}", m);
                }
            }
            _ => {
                println!("   ❌ NO METADATA FOUND");
                println!("      (This shouldn't happen with fallback mode)");
            }
        }

        println!();
        println!("   ─────────────────────────────────────────────────────");
    }

    println!("\n═══════════════════════════════════════════════════════════════\n");

    // Show what's in the FTS index
    println!("🔍 FTS Index Contents:\n");

    let mut fts_stmt = conn.prepare(
        "SELECT rowid, llm_summary, llm_title, llm_keywords 
         FROM documents_fts 
         LIMIT 1",
    )?;

    let fts_results: Vec<_> = fts_stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    for (rowid, summary, title, keywords) in fts_results {
        println!("   FTS Row ID: {}", rowid);
        println!("   Indexed Summary: {:?}", summary);
        println!("   Indexed Title: {:?}", title);
        println!("   Indexed Keywords: {:?}", keywords);
    }

    println!("\n═══════════════════════════════════════════════════════════════\n");
    println!("✨ Inspection Complete!\n");

    Ok(())
}
