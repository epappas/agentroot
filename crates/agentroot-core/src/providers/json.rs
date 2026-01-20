//! JSON Provider for indexing JSON files with semantic object/array splitting

use crate::db::hash_content;
use crate::error::{AgentRootError, Result};
use crate::providers::{ProviderConfig, SourceItem, SourceProvider};
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Provider for indexing JSON files
pub struct JSONProvider;

impl Default for JSONProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl JSONProvider {
    /// Create a new JSONProvider
    pub fn new() -> Self {
        Self
    }

    /// Parse JSON file and return objects/arrays as items
    fn parse_json_file(&self, path: &Path, config: &ProviderConfig) -> Result<Vec<SourceItem>> {
        let file_content = fs::read_to_string(path).map_err(|e| {
            AgentRootError::Io(std::io::Error::new(
                e.kind(),
                format!("Failed to read JSON file {:?}: {}", path, e),
            ))
        })?;

        let json_value: Value = serde_json::from_str(&file_content).map_err(|e| {
            AgentRootError::Parse(format!("Failed to parse JSON file {:?}: {}", path, e))
        })?;

        let filename = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown.json");

        let index_mode = config
            .options
            .get("index_mode")
            .map(|s| s.as_str())
            .unwrap_or("array");

        match index_mode {
            "array" => self.index_as_array(&json_value, filename, path),
            "object" => self.index_as_object(&json_value, filename, path),
            "full" => Ok(vec![self.index_full_document(&json_value, filename, path)]),
            _ => Err(AgentRootError::Parse(format!(
                "Invalid index_mode: {}. Expected: array, object, or full",
                index_mode
            ))),
        }
    }

    /// Index JSON as array (each top-level array item becomes a document)
    fn index_as_array(
        &self,
        json_value: &Value,
        filename: &str,
        path: &Path,
    ) -> Result<Vec<SourceItem>> {
        match json_value {
            Value::Array(arr) => {
                let mut items = Vec::new();
                for (idx, item) in arr.iter().enumerate() {
                    let content = serde_json::to_string_pretty(item)?;
                    let title = self.extract_title(item, filename, idx);
                    let uri = format!("json://{}/item_{}", path.display(), idx);
                    let hash = hash_content(&content);

                    let mut metadata = HashMap::new();
                    metadata.insert("file".to_string(), filename.to_string());
                    metadata.insert("index".to_string(), idx.to_string());
                    metadata.insert(
                        "item_type".to_string(),
                        self.json_type_name(item).to_string(),
                    );

                    if let Value::Object(obj) = item {
                        for (key, value) in obj {
                            if let Some(str_val) = value.as_str() {
                                metadata.insert(key.clone(), str_val.to_string());
                            }
                        }
                    }

                    items.push(SourceItem {
                        uri,
                        title,
                        content,
                        hash,
                        source_type: "json".to_string(),
                        metadata,
                    });
                }
                Ok(items)
            }
            _ => Err(AgentRootError::Parse(format!(
                "JSON file {:?} is not an array. Use index_mode=object or index_mode=full",
                path
            ))),
        }
    }

    /// Index JSON as object (each top-level key becomes a document)
    fn index_as_object(
        &self,
        json_value: &Value,
        filename: &str,
        path: &Path,
    ) -> Result<Vec<SourceItem>> {
        match json_value {
            Value::Object(obj) => {
                let mut items = Vec::new();
                for (idx, (key, value)) in obj.iter().enumerate() {
                    let content = serde_json::to_string_pretty(value)?;
                    let title = format!("{} - {}", filename, key);
                    let uri = format!("json://{}/key_{}", path.display(), key);
                    let hash = hash_content(&content);

                    let mut metadata = HashMap::new();
                    metadata.insert("file".to_string(), filename.to_string());
                    metadata.insert("key".to_string(), key.clone());
                    metadata.insert("index".to_string(), idx.to_string());
                    metadata.insert(
                        "value_type".to_string(),
                        self.json_type_name(value).to_string(),
                    );

                    items.push(SourceItem {
                        uri,
                        title,
                        content,
                        hash,
                        source_type: "json".to_string(),
                        metadata,
                    });
                }
                Ok(items)
            }
            _ => Err(AgentRootError::Parse(format!(
                "JSON file {:?} is not an object. Use index_mode=array or index_mode=full",
                path
            ))),
        }
    }

