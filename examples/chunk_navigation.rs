//! Chunk storage and navigation demo
//!
//! Demonstrates insert_chunk, get_chunks_for_document, search_chunks_fts,
//! search_chunks_by_label, and get_surrounding_chunks.
//!
//! Usage:
//!   cargo run --example chunk_navigation -p agentroot-core

use agentroot_core::db::{hash_content, Database};
use chrono::Utc;
use std::collections::HashMap;

fn main() -> agentroot_core::Result<()> {
    println!("=== Chunk Navigation Demo ===\n");

    let db = Database::open_in_memory()?;
    db.initialize()?;
    db.add_collection("source", ".", "**/*.rs", "file", None)?;

    let now = Utc::now().to_rfc3339();

    // Insert a Rust source file as a document
    let source_code = r#"use std::collections::HashMap;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub workers: usize,
}

impl Config {
    pub fn new(host: &str, port: u16) -> Self {
        Self { host: host.to_string(), port, workers: 4 }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.port == 0 { return Err("port must be > 0".into()); }
        if self.workers == 0 { return Err("workers must be > 0".into()); }
        Ok(())
    }

    pub fn to_url(&self) -> String {
        format!("http://{}:{}", self.host, self.port)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_validate() {
        let cfg = Config::new("localhost", 8080);
        assert!(cfg.validate().is_ok());
    }
}"#;

    let doc_hash = hash_content(source_code);
    db.insert_content(&doc_hash, source_code)?;
    db.insert_document("source", "src/config.rs", "Config Module", &doc_hash, &now, &now, "file", None)?;
    println!("Inserted document: src/config.rs (hash: {}...)\n", &doc_hash[..12]);

    // Insert semantic chunks
    struct ChunkDef {
        content: &'static str,
        chunk_type: &'static str,
        breadcrumb: &'static str,
        start_line: i32,
        end_line: i32,
        labels: Vec<(&'static str, &'static str)>,
        concepts: Vec<&'static str>,
    }

    let chunks = vec![
        ChunkDef {
            content: "use std::collections::HashMap;\nuse serde::{Serialize, Deserialize};",
            chunk_type: "Imports", breadcrumb: "imports",
            start_line: 1, end_line: 2,
            labels: vec![("layer", "dependency"), ("scope", "module")],
            concepts: vec!["serde", "hashmap"],
        },
        ChunkDef {
            content: "#[derive(Debug, Clone, Serialize, Deserialize)]\npub struct Config {\n    pub host: String,\n    pub port: u16,\n    pub workers: usize,\n}",
            chunk_type: "Struct", breadcrumb: "Config",
            start_line: 4, end_line: 9,
            labels: vec![("layer", "model"), ("kind", "data-structure")],
            concepts: vec!["config", "serialization"],
        },
        ChunkDef {
            content: "pub fn new(host: &str, port: u16) -> Self {\n    Self { host: host.to_string(), port, workers: 4 }\n}",
            chunk_type: "Function", breadcrumb: "Config::new",
            start_line: 12, end_line: 14,
            labels: vec![("layer", "constructor"), ("operation", "creation")],
            concepts: vec!["constructor", "initialization"],
        },
        ChunkDef {
            content: "pub fn validate(&self) -> Result<(), String> {\n    if self.port == 0 { return Err(\"port must be > 0\".into()); }\n    if self.workers == 0 { return Err(\"workers must be > 0\".into()); }\n    Ok(())\n}",
            chunk_type: "Function", breadcrumb: "Config::validate",
            start_line: 16, end_line: 20,
            labels: vec![("layer", "service"), ("operation", "validation")],
            concepts: vec!["validation", "error-handling"],
        },
        ChunkDef {
            content: "pub fn to_url(&self) -> String {\n    format!(\"http://{}:{}\", self.host, self.port)\n}",
            chunk_type: "Function", breadcrumb: "Config::to_url",
            start_line: 22, end_line: 24,
            labels: vec![("layer", "service"), ("operation", "formatting")],
            concepts: vec!["url", "string-formatting"],
        },
        ChunkDef {
            content: "#[cfg(test)]\nmod tests {\n    use super::*;\n    #[test]\n    fn test_validate() {\n        let cfg = Config::new(\"localhost\", 8080);\n        assert!(cfg.validate().is_ok());\n    }\n}",
            chunk_type: "Test", breadcrumb: "tests::test_validate",
            start_line: 27, end_line: 35,
            labels: vec![("layer", "test"), ("scope", "unit")],
            concepts: vec!["testing", "assertions"],
        },
    ];

