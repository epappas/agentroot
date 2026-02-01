// Custom indexing pipeline example

use agentroot_core::{Database, SemanticChunker};
use chrono::Utc;
use walkdir::WalkDir;

fn main() -> agentroot_core::Result<()> {
    println!("Agentroot Custom Indexing Example\n");

    // Open database
    let db_path = std::env::temp_dir().join("agentroot_custom_index.db");
    println!("Opening database at: {}", db_path.display());
    let db = Database::open(&db_path)?;
    db.initialize()?;

    // Create collection
    println!("Creating collection...");
    db.add_collection("custom", ".", "**/*.rs", "file", None)?;

    // Create semantic chunker for code files
    let chunker = SemanticChunker::new();

    // Scan current directory for Rust files
    println!("Scanning for .rs files in current directory...");
    let mut file_count = 0;

    for entry in WalkDir::new(".")
        .follow_links(false)
        .max_depth(2)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();

        // Filter for .rs files
        if !path.is_file() || path.extension().and_then(|s| s.to_str()) != Some("rs") {
            continue;
        }

        // Skip target directory
        if path.to_string_lossy().contains("target/") {
            continue;
        }

        // Read file content
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Failed to read {}: {}", path.display(), e);
                continue;
            }
        };

        // Compute content hash
        let hash = agentroot_core::db::hash_content(&content);

        // Insert content (deduplicated by hash)
        db.insert_content(&hash, &content)?;

        // Extract title from path
        let title = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("Unknown");

        // Get relative path
        let relative_path = path
            .strip_prefix(".")
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();

        // Insert document with timestamps
        let now = Utc::now().to_rfc3339();
        db.insert_document(
            "custom",
            &relative_path,
            title,
            &hash,
            &now,
            &now,
            "file",
            None,
        )?;

        // Chunk the content
        let chunks = chunker.chunk(&content, path)?;

        println!("  Indexed: {} ({} chunks)", relative_path, chunks.len());
        file_count += 1;

        // Demonstrate chunk inspection
        if file_count == 1 {
            println!("\n  First file chunks:");
            for (i, chunk) in chunks.iter().take(3).enumerate() {
                println!(
                    "    {}. {:?} at line {}",
                    i + 1,
                    chunk.chunk_type,
                    chunk.metadata.start_line
                );
            }
            println!();
        }
    }

    println!("\nIndexed {} Rust files", file_count);

    // Query statistics
    println!("\nDatabase statistics:");
    let collections = db.list_collections()?;
    for coll in collections {
        println!(
            "  Collection '{}': {} documents",
            coll.name, coll.document_count
        );
    }

    // Demonstrate search
    if file_count > 0 {
        println!("\nPerforming test search...");
        let options = agentroot_core::SearchOptions {
            limit: 5,
            min_score: 0.0,
            collection: Some("custom".to_string()),
            provider: None,
            full_content: false,
            metadata_filters: Vec::new(),
            ..Default::default()
        };

        let results = db.search_fts("fn main", &options)?;
        println!("Found {} results for 'fn main':", results.len());

        for result in results.iter().take(3) {
            println!("  - {} (score: {:.2})", result.display_path, result.score);
        }
    }

    // Cleanup
    println!("\nCleaning up...");
    std::fs::remove_file(&db_path).ok();

    println!("Example completed successfully!");

    Ok(())
}
