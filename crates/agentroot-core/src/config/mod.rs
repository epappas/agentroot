//! Configuration management

pub mod virtual_path;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use crate::error::Result;

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    /// Global context applied to all searches
    #[serde(default)]
    pub global_context: Option<String>,

    /// Collection configurations
    #[serde(default)]
    pub collections: HashMap<String, CollectionConfig>,
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
        let mut matching: Vec<(&str, &str)> = collection_config.context
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
