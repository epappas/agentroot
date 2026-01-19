//! GitHub provider
//!
//! Provides content from GitHub repositories, files, and gists.
//! Supports both public and private repositories with authentication.

use super::{ProviderConfig, SourceItem, SourceProvider};
use crate::db::hash_content;
use crate::error::{AgentRootError, Result};
use crate::index::extract_title;
use base64::Engine;
use serde::Deserialize;

/// GitHub provider
pub struct GitHubProvider {
    client: reqwest::Client,
}

const MAX_RETRIES: u32 = 3;
const INITIAL_BACKOFF_MS: u64 = 1000;

impl GitHubProvider {
    /// Create new GitHub provider
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .user_agent("agentroot/1.0")
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self { client }
    }

    /// Parse GitHub URL into components
    fn parse_github_url(&self, url: &str) -> Result<GitHubUrl> {
        let url = url.trim();

        if url.starts_with("https://github.com/") || url.starts_with("http://github.com/") {
            let parts: Vec<&str> = url
                .trim_start_matches("https://github.com/")
                .trim_start_matches("http://github.com/")
                .split('/')
                .collect();

            if parts.len() >= 2 {
                let owner = parts[0].to_string();
                let repo = parts[1].to_string();

                if parts.len() == 2 {
                    return Ok(GitHubUrl::Repository { owner, repo });
                }

                if parts.len() >= 5 && parts[2] == "blob" {
                    let branch = parts[3].to_string();
                    let path = parts[4..].join("/");
                    return Ok(GitHubUrl::File {
                        owner,
                        repo,
                        branch,
                        path,
                    });
                }
            }
        }

        Err(AgentRootError::InvalidInput(format!(
            "Invalid GitHub URL: {}. \
             Expected format: https://github.com/owner/repo or https://github.com/owner/repo/blob/branch/path",
            url
        )))
    }

    /// Get GitHub API token from environment
    fn get_token(&self, config: &ProviderConfig) -> Option<String> {
        config
            .get_option("github_token")
            .cloned()
            .or_else(|| std::env::var("GITHUB_TOKEN").ok())
    }

    /// Check rate limit from response headers and log warnings
    fn check_rate_limit(&self, response: &reqwest::Response) {
        if let Some(remaining) = response.headers().get("x-ratelimit-remaining") {
            if let Ok(remaining_str) = remaining.to_str() {
                if let Ok(remaining_count) = remaining_str.parse::<i32>() {
                    if remaining_count < 10 {
                        eprintln!(
                            "Warning: GitHub API rate limit low ({} requests remaining). \
                             Set GITHUB_TOKEN to increase limits.",
                            remaining_count
                        );
                    }
                }
            }
        }
    }

    /// Send request with retry logic for rate limits
    async fn send_with_retry(&self, request: reqwest::RequestBuilder) -> Result<reqwest::Response> {
        let mut retries = 0;
        let mut backoff_ms = INITIAL_BACKOFF_MS;

        loop {
            let req = request.try_clone().ok_or_else(|| {
                AgentRootError::ExternalError("Failed to clone request".to_string())
            })?;

            match req.send().await {
                Ok(response) => {
                    self.check_rate_limit(&response);

                    if response.status() == 429 && retries < MAX_RETRIES {
                        let retry_after = response
                            .headers()
                            .get("retry-after")
                            .and_then(|v| v.to_str().ok())
                            .and_then(|v| v.parse::<u64>().ok())
                            .unwrap_or(backoff_ms / 1000);

                        eprintln!(
                            "Rate limit exceeded. Retrying after {} seconds (attempt {}/{})",
                            retry_after,
                            retries + 1,
                            MAX_RETRIES
                        );

                        tokio::time::sleep(tokio::time::Duration::from_secs(retry_after)).await;
                        retries += 1;
                        backoff_ms *= 2;
                        continue;
                    }

                    return Ok(response);
                }
                Err(e) if retries < MAX_RETRIES && e.is_timeout() => {
                    eprintln!(
                        "Request timeout. Retrying in {} seconds (attempt {}/{})",
                        backoff_ms / 1000,
                        retries + 1,
                        MAX_RETRIES
                    );
                    tokio::time::sleep(tokio::time::Duration::from_millis(backoff_ms)).await;
                    retries += 1;
                    backoff_ms *= 2;
                }
                Err(e) => return Err(e.into()),
            }
        }
    }

    /// Fetch file from GitHub
    async fn fetch_file(
        &self,
        owner: &str,
        repo: &str,
        branch: &str,
        path: &str,
        token: Option<&str>,
    ) -> Result<String> {
        let raw_url = format!(
            "https://raw.githubusercontent.com/{}/{}/{}/{}",
            owner, repo, branch, path
        );

        let mut request = self.client.get(&raw_url);

        if let Some(token) = token {
            request = request.header("Authorization", format!("token {}", token));
        }

        let response = self.send_with_retry(request).await.map_err(|e| {
            AgentRootError::ExternalError(format!(
                "Failed to fetch file from GitHub: {}. Check your internet connection.",
                e
            ))
        })?;

        let status = response.status();
        if !status.is_success() {
            let error_msg = match status.as_u16() {
                404 => format!(
                    "File not found: {}/{}/{}/{}. Verify the repository, branch, and file path are correct.",
                    owner, repo, branch, path
                ),
                403 => {
                    "GitHub API rate limit exceeded or access forbidden. \
                     Set GITHUB_TOKEN environment variable with a personal access token to increase rate limits. \
                     Get token from: https://github.com/settings/tokens".to_string()
                }
                401 => {
                    "Authentication failed. Your GITHUB_TOKEN may be invalid or expired. \
                     Generate a new token at: https://github.com/settings/tokens".to_string()
                }
                _ => format!("GitHub API error {}: {}", status.as_u16(), status.canonical_reason().unwrap_or("Unknown error")),
            };
            return Err(AgentRootError::ExternalError(error_msg));
        }

        response.text().await.map_err(|e| {
            AgentRootError::ExternalError(format!("Failed to read file content: {}", e))
        })
    }

    /// Fetch README from repository
    async fn fetch_readme(
        &self,
        owner: &str,
        repo: &str,
        token: Option<&str>,
    ) -> Result<(String, String)> {
        let api_url = format!("https://api.github.com/repos/{}/{}/readme", owner, repo);

        let mut request = self.client.get(&api_url);

        if let Some(token) = token {
            request = request.header("Authorization", format!("token {}", token));
        }

        request = request.header("Accept", "application/vnd.github.v3+json");

        let response = self.send_with_retry(request).await.map_err(|e| {
            AgentRootError::ExternalError(format!(
                "Failed to fetch README from GitHub: {}. Check your internet connection.",
                e
            ))
        })?;

        let status = response.status();
        if !status.is_success() {
            let error_msg = match status.as_u16() {
                404 => format!(
                    "README not found for repository {}/{}. The repository may not have a README file, or it may not exist.",
                    owner, repo
                ),
                403 => {
                    "GitHub API rate limit exceeded or repository access forbidden. \
                     For public repositories, set GITHUB_TOKEN environment variable to increase rate limits. \
                     For private repositories, ensure your token has 'repo' scope. \
                     Get token from: https://github.com/settings/tokens".to_string()
                }
                401 => {
                    "Authentication failed. Your GITHUB_TOKEN may be invalid or expired. \
                     Generate a new token at: https://github.com/settings/tokens".to_string()
                }
                _ => format!("GitHub API error {}: {}", status.as_u16(), status.canonical_reason().unwrap_or("Unknown error")),
            };
            return Err(AgentRootError::ExternalError(error_msg));
        }

        let readme: ReadmeResponse = response.json().await.map_err(|e| {
            AgentRootError::ExternalError(format!("Failed to parse README response: {}", e))
        })?;
        let content = String::from_utf8(
            base64::engine::general_purpose::STANDARD
                .decode(readme.content.replace('\n', ""))
                .map_err(|e| {
                    AgentRootError::ExternalError(format!("Base64 decode error: {}", e))
                })?,
        )
        .map_err(|e| AgentRootError::ExternalError(format!("UTF-8 decode error: {}", e)))?;

        Ok((readme.name, content))
    }

    /// List files in repository
    async fn list_repo_files(
        &self,
        owner: &str,
        repo: &str,
        token: Option<&str>,
    ) -> Result<Vec<RepoFile>> {
        let api_url = format!(
            "https://api.github.com/repos/{}/{}/git/trees/HEAD?recursive=1",
            owner, repo
        );

        let mut request = self.client.get(&api_url);

        if let Some(token) = token {
            request = request.header("Authorization", format!("token {}", token));
        }

        request = request.header("Accept", "application/vnd.github.v3+json");

        let response = self.send_with_retry(request).await.map_err(|e| {
            AgentRootError::ExternalError(format!(
                "Failed to list files from GitHub repository: {}. Check your internet connection.",
                e
            ))
        })?;

        let status = response.status();
        if !status.is_success() {
            let error_msg = match status.as_u16() {
                404 => format!(
                    "Repository not found: {}/{}. Verify the repository owner and name are correct.",
                    owner, repo
                ),
                403 => {
                    "GitHub API rate limit exceeded or repository access forbidden. \
                     For public repositories, set GITHUB_TOKEN environment variable to increase rate limits. \
                     For private repositories, ensure your token has 'repo' scope. \
                     Get token from: https://github.com/settings/tokens".to_string()
                }
                401 => {
                    "Authentication failed. Your GITHUB_TOKEN may be invalid or expired. \
                     Generate a new token at: https://github.com/settings/tokens".to_string()
                }
                409 => format!(
                    "Repository {}/{} is empty or has no commits yet.",
                    owner, repo
                ),
                _ => format!("GitHub API error {}: {}", status.as_u16(), status.canonical_reason().unwrap_or("Unknown error")),
            };
            return Err(AgentRootError::ExternalError(error_msg));
        }

        let tree: TreeResponse = response.json().await.map_err(|e| {
            AgentRootError::ExternalError(format!("Failed to parse repository file tree: {}", e))
        })?;
        Ok(tree.tree)
    }
}

