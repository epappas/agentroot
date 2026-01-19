//! URL Provider for fetching content from web pages

use crate::db::hash_content;
use crate::error::{AgentRootError, Result};
use crate::providers::{ProviderConfig, SourceItem, SourceProvider};
use async_trait::async_trait;
use reqwest::{Client, StatusCode};
use std::time::Duration;

/// Provider for fetching content from URLs
pub struct URLProvider {
    client: Client,
}

impl Default for URLProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl URLProvider {
    /// Create a new URLProvider with default settings
    pub fn new() -> Self {
        let client = Client::builder()
            .user_agent(concat!("agentroot/", env!("CARGO_PKG_VERSION")))
            .timeout(Duration::from_secs(30))
            .redirect(reqwest::redirect::Policy::limited(10))
            .build()
            .unwrap_or_else(|_| Client::new());
        Self { client }
    }

    /// Create a URLProvider with custom client
    pub fn with_client(client: Client) -> Self {
        Self { client }
    }

    /// Fetch content from a URL with proper error handling
    async fn fetch_url(&self, url: &str) -> Result<String> {
        let response = self.client.get(url).send().await.map_err(|e| {
            if e.is_timeout() {
                AgentRootError::ExternalError(format!(
                    "Request timeout fetching {}: Server took too long to respond.",
                    url
                ))
            } else if e.is_connect() {
                AgentRootError::ExternalError(format!(
                    "Connection error fetching {}: Cannot reach server. Check your internet connection.",
                    url
                ))
            } else {
                AgentRootError::ExternalError(format!("Failed to fetch URL {}: {}", url, e))
            }
        })?;

        let status = response.status();
        if !status.is_success() {
            let error_msg = match status {
                StatusCode::NOT_FOUND => format!("URL not found (404): {}", url),
                StatusCode::FORBIDDEN => {
                    format!(
                        "Access forbidden (403): {}. Authentication may be required.",
                        url
                    )
                }
                StatusCode::UNAUTHORIZED => {
                    format!(
                        "Unauthorized (401): {}. Valid credentials are required.",
                        url
                    )
                }
                StatusCode::TOO_MANY_REQUESTS => {
                    format!("Rate limit exceeded (429): {}. Try again later.", url)
                }
                s if s.is_server_error() => {
                    format!(
                        "Server error ({}): {}. The server is experiencing issues.",
                        s.as_u16(),
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
            AgentRootError::ExternalError(format!(
                "Failed to read response body from {}: {}",
                url, e
            ))
        })
    }

    /// Extract title from content (looks for markdown # header or HTML title)
    fn extract_title(&self, content: &str, url: &str) -> String {
        if let Some(title) = content.lines().find(|line| line.trim().starts_with("# ")) {
            return title.trim_start_matches("# ").trim().to_string();
        }

        if let Some(start) = content.find("<title>") {
            if let Some(end) = content[start..].find("</title>") {
                let title = &content[start + 7..start + end];
                return title.trim().to_string();
            }
        }

        url.split('/')
            .filter(|s| !s.is_empty())
            .next_back()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "Untitled".to_string())
    }
}

#[async_trait]
impl SourceProvider for URLProvider {
    fn provider_type(&self) -> &'static str {
        "url"
    }

    async fn list_items(&self, config: &ProviderConfig) -> Result<Vec<SourceItem>> {
        let item = self.fetch_item(&config.base_path).await?;
        Ok(vec![item])
    }

    async fn fetch_item(&self, uri: &str) -> Result<SourceItem> {
        let content = self.fetch_url(uri).await?;
        let title = self.extract_title(&content, uri);
        let hash = hash_content(&content);

        let mut item = SourceItem::new(uri.to_string(), title, content, hash, "url".to_string());
        item.metadata.insert("url".to_string(), uri.to_string());

        Ok(item)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_type() {
        let provider = URLProvider::new();
        assert_eq!(provider.provider_type(), "url");
    }

    #[test]
    fn test_extract_title_from_markdown() {
        let provider = URLProvider::new();
        let content = "# Hello World\n\nSome content";
        let title = provider.extract_title(content, "https://example.com/test.md");
        assert_eq!(title, "Hello World");
    }

    #[test]
    fn test_extract_title_from_html() {
        let provider = URLProvider::new();
        let content = "<html><head><title>Test Page</title></head><body>Content</body></html>";
        let title = provider.extract_title(content, "https://example.com/test.html");
        assert_eq!(title, "Test Page");
    }

    #[test]
    fn test_extract_title_from_url() {
        let provider = URLProvider::new();
        let content = "Just some text";
        let title = provider.extract_title(content, "https://example.com/my-document.txt");
        assert_eq!(title, "my-document.txt");
    }

    #[test]
    fn test_extract_title_fallback() {
        let provider = URLProvider::new();
        let content = "Just some text";
        let title = provider.extract_title(content, "https://example.com/");
        assert_eq!(title, "example.com");
    }

    #[tokio::test]
    async fn test_fetch_invalid_url() {
        let provider = URLProvider::new();
        let result = provider
            .fetch_url("http://thisurldoesnotexist12345.invalid")
            .await;
        assert!(result.is_err());
    }
}
