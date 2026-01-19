//! GitHub provider example
//!
//! Demonstrates how to use the GitHubProvider to index content from GitHub
//! repositories and files.

use agentroot_core::{Database, GitHubProvider, ProviderConfig, SourceProvider};
use chrono::Utc;

fn main() -> agentroot_core::Result<()> {
    println!("Agentroot GitHub Provider Example\n");

    let db_path = std::env::temp_dir().join("agentroot_github_example.db");
    println!("Opening database at: {}", db_path.display());
    let db = Database::open(&db_path)?;
    db.initialize()?;

    println!("Creating GitHub collection...");
    db.add_collection(
        "rust-lang",
        "https://github.com/rust-lang/rust",
        "**/*.md",
        "github",
        None,
    )?;

    println!("\nFetching README from rust-lang/rust repository...");
    let provider = GitHubProvider::new();

    let repo_url = "https://github.com/rust-lang/rust";
    match provider.fetch_item(repo_url) {
        Ok(item) => {
            println!("Successfully fetched:");
            println!("  URI: {}", item.uri);
            println!("  Title: {}", item.title);
            println!("  Source Type: {}", item.source_type);
            println!("  Content Length: {} bytes", item.content.len());
            println!("  Hash: {}", item.hash);

            if let Some(owner) = item.metadata.get("owner") {
                println!("  Owner: {}", owner);
            }
            if let Some(repo) = item.metadata.get("repo") {
                println!("  Repository: {}", repo);
            }

            let now = Utc::now().to_rfc3339();
            db.insert_content(&item.hash, &item.content)?;
            db.insert_document(
                "rust-lang",
                &item.uri,
                &item.title,
                &item.hash,
                &now,
                &now,
                &item.source_type,
                item.metadata.get("source_uri").map(|s| s.as_str()),
            )?;

            println!("\nDocument indexed successfully!");
        }
        Err(e) => {
            eprintln!("Error fetching from GitHub: {}", e);
            println!("\nNote: This example requires internet connection.");
            println!("You can set GITHUB_TOKEN environment variable for higher rate limits.");
        }
    }

    println!("\nFetching specific file from repository...");
    let file_url = "https://github.com/rust-lang/rust/blob/master/CONTRIBUTING.md";
    match provider.fetch_item(file_url) {
        Ok(item) => {
            println!("Successfully fetched file:");
            println!("  URI: {}", item.uri);
            println!("  Title: {}", item.title);
            println!("  Content Length: {} bytes", item.content.len());

            if let Some(path) = item.metadata.get("path") {
                println!("  File Path: {}", path);
            }
        }
        Err(e) => {
            eprintln!("Error fetching file: {}", e);
        }
    }

    println!("\nListing markdown files from repository...");
    let config = ProviderConfig::new(
        "https://github.com/rust-lang/rust".to_string(),
        "**/*.md".to_string(),
    );

    match provider.list_items(&config) {
        Ok(items) => {
            println!("Found {} markdown files", items.len());
            for (i, item) in items.iter().take(5).enumerate() {
                println!("  {}. {} - {} bytes", i + 1, item.uri, item.content.len());
            }
            if items.len() > 5 {
                println!("  ... and {} more files", items.len() - 5);
            }
        }
        Err(e) => {
            eprintln!("Error listing files: {}", e);
            println!("Note: This operation may hit GitHub API rate limits.");
        }
    }

    println!("\nExample completed!");
    println!("\nProvider Architecture Benefits:");
    println!("  - Unified interface for multiple content sources");
    println!("  - Easy to add new providers (URLs, PDFs, databases, etc.)");
    println!("  - Maintains backward compatibility with file-based collections");
    println!("  - Extensible metadata storage per provider type");

    Ok(())
}
