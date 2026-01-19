//! Source provider abstraction
//!
//! Provides a unified interface for indexing content from different sources:
//! - File system (local files)
//! - GitHub (repositories, files, gists)
//! - URLs (web pages, PDFs)
//! - Databases (SQL, NoSQL)
//! - Calendar/Notes/Books
//!
//! Each provider implements the SourceProvider trait to enable seamless
//! integration with agentroot's indexing and search capabilities.

use crate::error::Result;
use std::collections::HashMap;
use std::sync::Arc;

pub mod file;
pub mod github;

pub use file::FileProvider;
pub use github::GitHubProvider;

/// Source provider trait - all content sources must implement this
#[async_trait::async_trait]
pub trait SourceProvider: Send + Sync {
    /// Provider type identifier (e.g., "file", "github", "url")
    fn provider_type(&self) -> &'static str;

    /// List all items from source (for scanning/indexing)
    async fn list_items(&self, config: &ProviderConfig) -> Result<Vec<SourceItem>>;

    /// Fetch single item by URI
    async fn fetch_item(&self, uri: &str) -> Result<SourceItem>;
}

/// Configuration for a provider instance
#[derive(Debug, Clone)]
pub struct ProviderConfig {
    /// Base path/URL for the provider
    pub base_path: String,

    /// Pattern to match items (glob for files, filter for others)
    pub pattern: String,

    /// Provider-specific options (auth tokens, filters, etc.)
    pub options: HashMap<String, String>,
}

impl ProviderConfig {
    /// Create new provider config
    pub fn new(base_path: String, pattern: String) -> Self {
        Self {
            base_path,
            pattern,
            options: HashMap::new(),
        }
    }

    /// Add option to config
    pub fn with_option(mut self, key: String, value: String) -> Self {
        self.options.insert(key, value);
        self
    }

    /// Get option value
    pub fn get_option(&self, key: &str) -> Option<&String> {
        self.options.get(key)
    }
}

/// Item from a source provider
#[derive(Debug, Clone)]
pub struct SourceItem {
    /// Unique identifier within collection (path for files, URL for GitHub)
    pub uri: String,

    /// Display title for the item
    pub title: String,

    /// Full content of the item
    pub content: String,

    /// Content hash (SHA-256)
    pub hash: String,

    /// Provider type that created this item
    pub source_type: String,

    /// Provider-specific metadata (commit SHA, author, URL, etc.)
    pub metadata: HashMap<String, String>,
}

impl SourceItem {
    /// Create new source item
    pub fn new(
        uri: String,
        title: String,
        content: String,
        hash: String,
        source_type: String,
    ) -> Self {
        Self {
            uri,
            title,
            content,
            hash,
            source_type,
            metadata: HashMap::new(),
        }
    }

    /// Add metadata to item
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

/// Registry for managing provider instances
pub struct ProviderRegistry {
    providers: HashMap<String, Arc<dyn SourceProvider>>,
}

impl ProviderRegistry {
    /// Create new empty registry
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
        }
    }

    /// Create registry with default providers
    pub fn with_defaults() -> Self {
        let mut registry = Self::new();
        registry.register(Arc::new(FileProvider::new()));
        registry.register(Arc::new(GitHubProvider::new()));
        registry
    }

    /// Register a provider
    pub fn register(&mut self, provider: Arc<dyn SourceProvider>) {
        self.providers
            .insert(provider.provider_type().to_string(), provider);
    }

    /// Get provider by type
    pub fn get(&self, provider_type: &str) -> Option<Arc<dyn SourceProvider>> {
        self.providers.get(provider_type).cloned()
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::with_defaults()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_config_json_parsing() {
        let json_config =
            r#"{"exclude_hidden":"false","follow_symlinks":"true","custom_key":"custom_value"}"#;

        let config_map: std::collections::HashMap<String, String> =
            serde_json::from_str(json_config).unwrap();

        assert_eq!(config_map.get("exclude_hidden"), Some(&"false".to_string()));
        assert_eq!(config_map.get("follow_symlinks"), Some(&"true".to_string()));
        assert_eq!(
            config_map.get("custom_key"),
            Some(&"custom_value".to_string())
        );

        let mut config = ProviderConfig::new("/tmp".to_string(), "**/*.md".to_string());
        for (key, value) in config_map {
            config = config.with_option(key, value);
        }

        assert_eq!(
            config.get_option("exclude_hidden"),
            Some(&"false".to_string())
        );
        assert_eq!(
            config.get_option("follow_symlinks"),
            Some(&"true".to_string())
        );
        assert_eq!(
            config.get_option("custom_key"),
            Some(&"custom_value".to_string())
        );
    }

    #[test]
    fn test_provider_config_json_empty() {
        let json_config = r#"{}"#;
        let config_map: std::collections::HashMap<String, String> =
            serde_json::from_str(json_config).unwrap();
        assert_eq!(config_map.len(), 0);
    }

    #[test]
    fn test_provider_config_json_invalid() {
        let json_config = r#"{"key": invalid}"#;
        let result: std::result::Result<std::collections::HashMap<String, String>, _> =
            serde_json::from_str(json_config);
        assert!(result.is_err());
    }

    #[test]
    fn test_provider_config_json_nested_not_supported() {
        let json_config = r#"{"key": {"nested": "value"}}"#;
        let result: std::result::Result<std::collections::HashMap<String, String>, _> =
            serde_json::from_str(json_config);
        assert!(
            result.is_err(),
            "Nested JSON should not parse into HashMap<String, String>"
        );
    }

    #[test]
    fn test_provider_config_special_characters() {
        let json_config =
            r#"{"path":"/tmp/test with spaces","pattern":"**/*.{md,txt}","token":"ghp_abc123"}"#;

        let config_map: std::collections::HashMap<String, String> =
            serde_json::from_str(json_config).unwrap();

        assert_eq!(
            config_map.get("path"),
            Some(&"/tmp/test with spaces".to_string())
        );
        assert_eq!(
            config_map.get("pattern"),
            Some(&"**/*.{md,txt}".to_string())
        );
        assert_eq!(config_map.get("token"), Some(&"ghp_abc123".to_string()));
    }
}
