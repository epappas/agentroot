# Provider System

Agentroot's provider system enables indexing content from multiple sources beyond local files. This guide explains how to use built-in providers and how to create custom ones.

## Overview

The provider system is based on a simple async trait that any content source can implement:

```rust
#[async_trait::async_trait]
pub trait SourceProvider: Send + Sync {
    /// Provider type identifier (e.g., "file", "github", "url")
    fn provider_type(&self) -> &'static str;

    /// List all items from source (for scanning/indexing)
    async fn list_items(&self, config: &ProviderConfig) -> Result<Vec<SourceItem>>;

    /// Fetch single item by URI
    async fn fetch_item(&self, uri: &str) -> Result<SourceItem>;
}
```

**Note**: As of v0.1.0, the provider system is fully async to support efficient network operations and proper error handling.

## Built-in Providers

### FileProvider

Indexes content from the local file system.

**Type**: `file` (default)

**Features**:
- Glob pattern matching (`**/*.rs`, `src/**/*.{js,ts}`)
- Exclude hidden files and directories
- Follow or ignore symlinks
- Configurable excluded directories

**Usage (CLI)**:
```bash
# Basic usage
agentroot collection add /path/to/code --name myproject --mask '**/*.rs'

# With explicit provider
agentroot collection add /path/to/code \
  --name myproject \
  --mask '**/*.md' \
  --provider file
```

**Usage (Library)**:
```rust
use agentroot_core::Database;

#[tokio::main]
async fn main() -> Result<()> {
    let db = Database::open("index.db")?;
    db.initialize()?;

    // Add file-based collection
    db.add_collection(
        "myproject",
        "/path/to/code",
        "**/*.rs",
        "file",
        None,
    )?;

    // Reindex using FileProvider (async)
    let count = db.reindex_collection("myproject").await?;
    println!("Indexed {} files", count);

    Ok(())
}
```

**Configuration Options**:

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `exclude_hidden` | boolean | `true` | Skip hidden files/directories |
| `follow_symlinks` | boolean | `true` | Follow symbolic links |

**Excluded Directories**:
- `node_modules`
- `.git`
- `.cache`
- `vendor`
- `dist`
- `build`
- `__pycache__`
- `.venv`
- `target`

### GitHubProvider

Indexes content from GitHub repositories.

**Type**: `github`

**Features**:
- Fetch repository README files
- Fetch specific files by URL
- List all files in repository
- Glob pattern filtering
- Authentication support
- Automatic retry with exponential backoff for rate limits
- Descriptive error messages with actionable suggestions
- Rate limit monitoring and warnings

**Usage (CLI)**:
```bash
# Add GitHub repository
agentroot collection add https://github.com/rust-lang/rust \
  --name rust-docs \
  --mask '**/*.md' \
  --provider github

# With authentication for higher rate limits
export GITHUB_TOKEN=ghp_your_token_here
agentroot update
```

**Usage (Library)**:
```rust
use agentroot_core::Database;

#[tokio::main]
async fn main() -> Result<()> {
    let db = Database::open("index.db")?;
    db.initialize()?;

    // Add GitHub collection
    db.add_collection(
        "rust-docs",
        "https://github.com/rust-lang/rust",
        "**/*.md",
        "github",
        None,  // Or: Some(r#"{"github_token": "ghp_..."}"#)
    )?;

    // Reindex using GitHubProvider (async with retry logic)
    match db.reindex_collection("rust-docs").await {
        Ok(count) => println!("Indexed {} files", count),
        Err(e) => {
            eprintln!("Error: {}", e);
            // Error messages include actionable suggestions
        }
    }

    Ok(())
}
```

**Supported URL Formats**:
- Repository: `https://github.com/owner/repo`
- Specific file: `https://github.com/owner/repo/blob/branch/path/file.md`

**Authentication**:

Set the `GITHUB_TOKEN` environment variable or provide it in provider config:

