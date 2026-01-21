//! Embed command

use crate::app::EmbedArgs;
use agentroot_core::index::{embed_documents, EmbedProgress};
use agentroot_core::{Database, Embedder, HttpEmbedder};
use anyhow::Result;
use std::sync::Arc;

pub async fn run(args: EmbedArgs, db: &Database) -> Result<()> {
    // Run migration to ensure schema is up to date
    db.migrate()?;

    // Get HTTP embedder from environment variables
    let embedder: Arc<dyn Embedder> = match HttpEmbedder::from_env() {
        Ok(http_embedder) => {
            println!(
                "Using HTTP embedding service: {}",
                http_embedder.model_name()
            );
            println!("Dimensions: {}", http_embedder.dimensions());
            Arc::new(http_embedder)
        }
        Err(_) => {
            eprintln!("Error: No embedding service configured");
            eprintln!();
            eprintln!("AgentRoot requires an external embedding service.");
            eprintln!("Configure one by setting environment variables:");
            eprintln!();
            eprintln!("  export AGENTROOT_EMBEDDING_URL=\"https://your-service.com/v1\"");
            eprintln!("  export AGENTROOT_EMBEDDING_MODEL=\"intfloat/e5-mistral-7b-instruct\"");
            eprintln!("  export AGENTROOT_EMBEDDING_DIMS=\"4096\"");
            eprintln!();
            eprintln!("Supported services:");
            eprintln!("  - vLLM (https://docs.vllm.ai)");
            eprintln!("  - Basilica (https://basilica.ai) - Recommended");
            eprintln!("  - OpenAI (https://openai.com/api)");
            eprintln!("  - Any OpenAI-compatible API");
            eprintln!();
            eprintln!("See VLLM_SETUP.md for detailed instructions.");
            return Err(anyhow::anyhow!("No embedding service configured"));
        }
    };

    println!(
        "Model loaded: {} ({} dimensions)",
        embedder.model_name(),
        embedder.dimensions()
    );

    let model_name = embedder.model_name().to_string();

    // Run embedding pipeline
    let stats = embed_documents(
        db,
        embedder.as_ref(),
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
