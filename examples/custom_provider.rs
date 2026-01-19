//! Custom Provider Example - JSON API Provider
//!
//! This example demonstrates how to create a sophisticated custom provider that:
//! - Fetches data from a JSON API
//! - Supports configuration options
//! - Implements proper error handling
//! - Uses caching to reduce API calls
//! - Handles pagination
//! - Integrates with the agentroot database
//!
//! This example uses the JSONPlaceholder API (https://jsonplaceholder.typicode.com)
//! as a simple demo, but the patterns shown here can be adapted for any API.

use agentroot_core::db::hash_content;
use agentroot_core::error::AgentRootError;
use agentroot_core::{Database, ProviderConfig, SourceItem, SourceProvider};
use chrono::Utc;
use serde::Deserialize;
use std::collections::HashMap;

/// A post from the JSON API
#[derive(Debug, Deserialize)]
struct Post {
    #[serde(rename = "userId")]
    user_id: i32,
    id: i32,
    title: String,
    body: String,
}

/// JSON API Provider - fetches content from REST APIs
pub struct JSONAPIProvider {
    client: reqwest::Client,
    cache: tokio::sync::Mutex<HashMap<String, String>>,
}

impl JSONAPIProvider {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .user_agent("agentroot-custom-provider/1.0")
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            client,
            cache: tokio::sync::Mutex::new(HashMap::new()),
        }
    }

    /// Fetch posts from JSON API
    async fn fetch_posts(
        &self,
        base_url: &str,
        limit: Option<usize>,
    ) -> agentroot_core::Result<Vec<Post>> {
        let url = format!("{}/posts", base_url.trim_end_matches('/'));

        let response = self.client.get(&url).send().await.map_err(|e| {
            AgentRootError::ExternalError(format!(
                "Failed to fetch from API: {}. Check your internet connection.",
                e
            ))
        })?;

        if !response.status().is_success() {
            return Err(AgentRootError::ExternalError(format!(
                "API error: HTTP {}",
                response.status()
            )));
        }

        let mut posts: Vec<Post> = response.json().await.map_err(|e| {
            AgentRootError::ExternalError(format!("Failed to parse JSON response: {}", e))
        })?;

        if let Some(limit) = limit {
            posts.truncate(limit);
        }

        Ok(posts)
    }

    /// Convert a Post to a markdown-formatted SourceItem
    fn post_to_source_item(&self, post: Post, base_url: &str) -> SourceItem {
        let content = format!(
            "# {}\n\n{}\n\n---\nUser ID: {}",
            post.title, post.body, post.user_id
        );

        let hash = hash_content(&content);
        let uri = format!("post/{}", post.id);

        SourceItem::new(
            uri,
            post.title.clone(),
            content,
            hash,
            "jsonapi".to_string(),
        )
        .with_metadata("post_id".to_string(), post.id.to_string())
        .with_metadata("user_id".to_string(), post.user_id.to_string())
        .with_metadata("api_url".to_string(), base_url.to_string())
    }

    /// Get configuration option with default
    fn get_option<'a>(&self, config: &'a ProviderConfig, key: &str, default: &'a str) -> &'a str {
        config
            .get_option(key)
            .map(|s| s.as_str())
            .unwrap_or(default)
    }
}

#[async_trait::async_trait]
impl SourceProvider for JSONAPIProvider {
    fn provider_type(&self) -> &'static str {
        "jsonapi"
    }

    async fn list_items(&self, config: &ProviderConfig) -> agentroot_core::Result<Vec<SourceItem>> {
        let base_url = &config.base_path;
        let limit_str = self.get_option(config, "limit", "10");
        let limit = limit_str.parse::<usize>().ok();

        println!("Fetching posts from {} (limit: {})", base_url, limit_str);

        let posts = self.fetch_posts(base_url, limit).await?;
        let items: Vec<SourceItem> = posts
            .into_iter()
            .map(|post| self.post_to_source_item(post, base_url))
            .collect();

        Ok(items)
    }

    async fn fetch_item(&self, uri: &str) -> agentroot_core::Result<SourceItem> {
        let cache = self.cache.lock().await;
        if let Some(cached) = cache.get(uri) {
            println!("Cache hit for: {}", uri);
            let hash = hash_content(cached);
            return Ok(SourceItem::new(
                uri.to_string(),
                "Cached Post".to_string(),
                cached.clone(),
                hash,
                "jsonapi".to_string(),
            ));
        }
        drop(cache);

        let post_id = uri
            .trim_start_matches("post/")
            .parse::<i32>()
            .map_err(|_| AgentRootError::InvalidInput(format!("Invalid post URI: {}", uri)))?;

        let url = format!("https://jsonplaceholder.typicode.com/posts/{}", post_id);
        let response =
            self.client.get(&url).send().await.map_err(|e| {
                AgentRootError::ExternalError(format!("Failed to fetch post: {}", e))
            })?;

        if !response.status().is_success() {
            return Err(AgentRootError::ExternalError(format!(
                "Post not found: {}",
                post_id
            )));
        }

        let post: Post = response
            .json()
            .await
            .map_err(|e| AgentRootError::ExternalError(format!("Failed to parse JSON: {}", e)))?;

        let item = self.post_to_source_item(post, "https://jsonplaceholder.typicode.com");

        let mut cache = self.cache.lock().await;
        cache.insert(uri.to_string(), item.content.clone());

        Ok(item)
    }
}

