//! Embed command

use crate::app::EmbedArgs;
use agentroot_core::index::{embed_documents, EmbedProgress};
use agentroot_core::{CandleEmbedder, Database, Embedder, DEFAULT_CANDLE_MODEL};
use anyhow::Result;

pub async fn run(args: EmbedArgs, db: &Database) -> Result<()> {
    // Run migration to ensure schema is up to date
    db.migrate()?;

    // Use Candle-based embedder
    println!("Loading embedding model...");
    let embedder = if let Some(model_name) = args.model.as_ref().and_then(|p| p.to_str()) {
        // Try loading from custom model name/path
        CandleEmbedder::from_hf(model_name)?
    } else {
        // Use default (downloads from HuggingFace if needed)
        CandleEmbedder::from_default()?
    };

    println!(
        "Model loaded: {} ({} dimensions)",
        embedder.model_name(),
        embedder.dimensions()
    );

    let model_name = embedder.model_name().to_string();

    println!("Starting embedding pipeline...");

    // Run embedding pipeline
    let stats = embed_documents(
        db,
        &embedder,
        &model_name,
        args.force,
        Some(Box::new(|progress: EmbedProgress| {
            let cache_pct = if progress.processed_chunks > 0 {
                progress.cached_chunks as f64 / progress.processed_chunks as f64 * 100.0
            } else {
                0.0
            };
            eprint!(
                "\rProcessing: {}/{} docs, {}/{} chunks ({:.0}% cached)   ",
                progress.processed_docs,
                progress.total_docs,
                progress.processed_chunks,
                progress.total_chunks,
                cache_pct
            );
        })),
    )
    .await?;

    eprintln!();
    println!("Embedding complete:");
    println!(
        "  Documents: {}/{}",
        stats.embedded_documents, stats.total_documents
    );
    println!(
        "  Chunks:    {} ({} cached, {} computed)",
        stats.embedded_chunks, stats.cached_chunks, stats.computed_chunks
    );
    if stats.embedded_chunks > 0 {
        println!("  Cache hit rate: {:.1}%", stats.cache_hit_rate());
    }

    Ok(())
}
