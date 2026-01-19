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
    client: reqwest::blocking::Client,
}

impl GitHubProvider {
    /// Create new GitHub provider
    pub fn new() -> Self {
        let client = reqwest::blocking::Client::builder()
            .user_agent("agentroot/1.0")
            .build()
            .unwrap_or_else(|_| reqwest::blocking::Client::new());

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
            "Invalid GitHub URL: {}",
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

    /// Fetch file from GitHub
    fn fetch_file(
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

        let response = request.send()?;

        if !response.status().is_success() {
            return Err(AgentRootError::ExternalError(format!(
                "GitHub API error: {}",
                response.status()
            )));
        }

        Ok(response.text()?)
    }

    /// Fetch README from repository
    fn fetch_readme(
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

        let response = request.send()?;

        if !response.status().is_success() {
            return Err(AgentRootError::ExternalError(format!(
                "GitHub API error: {}",
                response.status()
            )));
        }

        let readme: ReadmeResponse = response.json()?;
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
    fn list_repo_files(
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

        let response = request.send()?;

        if !response.status().is_success() {
            return Err(AgentRootError::ExternalError(format!(
                "GitHub API error: {}",
                response.status()
            )));
        }

        let tree: TreeResponse = response.json()?;
        Ok(tree.tree)
    }
}

impl Default for GitHubProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl SourceProvider for GitHubProvider {
    fn provider_type(&self) -> &'static str {
        "github"
    }

    fn list_items(&self, config: &ProviderConfig) -> Result<Vec<SourceItem>> {
        let github_url = self.parse_github_url(&config.base_path)?;
        let token = self.get_token(config);

        match github_url {
            GitHubUrl::Repository { owner, repo } => {
                let files = self.list_repo_files(&owner, &repo, token.as_deref())?;
                let pattern = glob::Pattern::new(&config.pattern)?;

                let mut items = Vec::new();

                for file in files {
                    if file.file_type == "blob" && pattern.matches(&file.path) {
                        let url = format!(
                            "https://github.com/{}/{}/blob/HEAD/{}",
                            owner, repo, file.path
                        );
                        match self.fetch_item(&url) {
                            Ok(item) => items.push(item),
                            Err(_) => continue,
                        }
                    }
                }

                Ok(items)
            }
            GitHubUrl::File { .. } => {
                let item = self.fetch_item(&config.base_path)?;
                Ok(vec![item])
            }
        }
    }

    fn fetch_item(&self, uri: &str) -> Result<SourceItem> {
        let github_url = self.parse_github_url(uri)?;
        let token = std::env::var("GITHUB_TOKEN").ok();

        match github_url {
            GitHubUrl::Repository { owner, repo } => {
                let (filename, content) = self.fetch_readme(&owner, &repo, token.as_deref())?;
                let title = extract_title(&content, &filename);
                let hash = hash_content(&content);
                let uri = format!("{}/{}/{}", owner, repo, filename);

                Ok(
                    SourceItem::new(uri, title, content, hash, "github".to_string())
                        .with_metadata("owner".to_string(), owner.clone())
                        .with_metadata("repo".to_string(), repo.clone())
                        .with_metadata("file".to_string(), filename),
                )
            }
            GitHubUrl::File {
                owner,
                repo,
                branch,
                path,
            } => {
                let content = self.fetch_file(&owner, &repo, &branch, &path, token.as_deref())?;
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
}