```bash
# Via environment variable (recommended)
export GITHUB_TOKEN=ghp_your_token_here

# Via provider config
db.add_collection(
    "rust-docs",
    "https://github.com/rust-lang/rust",
    "**/*.md",
    "github",
    Some(r#"{"github_token": "ghp_your_token_here"}"#),
)?;
```

**API Rate Limits**:
- **Without authentication**: 60 requests per hour
- **With authentication**: 5,000 requests per hour

For repositories with many files, authentication is strongly recommended.

**Automatic Rate Limit Handling**:

The GitHub provider automatically handles rate limits:
- **Retry logic**: Automatically retries failed requests up to 3 times with exponential backoff
- **Rate limit warnings**: Warns when fewer than 10 requests remain
- **Proper 429 handling**: Respects `Retry-After` headers from GitHub
- **Timeout retry**: Automatically retries transient network failures

**Error Messages**:

The provider provides descriptive error messages with actionable suggestions:
- **404**: "File not found: {path}. Verify the repository, branch, and file path are correct."
- **403**: "GitHub API rate limit exceeded. Set GITHUB_TOKEN environment variable..."
- **401**: "Authentication failed. Your GITHUB_TOKEN may be invalid or expired..."
- **409**: "Repository {owner}/{repo} is empty or has no commits yet."

All error messages include links to GitHub's token generation page when authentication is needed.

**Metadata Captured**:
- `owner`: Repository owner
- `repo`: Repository name
- `branch`: Branch name (for files)
- `path`: File path (for files)

## Provider Architecture

### Data Flow

```
Provider → SourceItem → Database

1. Provider.list_items() returns Vec<SourceItem>
2. Each SourceItem contains:
   - uri: Unique identifier
   - title: Display name
   - content: Full text
   - hash: SHA-256 content hash
   - source_type: Provider type
   - metadata: Provider-specific data
3. Database stores with source tracking
```

### ProviderConfig

Configuration passed to providers:

```rust
pub struct ProviderConfig {
    pub base_path: String,        // Base URL or path
    pub pattern: String,           // Glob pattern
    pub options: HashMap<String, String>, // Provider-specific options
}
```

**Example**:
```rust
let config = ProviderConfig::new(
    "https://github.com/rust-lang/rust".to_string(),
    "**/*.md".to_string(),
)
.with_option("github_token".to_string(), "ghp_...".to_string());
```

### SourceItem

Item returned by providers:

```rust
pub struct SourceItem {
    pub uri: String,               // "owner/repo/README.md"
    pub title: String,             // "README"
    pub content: String,           // Full file content
    pub hash: String,              // SHA-256 hash
    pub source_type: String,       // "github"
    pub metadata: HashMap<String, String>, // Extra data
}
```

### ProviderRegistry

Manages available providers:

```rust
use agentroot_core::ProviderRegistry;

// Get default registry (file + github)
let registry = ProviderRegistry::with_defaults();

// Get specific provider
let provider = registry.get("github").unwrap();

// Use provider directly
let config = ProviderConfig::new(base_path, pattern);
let items = provider.list_items(&config)?;
```

## Creating Custom Providers

### Step 1: Implement SourceProvider Trait

```rust
use agentroot_core::{ProviderConfig, SourceItem, SourceProvider, Result};

pub struct MyProvider {
    // Provider state (API clients, config, etc.)
    client: reqwest::Client,
}

impl MyProvider {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait::async_trait]
impl SourceProvider for MyProvider {
    fn provider_type(&self) -> &'static str {
        "myprovider"
    }

    async fn list_items(&self, config: &ProviderConfig) -> Result<Vec<SourceItem>> {
        // Implement: fetch all items matching config.pattern
        // Use async/await for network operations
        let items = vec![]; // Your implementation here
        Ok(items)
    }

    async fn fetch_item(&self, uri: &str) -> Result<SourceItem> {
        // Implement: fetch single item by URI
        // Use async/await for network operations
        todo!()
    }
}
```

### Step 2: Register Provider

```rust
// In your provider module
use std::sync::Arc;

pub fn register_provider(registry: &mut ProviderRegistry) {
    registry.register(Arc::new(MyProvider::new()));
}
```

