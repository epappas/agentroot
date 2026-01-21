//! Configuration management

pub mod virtual_path;

use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    /// Global context applied to all searches
    #[serde(default)]
    pub global_context: Option<String>,

    /// Collection configurations
    #[serde(default)]
    pub collections: HashMap<String, CollectionConfig>,

    /// LLM service configuration
    #[serde(default)]
    pub llm_service: LLMServiceConfig,
}

/// LLM service configuration for external inference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMServiceConfig {
    /// Base URL of the LLM service for chat/completions
    pub url: String,

    /// Model name for chat completions (query parsing, metadata generation)
    #[serde(default = "default_chat_model")]
    pub model: String,

    /// Base URL for embeddings service (can be different from LLM URL)
    #[serde(default)]
    pub embedding_url: Option<String>,

    /// Model name for embeddings
    #[serde(default = "default_embedding_model")]
    pub embedding_model: String,

    /// Embedding dimensions (will be auto-detected if not specified)
    #[serde(default)]
    pub embedding_dimensions: Option<usize>,

    /// API key (optional, for authenticated services)
    #[serde(default)]
    pub api_key: Option<String>,

    /// Request timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
}

impl LLMServiceConfig {
    /// Get the embeddings URL (falls back to main URL if not specified)
    pub fn embeddings_url(&self) -> &str {
        self.embedding_url.as_deref().unwrap_or(&self.url)
    }
}

impl Default for LLMServiceConfig {
    fn default() -> Self {
        Self {
            url: std::env::var("AGENTROOT_LLM_URL")
                .unwrap_or_else(|_| "http://localhost:8000".to_string()),
            model: default_chat_model(),
            embedding_url: std::env::var("AGENTROOT_EMBEDDING_URL").ok(),
            embedding_model: default_embedding_model(),
            embedding_dimensions: std::env::var("AGENTROOT_EMBEDDING_DIMS")
                .ok()
                .and_then(|s| s.parse().ok()),
            api_key: std::env::var("AGENTROOT_LLM_API_KEY").ok(),
            timeout_secs: default_timeout(),
        }
    }
}

fn default_chat_model() -> String {
    std::env::var("AGENTROOT_LLM_MODEL")
        .unwrap_or_else(|_| "meta-llama/Llama-3.1-8B-Instruct".to_string())
}

fn default_embedding_model() -> String {
    std::env::var("AGENTROOT_EMBEDDING_MODEL")
        .unwrap_or_else(|_| "sentence-transformers/all-MiniLM-L6-v2".to_string())
}

fn default_timeout() -> u64 {
    30
}

/// Per-collection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionConfig {
    /// Root path of the collection
    pub path: PathBuf,

    /// Glob pattern for files to index
    #[serde(default = "default_pattern")]
    pub pattern: String,

    /// Context strings keyed by path prefix
    #[serde(default)]
    pub context: HashMap<String, String>,

    /// Command to run before updating (e.g., git pull)
    #[serde(default)]
    pub update: Option<String>,
}

fn default_pattern() -> String {
    "**/*.md".to_string()
}

impl Config {
    /// Load config from default path
    pub fn load() -> Result<Self> {
        let path = Self::default_path();
        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            let config: Config = serde_yaml::from_str(&content)?;
            Ok(config)
        } else {
            Ok(Config::default())
        }
    }

    /// Save config to default path
    pub fn save(&self) -> Result<()> {
        let path = Self::default_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_yaml::to_string(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Get default config path
    pub fn default_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(crate::CONFIG_DIR_NAME)
            .join("config.yml")
    }

    /// Get context for a path (uses hierarchical inheritance)
    pub fn get_context_for_path(&self, collection: &str, path: &str) -> Option<String> {
        let collection_config = self.collections.get(collection)?;

        // Collect all matching contexts
        let mut matching: Vec<(&str, &str)> = collection_config
            .context
            .iter()
            .filter(|(prefix, _)| path.starts_with(*prefix) || prefix.is_empty() || *prefix == "/")
            .map(|(prefix, ctx)| (prefix.as_str(), ctx.as_str()))
            .collect();

        // Sort by prefix length (shortest first for inheritance)
        matching.sort_by_key(|(prefix, _)| prefix.len());

        // Combine contexts (general to specific)
        if matching.is_empty() {
            self.global_context.clone()
        } else {
            let combined: Vec<&str> = matching.iter().map(|(_, ctx)| *ctx).collect();
            let mut result = combined.join("\n\n");
            if let Some(ref global) = self.global_context {
                result = format!("{}\n\n{}", global, result);
            }
            Some(result)
        }
    }
}
