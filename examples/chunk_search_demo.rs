//! Interactive chunk search demo
//! 
//! Usage:
//!   cargo run --example chunk_search_demo "your search query"
//!   cargo run --example chunk_search_demo "label:layer:service"

use agentroot_core::{Database, SearchOptions};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let db_path = Database::default_path();
    let db = Database::open(&db_path)?;
    db.initialize()?;

    let query = std::env::args().nth(1).unwrap_or_else(|| "database".to_string());
    
    println!("🔍 Chunk Search: '{}'\n", query);
    
    let options = SearchOptions {
        limit: 10,
        min_score: 0.0,
        collection: None,
        provider: None,
        metadata_filters: vec![],
        ..Default::default()
    };

    let results = db.search_chunks_bm25(&query, &options)?;
    
    if results.is_empty() {
        println!("No chunks found matching '{}'", query);
        return Ok(());
    }
    
    for (i, result) in results.iter().enumerate() {
        println!("{}. {} ({})", 
            i + 1,
            result.chunk_breadcrumb.as_ref().unwrap_or(&"<no breadcrumb>".to_string()),
            result.chunk_type.as_ref().unwrap_or(&"Unknown".to_string())
        );
        println!("   📁 {}", result.display_path);
        println!("   📍 Lines {}-{}", 
            result.chunk_start_line.unwrap_or(0),
            result.chunk_end_line.unwrap_or(0)
        );
        
        if let Some(summary) = &result.chunk_summary {
            println!("   💡 {}", summary);
        }
        
        if !result.chunk_labels.is_empty() {
            let labels: Vec<String> = result.chunk_labels
                .iter()
                .map(|(k, v)| format!("{}:{}", k, v))
                .collect();
            println!("   🏷️  {}", labels.join(", "));
        }
        
        println!("   ⭐ Score: {:.3}\n", result.score);
    }
    
    println!("✨ Found {} chunks", results.len());
    
    Ok(())
}