impl Default for GitHubProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl SourceProvider for GitHubProvider {
    fn provider_type(&self) -> &'static str {
        "github"
    }

    async fn list_items(&self, config: &ProviderConfig) -> Result<Vec<SourceItem>> {
        let github_url = self.parse_github_url(&config.base_path)?;
        let token = self.get_token(config);

        match github_url {
            GitHubUrl::Repository { owner, repo } => {
                let files = self
                    .list_repo_files(&owner, &repo, token.as_deref())
                    .await?;
                let pattern = glob::Pattern::new(&config.pattern)?;

                let mut items = Vec::new();

                for file in files {
                    if file.file_type == "blob" && pattern.matches(&file.path) {
                        let url = format!(
                            "https://github.com/{}/{}/blob/HEAD/{}",
                            owner, repo, file.path
                        );
                        match self.fetch_item(&url).await {
                            Ok(item) => items.push(item),
                            Err(_) => continue,
                        }
                    }
                }

                Ok(items)
            }
            GitHubUrl::File { .. } => {
                let item = self.fetch_item(&config.base_path).await?;
                Ok(vec![item])
            }
        }
    }

    async fn fetch_item(&self, uri: &str) -> Result<SourceItem> {
        let github_url = self.parse_github_url(uri)?;
        let token = std::env::var("GITHUB_TOKEN").ok();

        match github_url {
            GitHubUrl::Repository { owner, repo } => {
                let (filename, content) =
                    self.fetch_readme(&owner, &repo, token.as_deref()).await?;
                let title = extract_title(&content, &filename);
                let hash = hash_content(&content);
                let uri = format!("{}/{}/{}", owner, repo, filename);

                Ok(
                    SourceItem::new(uri, title, content, hash, "github".to_string())
                        .with_metadata("owner".to_string(), owner)
                        .with_metadata("repo".to_string(), repo)
                        .with_metadata("file".to_string(), filename),
                )
            }
            GitHubUrl::File {
                owner,
                repo,
                branch,
                path,
            } => {
                let content = self
                    .fetch_file(&owner, &repo, &branch, &path, token.as_deref())
                    .await?;
                let title = extract_title(&content, &path);
                let hash = hash_content(&content);
                let uri = format!("{}/{}/{}", owner, repo, path);

                Ok(
                    SourceItem::new(uri, title, content, hash, "github".to_string())
                        .with_metadata("owner".to_string(), owner)
                        .with_metadata("repo".to_string(), repo)
                        .with_metadata("branch".to_string(), branch)
                        .with_metadata("path".to_string(), path),
                )
            }
        }
    }
}