    for (seq, chunk) in chunks.iter().enumerate() {
        let chunk_hash = hash_content(chunk.content);
        let labels: HashMap<String, String> = chunk.labels.iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        let concepts: Vec<String> = chunk.concepts.iter().map(|s| s.to_string()).collect();

        db.insert_chunk(
            &chunk_hash, &doc_hash, seq as i32, 0,
            chunk.content, Some(chunk.chunk_type), Some(chunk.breadcrumb),
            chunk.start_line, chunk.end_line, Some("rust"),
            Some(&format!("{} chunk for {}", chunk.chunk_type, chunk.breadcrumb)),
            Some(&format!("Defines {}", chunk.breadcrumb)),
            &concepts, &labels, &[],
            None, None, &now,
        )?;
    }
    println!("Inserted {} chunks\n", chunks.len());

    // 1. get_chunks_for_document
    println!("--- All chunks for document ---");
    let all = db.get_chunks_for_document(&doc_hash)?;
    for c in &all {
        println!("  seq={} type={:<10} breadcrumb={:<25} lines {}-{}",
            c.seq,
            c.chunk_type.as_deref().unwrap_or("?"),
            c.breadcrumb.as_deref().unwrap_or("?"),
            c.start_line, c.end_line,
        );
    }

    // 2. search_chunks_fts
    println!("\n--- FTS search: 'validate port workers' ---");
    let fts_results = db.search_chunks_fts("validate port workers", 5)?;
    for c in &fts_results {
        println!("  [{}] {} (lines {}-{})",
            c.chunk_type.as_deref().unwrap_or("?"),
            c.breadcrumb.as_deref().unwrap_or("?"),
            c.start_line, c.end_line,
        );
    }

    // 3. search_chunks_by_label
    println!("\n--- Label search: layer=service ---");
    let label_results = db.search_chunks_by_label("layer", "service")?;
    for c in &label_results {
        println!("  [{}] {} -- {}",
            c.chunk_type.as_deref().unwrap_or("?"),
            c.breadcrumb.as_deref().unwrap_or("?"),
            c.llm_labels.iter().map(|(k, v)| format!("{}:{}", k, v)).collect::<Vec<_>>().join(", "),
        );
    }

    println!("\n--- Label search: operation=validation ---");
    let val_results = db.search_chunks_by_label("operation", "validation")?;
    for c in &val_results {
        println!("  [{}] {}", c.chunk_type.as_deref().unwrap_or("?"), c.breadcrumb.as_deref().unwrap_or("?"));
    }

    // 4. get_surrounding_chunks
    println!("\n--- Surrounding chunks for 'Config::validate' ---");
    if let Some(target) = fts_results.first() {
        let (prev, next) = db.get_surrounding_chunks(&target.hash)?;
        if let Some(p) = &prev {
            println!("  PREV: [{}] {} (lines {}-{})", p.chunk_type.as_deref().unwrap_or("?"),
                p.breadcrumb.as_deref().unwrap_or("?"), p.start_line, p.end_line);
        }
        println!("  THIS: [{}] {} (lines {}-{})",
            target.chunk_type.as_deref().unwrap_or("?"),
            target.breadcrumb.as_deref().unwrap_or("?"),
            target.start_line, target.end_line);
        if let Some(n) = &next {
            println!("  NEXT: [{}] {} (lines {}-{})", n.chunk_type.as_deref().unwrap_or("?"),
                n.breadcrumb.as_deref().unwrap_or("?"), n.start_line, n.end_line);
        }
    }

    println!("\nDone.");
    Ok(())
}
