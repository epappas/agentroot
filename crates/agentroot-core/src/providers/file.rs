//! File system provider
//!
//! Provides content from local file system using glob patterns.

use super::{ProviderConfig, SourceItem, SourceProvider};
use crate::db::hash_content;
use crate::error::Result;
use crate::index::extract_title;
use glob::Pattern;
use std::path::Path;
use walkdir::{DirEntry, WalkDir};

/// Directories to exclude from scanning
const EXCLUDE_DIRS: &[&str] = &[
    "node_modules",
    ".git",
    ".cache",
    "vendor",
    "dist",
    "build",
    "__pycache__",
    ".venv",
    "target",
];

/// File system provider
pub struct FileProvider;

impl FileProvider {
    /// Create new file provider
    pub fn new() -> Self {
        Self
    }
}

impl Default for FileProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl SourceProvider for FileProvider {
    fn provider_type(&self) -> &'static str {
        "file"
    }

    fn list_items(&self, config: &ProviderConfig) -> Result<Vec<SourceItem>> {
        let root = Path::new(&config.base_path);
        let pattern = Pattern::new(&config.pattern)?;

        let exclude_hidden = config
            .get_option("exclude_hidden")
            .and_then(|v| v.parse::<bool>().ok())
            .unwrap_or(true);

        let follow_symlinks = config
            .get_option("follow_symlinks")
            .and_then(|v| v.parse::<bool>().ok())
            .unwrap_or(true);

        let exclude_dirs: Vec<String> = EXCLUDE_DIRS.iter().map(|s| s.to_string()).collect();

        let mut items = Vec::new();

        let walker = WalkDir::new(root)
            .follow_links(follow_symlinks)
            .into_iter()
            .filter_entry(|e| !should_skip(e, &exclude_dirs, exclude_hidden));

        for entry in walker {
            let entry = entry?;
            if !entry.file_type().is_file() {
                continue;
            }

            let path = entry.path();
            let relative = path
                .strip_prefix(root)
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| path.to_string_lossy().to_string());

            if pattern.matches(&relative) {
                let content = std::fs::read_to_string(path)?;
                let title = extract_title(&content, &relative);
                let hash = hash_content(&content);

                items.push(
                    SourceItem::new(relative, title, content, hash, "file".to_string())
                        .with_metadata("absolute_path".to_string(), path.display().to_string()),
                );
            }
        }

        Ok(items)
    }

    fn fetch_item(&self, uri: &str) -> Result<SourceItem> {
        let path = Path::new(uri);
        let content = std::fs::read_to_string(path)?;
        let title = extract_title(&content, uri);
        let hash = hash_content(&content);

        Ok(
            SourceItem::new(uri.to_string(), title, content, hash, "file".to_string())
                .with_metadata("absolute_path".to_string(), path.display().to_string()),
        )
    }
}

fn should_skip(entry: &DirEntry, exclude_dirs: &[String], exclude_hidden: bool) -> bool {
    let name = entry.file_name().to_string_lossy();

    if exclude_hidden && name.starts_with('.') {
        return true;
    }

    if entry.file_type().is_dir() && exclude_dirs.iter().any(|d| name == *d) {
        return true;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_file_provider_type() {
        let provider = FileProvider::new();
        assert_eq!(provider.provider_type(), "file");
    }

    #[test]
    fn test_file_provider_list_items() {
        let temp = TempDir::new().unwrap();
        let base = temp.path();

        fs::write(base.join("test1.md"), "# Test 1").unwrap();
        fs::write(base.join("test2.md"), "# Test 2").unwrap();
        fs::write(base.join("ignore.txt"), "ignore").unwrap();

        let config = ProviderConfig::new(base.to_string_lossy().to_string(), "**/*.md".to_string())
            .with_option("exclude_hidden".to_string(), "false".to_string());
        let provider = FileProvider::new();
        let items = provider.list_items(&config).unwrap();

        assert_eq!(items.len(), 2);
        assert!(items.iter().any(|i| i.uri == "test1.md"));
        assert!(items.iter().any(|i| i.uri == "test2.md"));
    }

    #[test]
    fn test_file_provider_fetch_item() {
        let temp = TempDir::new().unwrap();
        let base = temp.path();
        let file = base.join("test.md");

        fs::write(&file, "# Test Content").unwrap();

        let provider = FileProvider::new();
        let item = provider.fetch_item(file.to_str().unwrap()).unwrap();

        assert_eq!(item.content, "# Test Content");
        assert_eq!(item.title, "Test Content");
        assert_eq!(item.source_type, "file");
    }
}