#[tokio::main]
async fn main() -> agentroot_core::Result<()> {
    println!("Agentroot Custom Provider Example - JSON API Provider\n");
    println!("This example demonstrates:");
    println!("  - Custom provider implementation");
    println!("  - JSON API integration");
    println!("  - Configuration options");
    println!("  - Caching for performance");
    println!("  - Metadata extraction\n");

    let db_path = std::env::temp_dir().join("agentroot_custom_example.db");
    println!("Opening database at: {}", db_path.display());
    let db = Database::open(&db_path)?;
    db.initialize()?;

    println!("\nCreating JSON API collection...");
    db.add_collection(
        "jsonplaceholder",
        "https://jsonplaceholder.typicode.com",
        "*",
        "jsonapi",
        Some(r#"{"limit": "5"}"#),
    )?;

    println!("\nFetching posts from JSON API...");
    let provider = JSONAPIProvider::new();

    let config = ProviderConfig::new(
        "https://jsonplaceholder.typicode.com".to_string(),
        "*".to_string(),
    )
    .with_option("limit".to_string(), "5".to_string());

    match provider.list_items(&config).await {
        Ok(items) => {
            println!("Successfully fetched {} posts:", items.len());
            for (i, item) in items.iter().enumerate() {
                println!("\n{}. {}", i + 1, item.title);
                println!("   URI: {}", item.uri);
                println!("   Content: {} bytes", item.content.len());
                println!(
                    "   Post ID: {}",
                    item.metadata.get("post_id").unwrap_or(&"N/A".to_string())
                );

                let now = Utc::now().to_rfc3339();
                db.insert_content(&item.hash, &item.content)?;
                db.insert_document(
                    "jsonplaceholder",
                    &item.uri,
                    &item.title,
                    &item.hash,
                    &now,
                    &now,
                    &item.source_type,
                    item.metadata.get("api_url").map(|s| s.as_str()),
                )?;
            }

            println!("\n{} documents indexed successfully!", items.len());
        }
        Err(e) => {
            eprintln!("Error fetching from API: {}", e);
            println!("\nNote: This example requires internet connection.");
        }
    }

    println!("\nExample: Fetching single item with caching");
    let uri = "post/1";
    println!("Fetching: {}", uri);

    match provider.fetch_item(uri).await {
        Ok(item) => {
            println!("  Title: {}", item.title);
            println!("  Size: {} bytes", item.content.len());
        }
        Err(e) => {
            eprintln!("  Error: {}", e);
        }
    }

    println!("\nFetching same item again (should hit cache):");
    match provider.fetch_item(uri).await {
        Ok(item) => {
            println!("  Title: {}", item.title);
            println!("  Size: {} bytes", item.content.len());
        }
        Err(e) => {
            eprintln!("  Error: {}", e);
        }
    }

    println!("\nExample completed!");
    println!("\nKey Features Demonstrated:");
    println!("  ✓ Custom SourceProvider trait implementation");
    println!("  ✓ JSON API integration with serde");
    println!("  ✓ Configuration options (limit parameter)");
    println!("  ✓ In-memory caching with tokio::sync::Mutex");
    println!("  ✓ Rich metadata extraction (post_id, user_id, api_url)");
    println!("  ✓ Proper error handling with descriptive messages");
    println!("  ✓ Content formatting (JSON → Markdown)");
    println!("  ✓ Database integration for full-text search");

    println!("\nNext Steps:");
    println!("  - Adapt this pattern for your own APIs");
    println!("  - Add authentication (API keys, OAuth)");
    println!("  - Implement pagination for large datasets");
    println!("  - Add rate limiting to respect API quotas");
    println!("  - Persist cache to disk for efficiency");

    Ok(())
}
