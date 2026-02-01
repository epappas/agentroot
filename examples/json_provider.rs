//! JSON provider example
//!
//! Demonstrates how to use the JSONProvider to index JSON files with semantic
//! object/array chunking.

use agentroot_core::{Database, JSONProvider, ProviderConfig, SourceProvider};
use chrono::Utc;
use std::fs;

#[tokio::main]
async fn main() -> agentroot_core::Result<()> {
    println!("Agentroot JSON Provider Example\n");

    // Create temporary directory and sample JSON files
    let temp_dir = std::env::temp_dir().join("agentroot_json_example");
    fs::create_dir_all(&temp_dir)?;
    println!("Working directory: {}\n", temp_dir.display());

    // Create sample API responses JSON (array mode)
    let api_logs = temp_dir.join("api_logs.json");
    fs::write(
        &api_logs,
        r#"[
  {
    "id": "req_001",
    "method": "GET",
    "endpoint": "/api/users",
    "status": 200,
    "duration_ms": 45,
    "timestamp": "2024-01-15T10:30:00Z"
  },
  {
    "id": "req_002",
    "method": "POST",
    "endpoint": "/api/users",
    "status": 201,
    "duration_ms": 120,
    "timestamp": "2024-01-15T10:31:00Z"
  },
  {
    "id": "req_003",
    "method": "GET",
    "endpoint": "/api/products",
    "status": 200,
    "duration_ms": 67,
    "timestamp": "2024-01-15T10:32:00Z"
  },
  {
    "id": "req_004",
    "method": "DELETE",
    "endpoint": "/api/users/123",
    "status": 404,
    "duration_ms": 23,
    "timestamp": "2024-01-15T10:33:00Z"
  }
]"#,
    )?;
    println!("Created sample file: api_logs.json (array format)");

    // Create sample config JSON (object mode)
    let config_json = temp_dir.join("config.json");
    fs::write(
        &config_json,
        r#"{
  "database": {
    "host": "localhost",
    "port": 5432,
    "name": "production",
    "max_connections": 100
  },
  "cache": {
    "enabled": true,
    "ttl_seconds": 3600,
    "max_size_mb": 512
  },
  "logging": {
    "level": "info",
    "format": "json",
    "output": "stdout"
  },
  "features": {
    "analytics": true,
    "notifications": true,
    "backup": false
  }
}"#,
    )?;
    println!("Created sample file: config.json (object format)");

    // Create sample user profile JSON (full mode)
    let profile_json = temp_dir.join("profile.json");
    fs::write(
        &profile_json,
        r#"{
  "user_id": "user_12345",
  "name": "Alice Johnson",
  "email": "alice@example.com",
  "created_at": "2024-01-15T08:00:00Z",
  "preferences": {
    "theme": "dark",
    "language": "en",
    "notifications": true
  },
  "stats": {
    "posts": 42,
    "followers": 1523,
    "following": 387
  }
}"#,
    )?;
    println!("Created sample file: profile.json (full document)\n");

    // Setup database
    let db_path = temp_dir.join("json_example.db");
    println!("Opening database at: {}", db_path.display());
    let db = Database::open(&db_path)?;
    db.initialize()?;

    println!("\n=== Example 1: Index JSON Array (API Logs) ===\n");

    let provider = JSONProvider::new();
    let config = ProviderConfig::new(
        api_logs.to_string_lossy().to_string(),
        "**/*.json".to_string(),
    );

    println!("Parsing api_logs.json (array mode)...");
    let items = provider.list_items(&config).await?;
    println!("Found {} items", items.len());

    // Create collection
    db.add_collection(
        "api-logs",
        api_logs.to_string_lossy().as_ref(),
        "**/*.json",
        "json",
        Some(r#"{"index_mode":"array"}"#),
    )?;

    // Index all items
    let now = Utc::now().to_rfc3339();
    for (idx, item) in items.iter().enumerate() {
        println!("\nItem {}:", idx + 1);
        println!("  URI: {}", item.uri);
        println!("  Title: {}", item.title);
        println!("  Metadata:");
        for (key, value) in &item.metadata {
            if key != "file" && key != "index" && key != "item_type" {
                println!("    {}: {}", key, value);
            }
        }
        println!("  Content Preview:");
        for line in item.content.lines().take(5) {
            println!("    {}", line);
        }

        db.insert_content(&item.hash, &item.content)?;
        db.insert_document(
            "api-logs",
            &item.uri,
            &item.title,
            &item.hash,
            &now,
            &now,
            &item.source_type,
            Some(&item.uri),
        )?;
    }

    println!("\n=== Example 2: Index JSON Object (Config) ===\n");

    let mut config_config = ProviderConfig::new(
        config_json.to_string_lossy().to_string(),
        "**/*.json".to_string(),
    );
    config_config
        .options
        .insert("index_mode".to_string(), "object".to_string());

    println!("Parsing config.json (object mode)...");
    let config_items = provider.list_items(&config_config).await?;
    println!("Found {} top-level keys", config_items.len());

    db.add_collection(
        "config",
        config_json.to_string_lossy().as_ref(),
        "**/*.json",
        "json",
        Some(r#"{"index_mode":"object"}"#),
    )?;

    for (idx, item) in config_items.iter().enumerate() {
        println!("\nKey {}:", idx + 1);
        println!("  Title: {}", item.title);
        println!(
            "  Key: {}",
            item.metadata.get("key").unwrap_or(&"N/A".to_string())
        );
        println!(
            "  Value Type: {}",
            item.metadata
                .get("value_type")
                .unwrap_or(&"N/A".to_string())
        );
        println!("  Content:");
        for line in item.content.lines().take(5) {
            println!("    {}", line);
        }

        db.insert_content(&item.hash, &item.content)?;
        db.insert_document(
            "config",
            &item.uri,
            &item.title,
            &item.hash,
            &now,
            &now,
            &item.source_type,
            Some(&item.uri),
        )?;
    }

    println!("\n=== Example 3: Index Full JSON Document (Profile) ===\n");

    let mut profile_config = ProviderConfig::new(
        profile_json.to_string_lossy().to_string(),
        "**/*.json".to_string(),
    );
    profile_config
        .options
        .insert("index_mode".to_string(), "full".to_string());

    println!("Parsing profile.json (full mode)...");
    let profile_items = provider.list_items(&profile_config).await?;
    println!("Found {} document", profile_items.len());

    db.add_collection(
        "profiles",
        profile_json.to_string_lossy().as_ref(),
        "**/*.json",
        "json",
        Some(r#"{"index_mode":"full"}"#),
    )?;

    for item in profile_items.iter() {
        println!("\nDocument:");
        println!("  URI: {}", item.uri);
        println!("  Title: {}", item.title);
        println!(
            "  Type: {}",
            item.metadata.get("type").unwrap_or(&"N/A".to_string())
        );
        println!("  Content Length: {} bytes", item.content.len());

        db.insert_content(&item.hash, &item.content)?;
        db.insert_document(
            "profiles",
            &item.uri,
            &item.title,
            &item.hash,
            &now,
            &now,
            &item.source_type,
            Some(&item.uri),
        )?;
    }

    println!("\n=== Example 4: Fetch Specific Item by URI ===\n");

    let first_log_uri = format!("json://{}/item_0", api_logs.display());
    println!("Fetching: {}", first_log_uri);

    match provider.fetch_item(&first_log_uri).await {
        Ok(item) => {
            println!("Successfully fetched:");
            println!(
                "  ID: {}",
                item.metadata.get("id").unwrap_or(&"N/A".to_string())
            );
            println!(
                "  Method: {}",
                item.metadata.get("method").unwrap_or(&"N/A".to_string())
            );
            println!(
                "  Endpoint: {}",
                item.metadata.get("endpoint").unwrap_or(&"N/A".to_string())
            );
            println!(
                "  Status: {}",
                item.metadata.get("status").unwrap_or(&"N/A".to_string())
            );
        }
        Err(e) => {
            eprintln!("Error fetching item: {}", e);
        }
    }

    println!("\n=== Example 5: Search Indexed Data ===\n");

    // Reindex to update FTS
    db.reindex_collection("api-logs").await?;
    db.reindex_collection("config").await?;
    db.reindex_collection("profiles").await?;

    // Search API logs
    println!("Searching for 'POST'...");
    let options = agentroot_core::SearchOptions {
        limit: 10,
        min_score: 0.0,
        collection: Some("api-logs".to_string()),
        provider: None,
        metadata_filters: Vec::new(),
        detail: agentroot_core::DetailLevel::L2,
        ..Default::default()
    };
    let results = db.search_fts("POST", &options)?;

    println!("Found {} results", results.len());
    for result in results.iter() {
        println!("  - {} (score: {:.2})", result.title, result.score);
    }

    // Search config
    println!("\nSearching for 'database'...");
    let options = agentroot_core::SearchOptions {
        limit: 10,
        min_score: 0.0,
        collection: Some("config".to_string()),
        provider: None,
        metadata_filters: Vec::new(),
        detail: agentroot_core::DetailLevel::L2,
        ..Default::default()
    };
    let results = db.search_fts("database", &options)?;

    println!("Found {} results", results.len());
    for result in results {
        println!("  - {} (score: {:.2})", result.title, result.score);
    }

    // Search profile
    println!("\nSearching for 'Alice'...");
    let options = agentroot_core::SearchOptions {
        limit: 10,
        min_score: 0.0,
        collection: Some("profiles".to_string()),
        provider: None,
        metadata_filters: Vec::new(),
        detail: agentroot_core::DetailLevel::L2,
        ..Default::default()
    };
    let results = db.search_fts("Alice", &options)?;

    println!("Found {} results", results.len());
    for result in results {
        println!("  - {} (score: {:.2})", result.title, result.score);
    }

    println!("\n=== Summary ===");
    println!("✓ Indexed {} API log entries (array mode)", items.len());
    println!(
        "✓ Indexed {} config sections (object mode)",
        config_items.len()
    );
    println!("✓ Indexed {} user profile (full mode)", profile_items.len());
    println!("✓ All JSON data is searchable with full-text search");
    println!("✓ Each item has structured metadata extracted");
    println!("\nIndex Modes:");
    println!("  • array  - Each array element becomes a document");
    println!("  • object - Each top-level key becomes a document");
    println!("  • full   - Entire JSON as single document");
    println!("\nDatabase saved at: {}", db_path.display());

    Ok(())
}
