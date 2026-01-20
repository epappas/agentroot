//! Simple model downloader

use crate::error::{AgentRootError, Result};
use std::fs;
use std::path::{Path, PathBuf};

const HF_BASE_URL: &str = "https://huggingface.co";

/// Download a file from Hugging Face
pub fn download_hf_file(model_id: &str, filename: &str, cache_dir: &Path) -> Result<PathBuf> {
    // Create cache directory structure
    let model_cache = cache_dir.join(format!("models--{}", model_id.replace('/', "--")));
    let snapshot_dir = model_cache.join("snapshots").join("main");
    fs::create_dir_all(&snapshot_dir)?;

    let file_path = snapshot_dir.join(filename);

    // Return if already downloaded
    if file_path.exists() {
        tracing::debug!("File already cached: {}", file_path.display());
        return Ok(file_path);
    }

    // Download from HF
    let url = format!("{}/{}/resolve/main/{}", HF_BASE_URL, model_id, filename);
    tracing::info!("Downloading {} from {}", filename, url);

    let response = reqwest::blocking::get(&url).map_err(|e| AgentRootError::Http(e))?;

    if !response.status().is_success() {
        return Err(AgentRootError::ExternalError(format!(
            "Failed to download {}: HTTP {}",
            filename,
            response.status()
        )));
    }

    let bytes = response.bytes().map_err(|e| AgentRootError::Http(e))?;
    fs::write(&file_path, bytes)?;

    tracing::info!(
        "Downloaded {} ({} bytes)",
        filename,
        file_path.metadata()?.len()
    );

    Ok(file_path)
}

/// Download all files needed for a sentence-transformers model
pub fn download_sentence_transformer(model_id: &str) -> Result<PathBuf> {
    let cache_dir = dirs::cache_dir()
        .ok_or_else(|| AgentRootError::Config("Cannot determine cache directory".to_string()))?
        .join("huggingface")
        .join("hub");

    tracing::info!("Downloading sentence-transformer model: {}", model_id);

    // Download required files
    download_hf_file(model_id, "config.json", &cache_dir)?;
    download_hf_file(model_id, "tokenizer.json", &cache_dir)?;
    download_hf_file(model_id, "model.safetensors", &cache_dir)?;

    // Return snapshot directory
    let model_cache = cache_dir.join(format!("models--{}", model_id.replace('/', "--")));
    let snapshot_dir = model_cache.join("snapshots").join("main");

    Ok(snapshot_dir)
}
