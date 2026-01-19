//! Status command

use anyhow::Result;
use agentroot_core::Database;
use crate::app::OutputFormat;

pub async fn run(db: &Database, format: OutputFormat) -> Result<()> {
    let stats = db.get_stats()?;

    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&stats)?);
        }
        _ => {
            println!("Collections: {}", stats.collection_count);
            println!("Documents:   {}", stats.document_count);
            println!("Embedded:    {}", stats.embedded_count);
            println!("Pending:     {}", stats.pending_embedding);
        }
    }
    Ok(())
}