    /// Index full JSON document as single item
    fn index_full_document(&self, json_value: &Value, filename: &str, path: &Path) -> SourceItem {
        let content = serde_json::to_string_pretty(json_value).unwrap_or_default();
        let title = filename.to_string();
        let uri = format!("json://{}", path.display());
        let hash = hash_content(&content);

        let mut metadata = HashMap::new();
        metadata.insert("file".to_string(), filename.to_string());
        metadata.insert(
            "type".to_string(),
            self.json_type_name(json_value).to_string(),
        );

        SourceItem {
            uri,
            title,
            content,
            hash,
            source_type: "json".to_string(),
            metadata,
        }
    }

    /// Extract meaningful title from JSON value
    fn extract_title(&self, value: &Value, filename: &str, idx: usize) -> String {
        if let Value::Object(obj) = value {
            if let Some(title) = obj.get("title").and_then(|v| v.as_str()) {
                return title.to_string();
            }
            if let Some(name) = obj.get("name").and_then(|v| v.as_str()) {
                return name.to_string();
            }
            if let Some(id) = obj.get("id") {
                return format!("{} - ID {}", filename, id);
            }
        }

        format!("{} - Item {}", filename, idx)
    }

    /// Get JSON value type name
    fn json_type_name(&self, value: &Value) -> &'static str {
        match value {
            Value::Null => "null",
            Value::Bool(_) => "boolean",
            Value::Number(_) => "number",
            Value::String(_) => "string",
            Value::Array(_) => "array",
            Value::Object(_) => "object",
        }
    }

    /// Scan directory for JSON files matching pattern
    fn scan_directory(&self, base_path: &Path, pattern: &str) -> Result<Vec<PathBuf>> {
        let glob_pattern = glob::Pattern::new(pattern)?;
        let mut json_files = Vec::new();

        for entry in WalkDir::new(base_path)
            .follow_links(true)
            .into_iter()
            .filter_entry(|e| {
                let name = e.file_name().to_string_lossy();
                !name.starts_with('.')
                    && !matches!(
                        name.as_ref(),
                        "node_modules" | ".git" | ".cache" | "target" | "dist" | "build"
                    )
            })
        {
            let entry = entry?;
            if !entry.file_type().is_file() {
                continue;
            }

            let path = entry.path();
            if let Some(ext) = path.extension() {
                if ext.eq_ignore_ascii_case("json") {
                    if let Ok(relative) = path.strip_prefix(base_path) {
                        let relative_str = relative.to_string_lossy();
                        if glob_pattern.matches(&relative_str) {
                            json_files.push(path.to_path_buf());
                        }
                    }
                }
            }
        }

        Ok(json_files)
    }
}