### Step 3: Add Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_type() {
        let provider = MyProvider::new();
        assert_eq!(provider.provider_type(), "myprovider");
    }

    #[tokio::test]
    async fn test_list_items() {
        let provider = MyProvider::new();
        let config = ProviderConfig::new(
            "https://example.com".to_string(),
            "**/*.txt".to_string(),
        );
        let items = provider.list_items(&config).await.unwrap();
        assert!(!items.is_empty());
    }
}
```

### Step 4: Update Registry

Modify `crates/agentroot-core/src/providers/mod.rs`:

```rust
pub mod my_provider;
pub use my_provider::MyProvider;

impl ProviderRegistry {
    pub fn with_defaults() -> Self {
        let mut registry = Self::new();
        registry.register(Arc::new(FileProvider::new()));
        registry.register(Arc::new(GitHubProvider::new()));
        registry.register(Arc::new(MyProvider::new())); // Add your provider
        registry
    }
}
```

### Example: URL Provider

Here's a complete example of an async URL provider:

```rust
use agentroot_core::{ProviderConfig, SourceItem, SourceProvider, Result};
use agentroot_core::error::AgentRootError;
use agentroot_core::db::hash_content;

pub struct URLProvider {
    client: reqwest::Client,
}

impl URLProvider {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .user_agent("agentroot/1.0")
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self { client }
    }

    async fn fetch_url(&self, url: &str) -> Result<String> {
        let response = self.client.get(url).send().await.map_err(|e| {
            AgentRootError::ExternalError(format!(
                "Failed to fetch URL: {}. Check your internet connection.",
                e
            ))
        })?;

        if !response.status().is_success() {
            let status = response.status();
            return Err(AgentRootError::ExternalError(format!(
                "HTTP error {}: {}",
                status.as_u16(),
                status.canonical_reason().unwrap_or("Unknown error")
            )));
        }

        response.text().await.map_err(|e| {
            AgentRootError::ExternalError(format!("Failed to read response body: {}", e))
        })
    }
}

#[async_trait::async_trait]
impl SourceProvider for URLProvider {
    fn provider_type(&self) -> &'static str {
        "url"
    }

    async fn list_items(&self, config: &ProviderConfig) -> Result<Vec<SourceItem>> {
        // For URLs, base_path is the URL to fetch
        let item = self.fetch_item(&config.base_path).await?;
        Ok(vec![item])
    }

    async fn fetch_item(&self, uri: &str) -> Result<SourceItem> {
        let content = self.fetch_url(uri).await?;
        let title = uri.split('/').last().unwrap_or(uri).to_string();
        let hash = hash_content(&content);

        Ok(SourceItem::new(
            uri.to_string(),
            title,
            content,
            hash,
            "url".to_string(),
        )
        .with_metadata("url".to_string(), uri.to_string()))
    }
}
```

## Provider Use Cases

### Documentation Sites

Index documentation from multiple sources:

```bash
# Local docs
agentroot collection add ./docs --name local-docs --mask '**/*.md' --provider file

# GitHub docs
agentroot collection add https://github.com/rust-lang/rust \
  --name rust-docs \
  --mask '**/*.md' \
  --provider github

# Update all
agentroot update
agentroot embed
```

### Multi-Repository Codebase

Index multiple repositories:

```bash
# Add multiple GitHub repos
agentroot collection add https://github.com/org/frontend --name frontend --provider github
agentroot collection add https://github.com/org/backend --name backend --provider github
agentroot collection add https://github.com/org/shared --name shared --provider github

# Search across all
agentroot query "authentication flow"
```

### Hybrid Sources

Mix local and remote sources:

```bash
# Local development
agentroot collection add ~/projects/myapp --name myapp --provider file

# Dependencies from GitHub
agentroot collection add https://github.com/tokio-rs/tokio --name tokio --provider github

