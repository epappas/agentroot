//! Status command

use crate::app::OutputFormat;
use agentroot_core::Database;
use anyhow::Result;

pub async fn run(db: &Database, format: OutputFormat) -> Result<()> {
    let stats = db.get_stats()?;

    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&stats)?);
        }
        _ => {
            println!("Collections:     {}", stats.collection_count);
            println!("Documents:       {}", stats.document_count);
            println!();
            println!("Embeddings:");
            println!("  Embedded:      {}", stats.embedded_count);
            println!("  Pending:       {}", stats.pending_embedding);
            println!();
            println!("Metadata:");
            println!("  Generated:     {}", stats.metadata_count);
            println!("  Pending:       {}", stats.pending_metadata);
        }
    }
    Ok(())
}