#[async_trait]
impl SourceProvider for JSONProvider {
    fn provider_type(&self) -> &'static str {
        "json"
    }

    async fn list_items(&self, config: &ProviderConfig) -> Result<Vec<SourceItem>> {
        let base_path = Path::new(&config.base_path);

        if base_path.is_file() {
            if base_path
                .extension()
                .map(|e| e.eq_ignore_ascii_case("json"))
                .unwrap_or(false)
            {
                return self.parse_json_file(base_path, config);
            } else {
                return Err(AgentRootError::Parse(format!(
                    "File {:?} is not a JSON file",
                    base_path
                )));
            }
        }

        if !base_path.exists() {
            return Err(AgentRootError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Path not found: {:?}", base_path),
            )));
        }

        let json_files = self.scan_directory(base_path, &config.pattern)?;
        let mut all_items = Vec::new();

        for json_file in json_files {
            match self.parse_json_file(&json_file, config) {
                Ok(items) => all_items.extend(items),
                Err(e) => {
                    tracing::warn!("Failed to parse JSON file {:?}: {}", json_file, e);
                }
            }
        }

        Ok(all_items)
    }

    async fn fetch_item(&self, uri: &str) -> Result<SourceItem> {
        if !uri.starts_with("json://") {
            return Err(AgentRootError::Parse(format!(
                "Invalid JSON URI: {}. Expected format: json://path/to/file.json/item_N or json://path/to/file.json/key_X",
                uri
            )));
        }

        let uri_path = &uri[7..];

        if !uri_path.contains("/item_") && !uri_path.contains("/key_") {
            let file_path = Path::new(uri_path);
            let config =
                ProviderConfig::new(file_path.to_string_lossy().to_string(), "**/*".to_string());
            let items = self.parse_json_file(file_path, &config)?;
            return items.into_iter().next().ok_or_else(|| {
                AgentRootError::Parse(format!("No items found in JSON file {:?}", file_path))
            });
        }

        let parts: Vec<&str> = uri_path.rsplitn(2, '/').collect();
        if parts.len() != 2 {
            return Err(AgentRootError::Parse(format!(
                "Invalid JSON URI format: {}",
                uri
            )));
        }

        let file_path = Path::new(parts[1]);
        let config =
            ProviderConfig::new(file_path.to_string_lossy().to_string(), "**/*".to_string());

        let all_items = self.parse_json_file(file_path, &config)?;

        all_items
            .into_iter()
            .find(|item| item.uri == uri)
            .ok_or_else(|| {
                AgentRootError::Parse(format!("Item not found in JSON file {:?}", file_path))
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_type() {
        let provider = JSONProvider::new();
        assert_eq!(provider.provider_type(), "json");
    }

    #[tokio::test]
    async fn test_parse_json_array() {
        let provider = JSONProvider::new();
        let json_content = r#"[
            {"name": "Alice", "age": 30},
            {"name": "Bob", "age": 25}
        ]"#;

        let temp_dir = tempfile::tempdir().unwrap();
        let json_path = temp_dir.path().join("test.json");
        fs::write(&json_path, json_content).unwrap();

        let config = ProviderConfig::new(
            json_path.to_string_lossy().to_string(),
            "**/*.json".to_string(),
        );
        let items = provider.parse_json_file(&json_path, &config).unwrap();

        assert_eq!(items.len(), 2);
        assert!(items[0].content.contains("Alice"));
        assert_eq!(items[0].metadata.get("name").unwrap(), "Alice");
    }

    #[tokio::test]
    async fn test_parse_json_object() {
        let provider = JSONProvider::new();
        let json_content = r#"{
            "users": {"count": 100},
            "posts": {"count": 500}
        }"#;

        let temp_dir = tempfile::tempdir().unwrap();
        let json_path = temp_dir.path().join("test.json");
        fs::write(&json_path, json_content).unwrap();

        let mut config = ProviderConfig::new(
            json_path.to_string_lossy().to_string(),
            "**/*.json".to_string(),
        );
        config
            .options
            .insert("index_mode".to_string(), "object".to_string());

        let items = provider.parse_json_file(&json_path, &config).unwrap();

        assert_eq!(items.len(), 2);
        assert!(
            items[0].metadata.get("key").unwrap() == "users"
                || items[0].metadata.get("key").unwrap() == "posts"
        );
    }

    #[tokio::test]
    async fn test_parse_json_full() {
        let provider = JSONProvider::new();
        let json_content = r#"{"name": "Alice", "age": 30}"#;

        let temp_dir = tempfile::tempdir().unwrap();
        let json_path = temp_dir.path().join("test.json");
        fs::write(&json_path, json_content).unwrap();

        let mut config = ProviderConfig::new(
            json_path.to_string_lossy().to_string(),
            "**/*.json".to_string(),
        );
        config
            .options
            .insert("index_mode".to_string(), "full".to_string());

        let items = provider.parse_json_file(&json_path, &config).unwrap();

        assert_eq!(items.len(), 1);
        assert!(items[0].content.contains("Alice"));
    }

    #[tokio::test]
    async fn test_fetch_item_by_uri() {
        let provider = JSONProvider::new();
        let json_content = r#"[{"name": "Alice"}, {"name": "Bob"}]"#;

        let temp_dir = tempfile::tempdir().unwrap();
        let json_path = temp_dir.path().join("test.json");
        fs::write(&json_path, json_content).unwrap();

        let uri = format!("json://{}/item_0", json_path.display());
        let item = provider.fetch_item(&uri).await.unwrap();

        assert!(item.content.contains("Alice"));
        assert_eq!(item.metadata.get("index").unwrap(), "0");
    }
}
