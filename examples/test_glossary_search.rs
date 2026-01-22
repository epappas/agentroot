//! Quick test of glossary search functionality

use agentroot_core::db::Database;

fn main() {
    let db = Database::open_in_memory().unwrap();
    db.initialize().unwrap();

    // Insert test concepts
    println!("Inserting concepts...");
    let id1 = db.upsert_concept("semantic search").unwrap();
    let id2 = db.upsert_concept("hybrid search").unwrap();
    let id3 = db.upsert_concept("URLProvider").unwrap();
    let id4 = db.upsert_concept("AI coding agents").unwrap();

    println!("  semantic search: id={}", id1);
    println!("  hybrid search: id={}", id2);
    println!("  URLProvider: id={}", id3);
    println!("  AI coding agents: id={}", id4);

    // Try searching
    println!("\n=== Test 1: Search 'semantic' ===");
    let results = db.search_concepts("semantic", 10).unwrap();
    println!("Found {} results:", results.len());
    for r in &results {
        println!("  - '{}' (normalized: '{}')", r.term, r.normalized);
    }

    println!("\n=== Test 2: Search 'semantic search' ===");
    let results = db.search_concepts("semantic search", 10).unwrap();
    println!("Found {} results:", results.len());
    for r in &results {
        println!("  - '{}' (normalized: '{}')", r.term, r.normalized);
    }

    println!("\n=== Test 3: Search 'provider' ===");
    let results = db.search_concepts("provider", 10).unwrap();
    println!("Found {} results:", results.len());
    for r in &results {
        println!("  - '{}' (normalized: '{}')", r.term, r.normalized);
    }

    println!("\n=== Test 4: Search 'workflow' ===");
    let results = db.search_concepts("workflow", 10).unwrap();
    println!("Found {} results:", results.len());
    for r in &results {
        println!("  - '{}' (normalized: '{}')", r.term, r.normalized);
    }
}
