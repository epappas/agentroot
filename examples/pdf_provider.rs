//! PDF Provider Example
//!
//! Demonstrates how to use the PDFProvider to index PDF documents.
//! This example shows:
//! - Indexing PDF files from a directory
//! - Extracting text content from PDFs
//! - Searching indexed PDF content
//! - Handling both single files and directories

use agentroot_core::{Database, PDFProvider, ProviderConfig};
use chrono::Utc;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== Agentroot PDF Provider Example ===\n");

    // Initialize database
    let db_path = PathBuf::from("example_pdf.db");
    if db_path.exists() {
        std::fs::remove_file(&db_path)?;
    }

    let db = Database::open(&db_path)?;
    db.initialize()?;

    println!("✓ Database initialized\n");

    // Example 1: Index a single PDF file
    println!("Example 1: Indexing a single PDF file");
    println!("----------------------------------------");

    let provider = PDFProvider::new();

    // Note: Replace with an actual PDF file path on your system
    let pdf_path = "example.pdf";

    if PathBuf::from(pdf_path).exists() {
        match provider.fetch_item(pdf_path).await {
            Ok(item) => {
                println!("  Title: {}", item.title);
                println!("  URI: {}", item.uri);
                println!("  Content length: {} characters", item.content.len());
                println!("  Hash: {}", item.hash);

                // Insert into database
                let now = Utc::now().to_rfc3339();
                db.insert_document(
                    "pdfs",
                    &item.uri,
                    &item.title,
                    &item.hash,
                    &now,
                    &now,
                    "pdf",
                    Some(&item.uri),
                )?;

                // Store content
                db.store_content(&item.hash, &item.content)?;

                println!("  ✓ Indexed successfully\n");
            }
            Err(e) => {
                println!("  ✗ Error: {}\n", e);
                println!("  Note: Please provide a valid PDF file path\n");
            }
        }
    } else {
        println!("  Skipping: example.pdf not found");
        println!("  To use this example, create a PDF file named 'example.pdf'\n");
    }

    // Example 2: Index all PDFs in a directory
    println!("Example 2: Indexing PDF files from a directory");
    println!("-----------------------------------------------");

    let pdf_dir = "./pdfs";
    if PathBuf::from(pdf_dir).exists() {
        let config = ProviderConfig::new(pdf_dir.to_string(), "**/*.pdf".to_string());

        match provider.list_items(&config).await {
            Ok(items) => {
                println!("  Found {} PDF files", items.len());

                for (i, item) in items.iter().enumerate() {
                    println!("\n  PDF #{}", i + 1);
                    println!("    Title: {}", item.title);
                    println!("    File: {}", item.uri);
                    println!("    Size: {} bytes", item.content.len());

                    // Insert into database
                    let now = Utc::now().to_rfc3339();
                    db.insert_document(
                        "pdfs",
                        &item.uri,
                        &item.title,
                        &item.hash,
                        &now,
                        &now,
                        "pdf",
                        Some(&item.uri),
                    )?;

                    db.store_content(&item.hash, &item.content)?;
                }

                println!("\n  ✓ Indexed {} PDFs successfully\n", items.len());
            }
            Err(e) => {
                println!("  ✗ Error: {}\n", e);
                println!(
                    "  Note: Create a './pdfs' directory with PDF files to use this example\n"
                );
            }
        }
    } else {
        println!("  Skipping: ./pdfs directory not found");
        println!("  To use this example, create a './pdfs' directory with PDF files\n");
    }

    // Example 3: Search indexed PDFs
    println!("Example 3: Searching indexed PDF content");
    println!("------------------------------------------");

    // Add collection to database
    db.add_collection("pdfs", pdf_dir, "**/*.pdf", "pdf", None)?;

    // Update FTS index
    let docs = db.get_all_documents("pdfs")?;
    if !docs.is_empty() {
        println!("  Indexing {} documents for search...", docs.len());

        for doc in docs {
            if let Some(content) = db.get_content(&doc.hash)? {
                db.index_document(&doc.filepath, &doc.title, &content)?;
            }
        }

        println!("  ✓ Search index updated\n");

        // Perform a search
        let search_options = agentroot_core::SearchOptions {
            limit: 5,
            min_score: 0.0,
            collection: Some("pdfs".to_string()),
            provider: Some("pdf".to_string()),
            full_content: false,
        };

        println!("  Searching for 'document'...");
        match db.search_fts("document", &search_options) {
            Ok(results) => {
                println!("  Found {} results:\n", results.len());

                for (i, result) in results.iter().enumerate() {
                    println!("    Result #{}", i + 1);
                    println!("      Title: {}", result.title);
                    println!("      File: {}", result.display_path);
                    println!("      Score: {:.2}", result.score);

                    if let Some(context) = &result.context {
                        let preview = context.chars().take(100).collect::<String>();
                        println!("      Preview: {}...", preview);
                    }
                    println!();
                }
            }
            Err(e) => {
                println!("  ✗ Search error: {}", e);
            }
        }
    } else {
        println!("  No documents indexed. Skipping search example.\n");
    }

    // Example 4: Using PDFProvider with the Database API
    println!("Example 4: Using PDFProvider with Database.reindex_collection()");
    println!("------------------------------------------------------------------");

    println!("  This method automatically uses the provider system:");
    println!("  1. Database detects 'pdf' provider type");
    println!("  2. Loads PDFProvider from registry");
    println!("  3. Scans directory for PDF files");
    println!("  4. Extracts text and indexes content");
    println!();
    println!("  Usage:");
    println!(
        "    db.add_collection(\"my-pdfs\", \"/path/to/pdfs\", \"**/*.pdf\", \"pdf\", None)?;"
    );
    println!("    db.reindex_collection(\"my-pdfs\").await?;");
    println!();

    // Cleanup
    println!("✓ Example completed");
    println!("\nNote: The database file 'example_pdf.db' has been created.");
    println!("      You can delete it when done: rm example_pdf.db");

    Ok(())
}
