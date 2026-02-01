//! Interactive demo showing automated metadata generation
//!
//! This example creates sample documents and shows the metadata
//! that gets automatically generated using an external LLM service.
//!
//! Requirements:
//! - Set AGENTROOT_LLM_URL environment variable
//! - Set AGENTROOT_LLM_MODEL environment variable
//! - Running external LLM service (vLLM, Basilica, OpenAI, etc.)
//!
//! Example:
//!   export AGENTROOT_LLM_URL="https://your-service.com/v1"
//!   export AGENTROOT_LLM_MODEL="Qwen/Qwen2.5-7B-Instruct"
//!   cargo run --example metadata_demo

use agentroot_core::{Database, HttpMetadataGenerator, MetadataGenerator, SearchOptions};
use std::fs;
use tempfile::TempDir;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("🎯 Agentroot Metadata Generation Demo\n");
    println!("═══════════════════════════════════════════════════════════════\n");

    // Create temporary directory with sample documents
    let temp_dir = TempDir::new()?;
    println!("📁 Creating sample documents...\n");

    // Document 1: Rust Tutorial
    let rust_doc = temp_dir.path().join("rust-tutorial.md");
    fs::write(
        &rust_doc,
        r#"# Getting Started with Rust

Rust is a systems programming language that runs blazingly fast, prevents 
segfaults, and guarantees thread safety. It accomplishes these goals without 
using a garbage collector or runtime.

## Why Rust?

- **Memory Safety**: Rust's ownership system ensures memory safety at compile time
- **Zero-cost Abstractions**: High-level features with no runtime overhead
- **Concurrency**: Fearless concurrency with compile-time guarantees

## Installation

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

## Your First Program

```rust
fn main() {
    println!("Hello, world!");
}
```

Rust is perfect for systems programming, web servers, embedded devices, 
and command-line tools. It's beginner-friendly yet powerful enough for 
experts.
"#,
    )?;

    // Document 2: Advanced Async Programming
    let async_doc = temp_dir.path().join("async-rust.md");
    fs::write(
        &async_doc,
        r#"# Asynchronous Programming in Rust

This guide covers advanced async/await patterns in Rust using Tokio.

## Overview

Asynchronous programming allows you to write concurrent code that can handle 
thousands of connections efficiently without threads. Rust's async model is 
zero-cost and provides excellent performance.

## Key Concepts

### Futures
A Future represents a computation that may not have completed yet.

### Async/Await
The async/await syntax makes asynchronous code look synchronous:

```rust
async fn fetch_data() -> Result<String> {
    let response = reqwest::get("https://api.example.com")
        .await?
        .text()
        .await?;
    Ok(response)
}
```

### Tokio Runtime
Tokio is the most popular async runtime for Rust.

This is an advanced topic requiring solid understanding of Rust fundamentals.
"#,
    )?;

    // Document 3: Python Basics
    let python_doc = temp_dir.path().join("python-basics.md");
    fs::write(
        &python_doc,
        r#"# Python Programming Basics

Learn Python from scratch! This tutorial is designed for complete beginners.

## What is Python?

Python is a high-level, interpreted programming language known for its 
simplicity and readability. It's perfect for beginners and widely used 
in web development, data science, automation, and AI.

## Hello World

```python
print("Hello, World!")
```

## Variables and Types

```python
name = "Alice"
age = 25
is_student = True
```

Python is dynamically typed, so you don't declare variable types.

## Lists and Loops

```python
fruits = ["apple", "banana", "cherry"]
for fruit in fruits:
    print(fruit)
```

Start your Python journey today!
"#,
    )?;

    // Document 4: Configuration Reference
    let config_doc = temp_dir.path().join("config-reference.md");
    fs::write(
        &config_doc,
        r#"# Application Configuration Reference

Complete reference for all configuration options.

## Database Configuration

```toml
[database]
url = "postgres://localhost/mydb"
pool_size = 10
timeout = 30
```

## Server Configuration

```toml
[server]
host = "127.0.0.1"
port = 8080
workers = 4
```

## Logging Configuration

```toml
[logging]
level = "info"
format = "json"
```

See examples/ directory for complete configuration samples.
"#,
    )?;

    println!("✅ Created 4 sample documents\n");

    // Create database
    let db_dir = TempDir::new()?;
    let db_path = db_dir.path().join("demo.sqlite");
    let db = Database::open(&db_path)?;
    db.initialize()?;

    // Add collection
    db.add_collection(
        "demo",
        temp_dir.path().to_str().unwrap(),
        "*.md",
        "file",
        Some(r#"{"exclude_hidden":"false"}"#),
    )?;

    println!("🔧 Indexing documents with metadata generation...\n");

    // Try to load LLM model
    let generator_result = HttpMetadataGenerator::from_env();
    let generator = generator_result.ok();

    if generator.is_some() {
        println!("✨ Using LLM model for high-quality metadata\n");
    } else {
        println!("📝 Using fallback heuristics (LLM model not available)\n");
    }

    // Index with metadata
    if let Some(ref gen) = generator {
        db.reindex_collection_with_metadata("demo", Some(gen as &dyn MetadataGenerator))
            .await?;
    } else {
        db.reindex_collection_with_metadata("demo", None).await?;
    }

    println!("✅ Indexing complete!\n");
    println!("═══════════════════════════════════════════════════════════════\n");

    // Show metadata for each document
    let search_opts = SearchOptions {
        limit: 10,
        min_score: 0.0,
        collection: Some("demo".to_string()),
        provider: None,

        metadata_filters: Vec::new(),
        ..Default::default()
    };

    let results = db.search_fts("programming OR configuration", &search_opts)?;

    for (i, result) in results.iter().enumerate() {
        println!("\n📄 Document {}: {}", i + 1, result.title);
        println!("   File: {}", result.filepath);
        println!("   Doc ID: #{}", result.docid);
        println!();

        if let Some(summary) = &result.llm_summary {
            println!("   📋 Summary:");
            // Wrap text at 70 characters
            for line in summary.split('\n') {
                if line.trim().is_empty() {
                    continue;
                }
                let words: Vec<&str> = line.split_whitespace().collect();
                let mut current_line = String::from("      ");
                for word in words {
                    if current_line.len() + word.len() + 1 > 76 {
                        println!("{}", current_line);
                        current_line = String::from("      ");
                    }
                    if !current_line.ends_with(' ') && current_line.len() > 6 {
                        current_line.push(' ');
                    }
                    current_line.push_str(word);
                }
                if current_line.len() > 6 {
                    println!("{}", current_line);
                }
            }
            println!();
        }

        if let Some(title) = &result.llm_title {
            println!("   🏷️  Semantic Title: {}", title);
        }

        if let Some(category) = &result.llm_category {
            println!("   📂 Category: {}", category);
        }

        if let Some(difficulty) = &result.llm_difficulty {
            println!("   📊 Difficulty: {}", difficulty);
        }

        if let Some(keywords) = &result.llm_keywords {
            if !keywords.is_empty() {
                println!("   🔑 Keywords: {}", keywords.join(", "));
            }
        }

        println!("   ─────────────────────────────────────────────────────");
    }

    println!("\n═══════════════════════════════════════════════════════════════\n");

    // Demonstrate filtering by metadata
    println!("🔍 Demo: Filtering by Difficulty Level\n");

    // Search for beginner content
    let all_results = db.search_fts("programming", &search_opts)?;
    let beginner_docs: Vec<_> = all_results
        .iter()
        .filter(|r| {
            r.llm_difficulty
                .as_ref()
                .map_or(false, |d| d.to_lowercase().contains("beginner"))
        })
        .collect();

    println!("   Query: 'programming' + difficulty=beginner");
    println!("   Results: {} documents", beginner_docs.len());
    for doc in beginner_docs {
        println!(
            "      • {} ({})",
            doc.title,
            doc.llm_difficulty.as_ref().unwrap()
        );
    }
    println!();

    // Search for advanced content
    let advanced_docs: Vec<_> = all_results
        .iter()
        .filter(|r| {
            r.llm_difficulty
                .as_ref()
                .map_or(false, |d| d.to_lowercase().contains("advanced"))
        })
        .collect();

    println!("   Query: 'programming' + difficulty=advanced");
    println!("   Results: {} documents", advanced_docs.len());
    for doc in advanced_docs {
        println!(
            "      • {} ({})",
            doc.title,
            doc.llm_difficulty.as_ref().unwrap()
        );
    }

    println!("\n═══════════════════════════════════════════════════════════════\n");

    // Demonstrate category filtering
    println!("🔍 Demo: Filtering by Category\n");

    let config_docs: Vec<_> = all_results
        .iter()
        .filter(|r| {
            r.llm_category
                .as_ref()
                .map_or(false, |c| c.to_lowercase().contains("config"))
        })
        .collect();

    println!("   Query: category contains 'config'");
    println!("   Results: {} documents", config_docs.len());
    for doc in config_docs {
        println!(
            "      • {} ({})",
            doc.title,
            doc.llm_category.as_ref().unwrap()
        );
    }

    println!("\n═══════════════════════════════════════════════════════════════\n");
    println!("✨ Demo Complete!\n");
    println!("Key Takeaways:");
    println!("   • Metadata is generated automatically during indexing");
    println!("   • Works with or without LLM (fallback heuristics)");
    println!("   • Searchable via full-text search");
    println!("   • Filterable by category, difficulty, concepts");
    println!("   • Cached by content hash for efficiency");
    println!();

    Ok(())
}
