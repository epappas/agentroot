//! Embed command

use crate::app::EmbedArgs;
use agentroot_core::index::{embed_documents, EmbedProgress};
use agentroot_core::{Database, Embedder, HttpEmbedder, LlamaEmbedder, DEFAULT_EMBED_MODEL};
use anyhow::Result;
use std::sync::Arc;

pub async fn run(args: EmbedArgs, db: &Database) -> Result<()> {
    // Run migration to ensure schema is up to date
    db.migrate()?;

    // Try HTTP embedder first (from env vars or config)
    let embedder: Arc<dyn Embedder> = if let Ok(http_embedder) = HttpEmbedder::from_env() {
        println!(
            "Using HTTP embedding service: {}",
            http_embedder.model_name()
        );
        println!("Dimensions: {}", http_embedder.dimensions());
        Arc::new(http_embedder)
    } else {
        // Fall back to local embedder
        let model_path = if let Some(path) = args.model {
            path
        } else {
            let model_dir = dirs::data_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("."))
                .join("agentroot")
                .join("models");
            model_dir.join(DEFAULT_EMBED_MODEL)
        };

        if !model_path.exists() {
            eprintln!(
                "Error: Embedding model not found at {}",
                model_path.display()
            );
            eprintln!();
            eprintln!("To use vector embeddings, you have two options:");
            eprintln!();
            eprintln!("Option 1: Use an HTTP embedding service (recommended)");
            eprintln!("  Set environment variables:");
            eprintln!("    export AGENTROOT_EMBEDDING_URL=\"https://your-service.com\"");
            eprintln!("    export AGENTROOT_EMBEDDING_MODEL=\"intfloat/e5-mistral-7b-instruct\"");
            eprintln!("    export AGENTROOT_EMBEDDING_DIMS=\"4096\"");
            eprintln!();
            eprintln!("Option 2: Use a local GGUF model");
            eprintln!("  1. Create the models directory:");
            eprintln!("     mkdir -p ~/.local/share/agentroot/models");
            eprintln!(
                "  2. Download a GGUF embedding model (e.g., nomic-embed-text-v1.5.Q4_K_M.gguf)"
            );
            eprintln!("  3. Place it in ~/.local/share/agentroot/models/");
            eprintln!();
            return Err(anyhow::anyhow!("No embedding service available"));
        }

        println!("Loading local embedding model: {}", model_path.display());
        let local_embedder = LlamaEmbedder::new(&model_path)?;
        Arc::new(local_embedder)
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
