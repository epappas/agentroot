//! CSV provider example
//!
//! Demonstrates how to use the CSVProvider to index CSV files with row-based
//! semantic chunking.

use agentroot_core::{CSVProvider, Database, ProviderConfig, SourceProvider};
use chrono::Utc;
use std::fs;

#[tokio::main]
async fn main() -> agentroot_core::Result<()> {
    println!("Agentroot CSV Provider Example\n");

    // Create temporary directory and sample CSV files
    let temp_dir = std::env::temp_dir().join("agentroot_csv_example");
    fs::create_dir_all(&temp_dir)?;
    println!("Working directory: {}\n", temp_dir.display());

    // Create sample customers CSV
    let customers_csv = temp_dir.join("customers.csv");
    fs::write(
        &customers_csv,
        "customer_id,name,email,city,signup_date\n\
         1,Alice Johnson,alice@example.com,New York,2024-01-15\n\
         2,Bob Smith,bob@example.com,Los Angeles,2024-02-20\n\
         3,Carol White,carol@example.com,Chicago,2024-03-10\n\
         4,David Brown,david@example.com,Houston,2024-04-05\n\
         5,Eve Davis,eve@example.com,Phoenix,2024-05-12",
    )?;
    println!("Created sample file: customers.csv");

    // Create sample sales CSV with semicolon delimiter
    let sales_csv = temp_dir.join("sales.csv");
    fs::write(
        &sales_csv,
        "order_id;customer_id;product;amount;date\n\
         101;1;Laptop;1299.99;2024-01-20\n\
         102;2;Mouse;29.99;2024-02-25\n\
         103;1;Keyboard;89.99;2024-03-15\n\
         104;3;Monitor;399.99;2024-04-10\n\
         105;4;Headphones;199.99;2024-05-18",
    )?;
    println!("Created sample file: sales.csv (with semicolon delimiter)\n");

    // Setup database
    let db_path = temp_dir.join("csv_example.db");
    println!("Opening database at: {}", db_path.display());
    let db = Database::open(&db_path)?;
    db.initialize()?;

    println!("\n=== Example 1: Index Customers CSV (comma-delimited) ===\n");

    let provider = CSVProvider::new();
    let config = ProviderConfig::new(
        customers_csv.to_string_lossy().to_string(),
        "**/*.csv".to_string(),
    );

    println!("Parsing customers.csv...");
    let items = provider.list_items(&config).await?;
    println!("Found {} rows", items.len());

    // Create collection
    db.add_collection(
        "customers",
        customers_csv.to_string_lossy().as_ref(),
        "**/*.csv",
        "csv",
        None,
    )?;

    // Index all rows
    let now = Utc::now().to_rfc3339();
    for (idx, item) in items.iter().enumerate() {
        println!("\nRow {}:", idx + 1);
        println!("  URI: {}", item.uri);
        println!("  Title: {}", item.title);
        println!("  Hash: {}", item.hash);
        println!("  Metadata:");
        for (key, value) in &item.metadata {
            println!("    {}: {}", key, value);
        }
        println!("  Content Preview:");
        for line in item.content.lines().take(3) {
            println!("    {}", line);
        }

        db.insert_content(&item.hash, &item.content)?;
        db.insert_document(
            "customers",
            &item.uri,
            &item.title,
            &item.hash,
            &now,
            &now,
            &item.source_type,
            Some(&item.uri),
        )?;
    }

    println!("\n=== Example 2: Index Sales CSV (semicolon-delimited) ===\n");

    let mut sales_config = ProviderConfig::new(
        sales_csv.to_string_lossy().to_string(),
        "**/*.csv".to_string(),
    );
    sales_config
        .options
        .insert("delimiter".to_string(), ";".to_string());

    println!("Parsing sales.csv with semicolon delimiter...");
    let sales_items = provider.list_items(&sales_config).await?;
    println!("Found {} rows", sales_items.len());

    db.add_collection(
        "sales",
        sales_csv.to_string_lossy().as_ref(),
        "**/*.csv",
        "csv",
        Some(r#"{"delimiter":";"}"#),
    )?;

    for (idx, item) in sales_items.iter().enumerate() {
        println!("\nRow {}:", idx + 1);
        println!(
            "  Product: {}",
            item.metadata.get("product").unwrap_or(&"N/A".to_string())
        );
        println!(
            "  Amount: {}",
            item.metadata.get("amount").unwrap_or(&"N/A".to_string())
        );
        println!(
            "  Date: {}",
            item.metadata.get("date").unwrap_or(&"N/A".to_string())
        );

        db.insert_content(&item.hash, &item.content)?;
        db.insert_document(
            "sales",
            &item.uri,
            &item.title,
            &item.hash,
            &now,
            &now,
            &item.source_type,
            Some(&item.uri),
        )?;
    }

    println!("\n=== Example 3: Fetch Specific Row by URI ===\n");

    let first_customer_uri = format!("csv://{}/row_1", customers_csv.display());
    println!("Fetching: {}", first_customer_uri);

    match provider.fetch_item(&first_customer_uri).await {
        Ok(item) => {
            println!("Successfully fetched:");
            println!(
                "  Name: {}",
                item.metadata.get("name").unwrap_or(&"N/A".to_string())
            );
            println!(
                "  Email: {}",
                item.metadata.get("email").unwrap_or(&"N/A".to_string())
            );
            println!(
                "  City: {}",
                item.metadata.get("city").unwrap_or(&"N/A".to_string())
            );
        }
        Err(e) => {
            eprintln!("Error fetching row: {}", e);
        }
    }

    println!("\n=== Example 4: Search Indexed Data ===\n");

    // Reindex to update FTS
    db.reindex_collection("customers").await?;
    db.reindex_collection("sales").await?;

    // Search for customers
    println!("Searching for 'Alice'...");
    let options = agentroot_core::SearchOptions {
        limit: 10,
        min_score: 0.0,
        collection: Some("customers".to_string()),
        provider: None,
        full_content: true,
        metadata_filters: Vec::new(),
        ..Default::default()
    };
    let results = db.search_fts("Alice", &options)?;

    println!("Found {} results", results.len());
    for result in results.iter().take(3) {
        println!("  - {} (score: {:.2})", result.title, result.score);
        if let Some(ref body) = result.body {
            for line in body.lines().take(2) {
                println!("    {}", line);
            }
        }
    }

    // Search for products
    println!("\nSearching for 'Laptop'...");
    let options = agentroot_core::SearchOptions {
        limit: 10,
        min_score: 0.0,
        collection: Some("sales".to_string()),
        provider: None,
        full_content: true,
        metadata_filters: Vec::new(),
        ..Default::default()
    };
    let results = db.search_fts("Laptop", &options)?;

    println!("Found {} results", results.len());
    for result in results {
        println!("  - {} (score: {:.2})", result.title, result.score);
    }

    println!("\n=== Summary ===");
    println!("✓ Indexed {} customer records", items.len());
    println!("✓ Indexed {} sales records", sales_items.len());
    println!("✓ All records are searchable with full-text search");
    println!("✓ Each row has rich metadata extracted from columns");
    println!("\nDatabase saved at: {}", db_path.display());

    Ok(())
}