# Search both
agentroot query "async runtime performance"
```

### Collection Management via MCP

Agentroot provides MCP (Model Context Protocol) tools for managing collections programmatically:

**Available MCP Tools**:
- `collection_add`: Add a new collection
- `collection_remove`: Remove a collection
- `collection_update`: Reindex a collection
- `status`: View provider statistics

**Example Usage (via MCP)**:
```json
{
  "name": "collection_add",
  "arguments": {
    "name": "rust-docs",
    "path": "https://github.com/rust-lang/rust",
    "pattern": "**/*.md",
    "provider": "github",
    "config": "{\"github_token\": \"ghp_...\"}"
  }
}
```

**Search with Provider Filter**:
```json
{
  "name": "search",
  "arguments": {
    "query": "async runtime",
    "provider": "github",
    "limit": 10
  }
}
```

This filters search results to only show documents from GitHub collections.

**Provider Statistics**:

The `status` MCP tool now shows per-provider statistics:
```json
{
  "providers": [
    {"provider": "file", "collections": 2, "documents": 150},
    {"provider": "github", "collections": 3, "documents": 45}
  ]
}
```

## Best Practices

### 1. Use Appropriate Providers

- **FileProvider**: Local development, quick iteration
- **GitHubProvider**: Public repositories, documentation
- **Custom providers**: Specialized sources (databases, APIs, etc.)

### 2. Pattern Matching

Use specific patterns to reduce indexing time:

```bash
# Good: Specific file types
--mask '**/*.{rs,toml}'

# Less good: Too broad
--mask '**/*'
```

### 3. Authentication

Always use authentication for GitHub:

```bash
export GITHUB_TOKEN=ghp_your_token_here
```

### 4. Incremental Updates

Providers support incremental updates via content hashing:

```bash
# First run: indexes everything
agentroot update

# Subsequent runs: only changed files
agentroot update  # Much faster!
```

### 5. Error Handling

Handle provider errors gracefully:

```rust
match db.reindex_collection("github-repo") {
    Ok(count) => println!("Indexed {} files", count),
    Err(e) => eprintln!("Failed to index: {}", e),
}
```

## Troubleshooting

### GitHub API Rate Limits

**Problem**: `GitHub API error: 403`

**Solution**: Set `GITHUB_TOKEN` environment variable

```bash
export GITHUB_TOKEN=ghp_your_token_here
agentroot update
```

### Network Errors

**Problem**: `HTTP error: connection refused`

**Solution**: Check internet connection and proxy settings

```bash
# Set proxy if needed
export HTTP_PROXY=http://proxy.example.com:8080
export HTTPS_PROXY=http://proxy.example.com:8080
```

### Pattern Not Matching

**Problem**: No files indexed from GitHub

**Solution**: Verify glob pattern and repository structure

```bash
# Test pattern locally first
agentroot collection add https://github.com/owner/repo --name test --mask '**/*.md'
agentroot update
agentroot collection list  # Check document count
```

### Provider Not Found

**Problem**: `Unknown provider type: xyz`

**Solution**: Check provider is registered

```rust
let registry = ProviderRegistry::with_defaults();
let types = registry.list_types();
println!("Available providers: {:?}", types);
```

## Performance Considerations

### FileProvider

- **Fast**: Direct file system access
- **Scales**: Handles 10,000+ files easily
- **Cache-friendly**: High cache hit rates on re-index

### GitHubProvider

- **Network-bound**: Limited by API rate limits
- **Caching**: GitHub returns ETags for efficient updates
- **Batch operations**: Use authentication for better limits

### Custom Providers

- **Implement caching**: Store fetched content locally
- **Handle rate limits**: Implement backoff and retry
- **Parallel fetching**: Use async/await for performance

## Future Providers

Planned providers for future releases:

- **URLProvider**: Index web pages and documents
- **PDFProvider**: Extract text from PDF files
- **SQLProvider**: Index database content
- **S3Provider**: Index files from AWS S3
- **CalendarProvider**: Index calendar events
- **NotionProvider**: Index Notion pages
- **ConfluenceProvider**: Index Confluence pages

Contributions welcome! See [CONTRIBUTING.md](../CONTRIBUTING.md) for guidelines.

## API Reference

See [AGENTS.md](../AGENTS.md) for complete API documentation and technical details.
