//! URL Provider Example
//!
//! Demonstrates how to create a custom provider that fetches content from URLs.
//! This example shows:
//! - Implementing the SourceProvider trait with async methods
//! - Proper error handling with descriptive messages
//! - Using reqwest for HTTP requests
//! - Integration with the database

use agentroot_core::db::hash_content;
use agentroot_core::error::AgentRootError;
use agentroot_core::{Database, ProviderConfig, SourceItem, SourceProvider};
use chrono::Utc;

/// URL Provider - fetches content from web pages
pub struct URLProvider {
    client: reqwest::Client,
}

impl Default for URLProvider {
    fn default() -> Self {
        Self::new()
    }
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

    async fn fetch_url(&self, url: &str) -> agentroot_core::Result<String> {
        let response = self.client.get(url).send().await.map_err(|e| {
            AgentRootError::ExternalError(format!(
                "Failed to fetch URL {}: {}. Check your internet connection.",
                url, e
            ))
        })?;

        let status = response.status();
        if !status.is_success() {
            let error_msg = match status.as_u16() {
                404 => format!("URL not found: {}", url),
                403 => format!(
                    "Access forbidden: {}. The server rejected the request.",
                    url
                ),
                500..=599 => {
                    format!(
                        "Server error {}: {}. Try again later.",
                        status.as_u16(),
                        url
                    )
                }
                _ => format!(
                    "HTTP error {}: {}",
                    status.as_u16(),
                    status.canonical_reason().unwrap_or("Unknown error")
                ),
            };
            return Err(AgentRootError::ExternalError(error_msg));
        }

        response.text().await.map_err(|e| {
            AgentRootError::ExternalError(format!("Failed to read response body: {}", e))
        })
    }

    fn extract_title(&self, content: &str, url: &str) -> String {
        if let Some(title) = content.lines().find(|line| line.trim().starts_with("# ")) {
            return title.trim_start_matches("# ").trim().to_string();
        }

        url.split('/')
            .next_back()
            .unwrap_or(url)
            .trim_end_matches(".md")
            .trim_end_matches(".html")
            .to_string()
    }
}

#[async_trait::async_trait]
impl SourceProvider for URLProvider {
    fn provider_type(&self) -> &'static str {
        "url"
    }

    async fn list_items(&self, config: &ProviderConfig) -> agentroot_core::Result<Vec<SourceItem>> {
        let item = self.fetch_item(&config.base_path).await?;
        Ok(vec![item])
    }

    async fn fetch_item(&self, uri: &str) -> agentroot_core::Result<SourceItem> {
        let content = self.fetch_url(uri).await?;
        let title = self.extract_title(&content, uri);
        let hash = hash_content(&content);

        Ok(
            SourceItem::new(uri.to_string(), title, content, hash, "url".to_string())
                .with_metadata("url".to_string(), uri.to_string()),
        )
    }
}

#[tokio::main]
async fn main() -> agentroot_core::Result<()> {
    println!("Agentroot URL Provider Example\n");

    let db_path = std::env::temp_dir().join("agentroot_url_example.db");
    println!("Opening database at: {}", db_path.display());
    let db = Database::open(&db_path)?;
    db.initialize()?;

    println!("Creating URL-based collection...");
    db.add_collection(
        "rust-changelog",
        "https://raw.githubusercontent.com/rust-lang/rust/master/RELEASES.md",
        "*.md",
        "url",
        None,
    )?;

    println!("\nFetching content from URL...");
    let provider = URLProvider::new();

    let url = "https://raw.githubusercontent.com/rust-lang/rust/master/RELEASES.md";
    match provider.fetch_item(url).await {
        Ok(item) => {
            println!("Successfully fetched:");
            println!("  URI: {}", item.uri);
            println!("  Title: {}", item.title);
            println!("  Content Length: {} bytes", item.content.len());
            println!("  Hash: {}", item.hash);

            let now = Utc::now().to_rfc3339();
            db.insert_content(&item.hash, &item.content)?;
            db.insert_document(
                "rust-changelog",
                &item.uri,
                &item.title,
                &item.hash,
                &now,
                &now,
                &item.source_type,
                item.metadata.get("url").map(|s| s.as_str()),
            )?;

            println!("\nDocument indexed successfully!");
        }
        Err(e) => {
            eprintln!("Error fetching from URL: {}", e);
            println!("\nNote: This example requires internet connection.");
        }
    }

    println!("\nExample: Fetching multiple URLs");
    let urls = vec![
        "https://raw.githubusercontent.com/rust-lang/rust/master/README.md",
        "https://raw.githubusercontent.com/rust-lang/rust/master/CONTRIBUTING.md",
    ];

    for url in urls {
        println!("\nFetching: {}", url);
        match provider.fetch_item(url).await {
            Ok(item) => {
                println!("  Title: {}", item.title);
                println!("  Size: {} bytes", item.content.len());
            }
            Err(e) => {
                eprintln!("  Error: {}", e);
            }
        }
    }

    println!("\nExample completed!");
    println!("\nKey Features Demonstrated:");
    println!("  - Custom SourceProvider implementation");
    println!("  - Async HTTP requests with reqwest");
    println!("  - Proper error handling with descriptive messages");
    println!("  - Title extraction from markdown content");
    println!("  - Integration with database for indexing");
    println!("  - Metadata storage for URL tracking");

    Ok(())
}
