// Basic search example using Agentroot as a library

use agentroot_core::{Database, SearchOptions};
use chrono::Utc;

fn main() -> agentroot_core::Result<()> {
    println!("Agentroot Basic Search Example\n");

    // Open database in temporary location
    let db_path = std::env::temp_dir().join("agentroot_example.db");
    println!("Opening database at: {}", db_path.display());
    let db = Database::open(&db_path)?;
    db.initialize()?;

    // Create a test collection
    println!("Creating collection 'example'...");
    db.add_collection("example", ".", "**/*.rs")?;

    // Insert sample content
    println!("Inserting sample documents...");

    let sample_code = r#"/// Handles errors in the application.
pub fn handle_error(err: &str) -> Result<(), AppError> {
    eprintln!("Error: {}", err);
    Err(AppError::Generic(err.to_string()))
}

/// Process user input and validate data.
pub fn process_input(input: &str) -> Result<String, AppError> {
    if input.is_empty() {
        return Err(AppError::EmptyInput);
    }
    Ok(input.trim().to_uppercase())
}"#;

    let now = Utc::now().to_rfc3339();
    let hash = agentroot_core::db::hash_content(sample_code);
    db.insert_content(&hash, sample_code)?;
    db.insert_document(
        "example",
        "src/error_handler.rs",
        "Error Handler",
        &hash,
        &now,
        &now,
    )?;

    let sample_main = r#"fn main() {
    println!("Starting application...");
    match process_input("test data") {
        Ok(result) => println!("Processed: {}", result),
        Err(e) => eprintln!("Error: {}", e),
    }
}"#;

    let hash2 = agentroot_core::db::hash_content(sample_main);
    db.insert_content(&hash2, sample_main)?;
    db.insert_document(
        "example",
        "src/main.rs",
        "Main Entry Point",
        &hash2,
        &now,
        &now,
    )?;

    // Perform search
    println!("\nSearching for 'error'...");
    let options = SearchOptions {
        limit: 10,
        min_score: 0.0,
        collection: None,
        full_content: false,
    };

    let results = db.search_fts("error", &options)?;

    if results.is_empty() {
        println!("No results found.");
    } else {
        println!("Found {} results:\n", results.len());
        for (i, result) in results.iter().enumerate() {
            println!(
                "{}. {} (score: {:.2})",
                i + 1,
                result.display_path,
                result.score
            );
            println!("   Title: {}", result.title);
            println!("   Collection: {}", result.collection_name);
            println!();
        }
    }

    // Get document by path
    println!("Retrieving document 'src/error_handler.rs'...");
    if let Some(doc) = db.find_active_document("example", "src/error_handler.rs")? {
        println!("Found: {} (hash: {})", doc.title, doc.hash);

        // Get content
        if let Some(content) = db.get_content(&doc.hash)? {
            let lines: Vec<&str> = content.lines().collect();
            println!("Content: {} lines", lines.len());
        }
    }

    // List collections
    println!("\nListing collections:");
    let collections = db.list_collections()?;
    for coll in collections {
        println!("  - {} ({} documents)", coll.name, coll.document_count);
    }

    // Cleanup
    println!("\nCleaning up...");
    std::fs::remove_file(&db_path).ok();

    println!("\nExample completed successfully!");

    Ok(())
}
