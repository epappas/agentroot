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
pub trait SourceProvider: Send + Sync {
    /// Provider type identifier (e.g., "file", "github", "url")
    fn provider_type(&self) -> &'static str;

    /// List all items from source (for scanning/indexing)
    fn list_items(&self, config: &ProviderConfig) -> Result<Vec<SourceItem>>;

    /// Fetch single item by URI
    fn fetch_item(&self, uri: &str) -> Result<SourceItem>;
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

    /// List all registered provider types
    pub fn list_types(&self) -> Vec<String> {
        self.providers.keys().cloned().collect()
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::with_defaults()
    }
}
