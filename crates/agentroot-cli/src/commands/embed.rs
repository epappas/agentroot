//! Embed command

use anyhow::Result;
use agentroot_core::{Database, Embedder, LlamaEmbedder, DEFAULT_EMBED_MODEL};
use agentroot_core::index::{embed_documents, EmbedProgress};
use crate::app::EmbedArgs;

pub async fn run(args: EmbedArgs, db: &Database) -> Result<()> {
    // Run migration to ensure schema is up to date
    db.migrate()?;

    // Determine model path
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
        eprintln!("Error: Embedding model not found at {}", model_path.display());
        eprintln!();
        eprintln!("To use vector embeddings, download an embedding model:");
        eprintln!("  1. Create the models directory:");
        eprintln!("     mkdir -p ~/.local/share/agentroot/models");
        eprintln!();
        eprintln!("  2. Download a GGUF embedding model, e.g.:");
        eprintln!("     - nomic-embed-text-v1.5.Q4_K_M.gguf");
        eprintln!("     - bge-small-en-v1.5.Q4_K_M.gguf");
        eprintln!();
        eprintln!("  3. Place it in ~/.local/share/agentroot/models/");
        eprintln!();
        eprintln!("Or specify a model path with: agentroot embed --model /path/to/model.gguf");
        return Err(anyhow::anyhow!("Model not found"));
    }

    println!("Loading embedding model: {}", model_path.display());
    let embedder = LlamaEmbedder::new(&model_path)?;
    println!("Model loaded: {} ({} dimensions)", embedder.model_name(), embedder.dimensions());

    let model_name = embedder.model_name().to_string();

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
    ).await?;

    eprintln!();
    println!("Embedding complete:");
    println!("  Documents: {}/{}", stats.embedded_documents, stats.total_documents);
    println!("  Chunks:    {} ({} cached, {} computed)",
        stats.embedded_chunks,
        stats.cached_chunks,
        stats.computed_chunks
    );
    if stats.embedded_chunks > 0 {
        println!("  Cache hit rate: {:.1}%", stats.cache_hit_rate());
    }

    Ok(())
}
