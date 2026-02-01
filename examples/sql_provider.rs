//! SQL Provider Example
//!
//! Demonstrates how to use the SQLProvider to index database content.
//! This example shows:
//! - Creating a sample SQLite database
//! - Indexing table data with SQLProvider
//! - Custom SQL queries for selective indexing
//! - Searching indexed database content

use agentroot_core::{Database, ProviderConfig, SQLProvider};
use chrono::Utc;
use rusqlite::{params, Connection};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== Agentroot SQL Provider Example ===\n");

    // Create a sample SQLite database with content to index
    println!("Step 1: Creating sample database");
    println!("----------------------------------");

    let sample_db_path = PathBuf::from("sample_data.db");
    if sample_db_path.exists() {
        std::fs::remove_file(&sample_db_path)?;
    }

    let sample_conn = Connection::open(&sample_db_path)?;

    // Create a sample articles table
    sample_conn.execute(
        "CREATE TABLE articles (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL,
            content TEXT NOT NULL,
            author TEXT,
            category TEXT,
            published_date TEXT
        )",
        [],
    )?;

    // Insert sample data
    let articles = vec![
        (
            "Introduction to Rust",
            "Rust is a systems programming language that runs blazingly fast, prevents segfaults, and guarantees thread safety.",
            "Alice Smith",
            "Programming",
            "2024-01-15",
        ),
        (
            "Understanding Async/Await",
            "Asynchronous programming in Rust allows you to write non-blocking code using async/await syntax.",
            "Bob Johnson",
            "Programming",
            "2024-01-20",
        ),
        (
            "Database Best Practices",
            "Learn the essential best practices for database design, indexing, and query optimization.",
            "Carol White",
            "Database",
            "2024-01-25",
        ),
        (
            "Web Development with Rust",
            "Build fast and reliable web applications using Rust frameworks like Actix-web and Rocket.",
            "David Brown",
            "Web Development",
            "2024-02-01",
        ),
        (
            "Error Handling Patterns",
            "Master error handling in Rust using Result, Option, and the question mark operator.",
            "Eve Davis",
            "Programming",
            "2024-02-05",
        ),
    ];

    for (title, content, author, category, date) in articles {
        sample_conn.execute(
            "INSERT INTO articles (title, content, author, category, published_date) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![title, content, author, category, date],
        )?;
    }

    println!("  ✓ Created sample database with {} articles\n", 5);

    // Initialize agentroot database
    let index_db_path = PathBuf::from("example_sql.db");
    if index_db_path.exists() {
        std::fs::remove_file(&index_db_path)?;
    }

    let db = Database::open(&index_db_path)?;
    db.initialize()?;

    println!("Step 2: Indexing database content");
    println!("-----------------------------------");

    let provider = SQLProvider::new();

    // Example 1: Index entire table
    println!("  Example 1: Indexing entire table");

    let mut config =
        ProviderConfig::new(sample_db_path.to_string_lossy().to_string(), "".to_string());
    config
        .options
        .insert("table".to_string(), "articles".to_string());
    config
        .options
        .insert("id_column".to_string(), "id".to_string());
    config
        .options
        .insert("title_column".to_string(), "title".to_string());
    config
        .options
        .insert("content_column".to_string(), "content".to_string());

    match provider.list_items(&config).await {
        Ok(items) => {
            println!("    Found {} articles", items.len());

            for item in &items {
                let now = Utc::now().to_rfc3339();
                db.insert_document(
                    "articles",
                    &item.uri,
                    &item.title,
                    &item.hash,
                    &now,
                    &now,
                    "sql",
                    Some(&item.uri),
                )?;
                db.store_content(&item.hash, &item.content)?;
            }

            println!("    ✓ Indexed {} articles\n", items.len());
        }
        Err(e) => {
            println!("    ✗ Error: {}\n", e);
        }
    }

    // Example 2: Index with custom query (filter by category)
    println!("  Example 2: Indexing with custom query");

    let mut config_filtered =
        ProviderConfig::new(sample_db_path.to_string_lossy().to_string(), "".to_string());
    config_filtered.options.insert(
        "query".to_string(),
        "SELECT id, title, content FROM articles WHERE category = 'Programming'".to_string(),
    );
    config_filtered
        .options
        .insert("id_column".to_string(), "id".to_string());
    config_filtered
        .options
        .insert("title_column".to_string(), "title".to_string());
    config_filtered
        .options
        .insert("content_column".to_string(), "content".to_string());

    match provider.list_items(&config_filtered).await {
        Ok(items) => {
            println!("    Found {} programming articles", items.len());

            for item in &items {
                println!("      - {}", item.title);
            }
            println!();
        }
        Err(e) => {
            println!("    ✗ Error: {}\n", e);
        }
    }

    // Example 3: Add collection and use database reindex
    println!("Step 3: Using collection-based indexing");
    println!("----------------------------------------");

    db.add_collection(
        "articles",
        &sample_db_path.to_string_lossy(),
        "",
        "sql",
        Some(r#"{"table":"articles","id_column":"id","title_column":"title","content_column":"content"}"#),
    )?;

    let count = db.reindex_collection("articles").await?;
    println!("  ✓ Reindexed {} documents via collection\n", count);

    // Example 4: Search indexed content
    println!("Step 4: Searching indexed database content");
    println!("--------------------------------------------");

    let search_options = agentroot_core::SearchOptions {
        limit: 10,
        min_score: 0.0,
        collection: Some("articles".to_string()),
        provider: Some("sql".to_string()),

        metadata_filters: Vec::new(),
        ..Default::default()
    };

    let queries = vec!["async programming", "database", "error handling"];

    for query in queries {
        println!("  Query: \"{}\"", query);
        match db.search_fts(query, &search_options) {
            Ok(results) => {
                if results.is_empty() {
                    println!("    No results found\n");
                } else {
                    for result in results {
                        println!("    - {} (score: {:.2})", result.title, result.score);
                    }
                    println!();
                }
            }
            Err(e) => {
                println!("    ✗ Error: {}\n", e);
            }
        }
    }

    // Example 5: Retrieve specific document
    println!("Step 5: Retrieving specific documents");
    println!("---------------------------------------");

    let uri = format!("sql://{}/1", sample_db_path.to_string_lossy());
    println!("  Fetching document: {}", uri);

    match provider.fetch_item(&uri).await {
        Ok(item) => {
            println!("    Title: {}", item.title);
            println!(
                "    Content: {}",
                item.content.chars().take(100).collect::<String>()
            );
            println!("    ...\n");
        }
        Err(e) => {
            println!("    ✗ Error: {}\n", e);
        }
    }

    // Example 6: Advanced configuration
    println!("Step 6: Advanced configuration examples");
    println!("----------------------------------------");

    println!("  Configuration options:");
    println!();
    println!("  1. Basic table indexing:");
    println!("     {{");
    println!("       \"table\": \"my_table\",");
    println!("       \"id_column\": \"id\",");
    println!("       \"title_column\": \"title\",");
    println!("       \"content_column\": \"body\"");
    println!("     }}");
    println!();
    println!("  2. Custom SQL query:");
    println!("     {{");
    println!("       \"query\": \"SELECT id, name as title, description as content FROM products WHERE active = 1\"");
    println!("     }}");
    println!();
    println!("  3. With joins:");
    println!("     {{");
    println!("       \"query\": \"SELECT p.id, p.title, p.content || ' ' || c.name as content FROM posts p JOIN categories c ON p.category_id = c.id\"");
    println!("     }}");
    println!();

    // Cleanup information
    println!("✓ Example completed");
    println!("\nCreated files:");
    println!("  - sample_data.db (sample SQLite database)");
    println!("  - example_sql.db (agentroot index)");
    println!("\nCleanup:");
    println!("  rm sample_data.db example_sql.db");
    println!();
    println!("Use cases:");
    println!("  - Index knowledge base from CMS databases");
    println!("  - Search across product catalogs");
    println!("  - Query documentation stored in databases");
    println!("  - Index user-generated content");

    Ok(())
}