/// GitHub URL type
#[derive(Debug, Clone)]
enum GitHubUrl {
    Repository {
        owner: String,
        repo: String,
    },
    File {
        owner: String,
        repo: String,
        branch: String,
        path: String,
    },
}

/// GitHub API response for README
#[derive(Debug, Deserialize)]
struct ReadmeResponse {
    name: String,
    content: String,
}

/// GitHub API response for tree
#[derive(Debug, Deserialize)]
struct TreeResponse {
    tree: Vec<RepoFile>,
}

/// Repository file from tree API
#[derive(Debug, Deserialize)]
struct RepoFile {
    path: String,
    #[serde(rename = "type")]
    file_type: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_github_provider_type() {
        let provider = GitHubProvider::new();
        assert_eq!(provider.provider_type(), "github");
    }

    #[test]
    fn test_parse_github_repo_url() {
        let provider = GitHubProvider::new();
        let url = "https://github.com/rust-lang/rust";
        let parsed = provider.parse_github_url(url).unwrap();

        match parsed {
            GitHubUrl::Repository { owner, repo } => {
                assert_eq!(owner, "rust-lang");
                assert_eq!(repo, "rust");
            }
            _ => panic!("Expected Repository variant"),
        }
    }

    #[test]
    fn test_parse_github_file_url() {
        let provider = GitHubProvider::new();
        let url = "https://github.com/rust-lang/rust/blob/master/README.md";
        let parsed = provider.parse_github_url(url).unwrap();

        match parsed {
            GitHubUrl::File {
                owner,
                repo,
                branch,
                path,
            } => {
                assert_eq!(owner, "rust-lang");
                assert_eq!(repo, "rust");
                assert_eq!(branch, "master");
                assert_eq!(path, "README.md");
            }
            _ => panic!("Expected File variant"),
        }
    }

    #[test]
    fn test_parse_invalid_url() {
        let provider = GitHubProvider::new();
        let url = "https://example.com/not-github";
        let result = provider.parse_github_url(url);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_github_url_variants() {
        let provider = GitHubProvider::new();

        let test_cases = vec![
            ("https://github.com/rust-lang/rust", true),
            ("http://github.com/rust-lang/rust", true),
            ("https://github.com/user/repo/blob/main/README.md", true),
            (
                "https://github.com/user/repo/blob/feature-branch/src/main.rs",
                true,
            ),
            ("https://gitlab.com/user/repo", false),
            ("github.com/user/repo", false),
            ("https://github.com/", false),
            ("https://github.com/user", false),
        ];

        for (url, should_succeed) in test_cases {
            let result = provider.parse_github_url(url);
            assert_eq!(
                result.is_ok(),
                should_succeed,
                "URL: {} - Expected success: {}, Got: {:?}",
                url,
                should_succeed,
                result
            );
        }
    }

    #[test]
    fn test_parse_github_file_url_components() {
        let provider = GitHubProvider::new();
        let url = "https://github.com/rust-lang/rust/blob/master/src/main.rs";
        let result = provider.parse_github_url(url).unwrap();

        match result {
            GitHubUrl::File {
                owner,
                repo,
                branch,
                path,
            } => {
                assert_eq!(owner, "rust-lang");
                assert_eq!(repo, "rust");
                assert_eq!(branch, "master");
                assert_eq!(path, "src/main.rs");
            }
            _ => panic!("Expected File variant"),
        }
    }

    #[test]
    fn test_parse_github_file_url_nested_path() {
        let provider = GitHubProvider::new();
        let url = "https://github.com/owner/repo/blob/main/deep/nested/path/file.md";
        let result = provider.parse_github_url(url).unwrap();

        match result {
            GitHubUrl::File { path, .. } => {
                assert_eq!(path, "deep/nested/path/file.md");
            }
            _ => panic!("Expected File variant"),
        }
    }

    #[test]
    fn test_get_token_from_config() {
        let provider = GitHubProvider::new();

        let config = ProviderConfig::new(
            "https://github.com/user/repo".to_string(),
            "*.md".to_string(),
        )
        .with_option("github_token".to_string(), "ghp_test123".to_string());

        let token = provider.get_token(&config);
        assert_eq!(token, Some("ghp_test123".to_string()));
    }

    #[test]
    fn test_get_token_priority() {
        let provider = GitHubProvider::new();

        let config_with_token = ProviderConfig::new(
            "https://github.com/user/repo".to_string(),
            "*.md".to_string(),
        )
        .with_option("github_token".to_string(), "ghp_config".to_string());

        let token = provider.get_token(&config_with_token);
        assert_eq!(token, Some("ghp_config".to_string()));
    }

    #[test]
    fn test_provider_type() {
        let provider = GitHubProvider::new();
        assert_eq!(provider.provider_type(), "github");
    }

    #[test]
    fn test_parse_github_url_edge_cases() {
        let provider = GitHubProvider::new();

        let edge_cases = vec![
            "https://github.com/user/repo-with-dashes",
            "https://github.com/user/repo_with_underscores",
            "https://github.com/user/repo.with.dots",
            "https://github.com/user-with-dash/repo",
            "https://github.com/user_with_underscore/repo",
        ];

        for url in edge_cases {
            let result = provider.parse_github_url(url);
            assert!(result.is_ok(), "Failed to parse valid URL: {}", url);
        }
    }
}
