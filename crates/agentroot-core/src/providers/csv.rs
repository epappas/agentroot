//! CSV Provider for indexing CSV files row-by-row

use crate::db::hash_content;
use crate::error::{AgentRootError, Result};
use crate::providers::{ProviderConfig, SourceItem, SourceProvider};
use async_trait::async_trait;
use csv::ReaderBuilder;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Provider for indexing CSV files
pub struct CSVProvider;

impl Default for CSVProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl CSVProvider {
    /// Create a new CSVProvider
    pub fn new() -> Self {
        Self
    }

    /// Parse CSV file and return rows as items
    fn parse_csv_file(&self, path: &Path, config: &ProviderConfig) -> Result<Vec<SourceItem>> {
        let file_content = fs::read_to_string(path).map_err(|e| {
            AgentRootError::Io(std::io::Error::new(
                e.kind(),
                format!("Failed to read CSV file {:?}: {}", path, e),
            ))
        })?;

        let delimiter = config
            .options
            .get("delimiter")
            .and_then(|s| s.chars().next())
            .unwrap_or(',');

        let has_headers = config
            .options
            .get("has_headers")
            .map(|s| s == "true")
            .unwrap_or(true);

        let mut reader = ReaderBuilder::new()
            .delimiter(delimiter as u8)
            .has_headers(has_headers)
            .from_reader(file_content.as_bytes());

        let headers = if has_headers {
            reader.headers()?.clone()
        } else {
            csv::StringRecord::new()
        };

        let filename = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown.csv");

        let mut items = Vec::new();
        for (row_num, result) in reader.records().enumerate() {
            let record = result.map_err(|e| {
                AgentRootError::Parse(format!("Failed to parse CSV row {}: {}", row_num + 1, e))
            })?;

            let row_content = if has_headers && !headers.is_empty() {
                let mut parts = Vec::new();
                for (idx, field) in record.iter().enumerate() {
                    let header = headers.get(idx).unwrap_or("unknown");
                    parts.push(format!("{}: {}", header, field));
                }
                parts.join("\n")
            } else {
                record
                    .iter()
                    .enumerate()
                    .map(|(idx, field)| format!("column_{}: {}", idx, field))
                    .collect::<Vec<_>>()
                    .join("\n")
            };

            let title = if has_headers && !headers.is_empty() {
                format!("{} - Row {}", filename, row_num + 1)
            } else {
                format!("{} - Row {}", filename, row_num + 1)
            };

            let uri = format!("csv://{}/row_{}", path.display(), row_num + 1);
            let hash = hash_content(&row_content);

            let mut metadata = HashMap::new();
            metadata.insert("file".to_string(), filename.to_string());
            metadata.insert("row_number".to_string(), (row_num + 1).to_string());
            metadata.insert("column_count".to_string(), record.len().to_string());

            if has_headers {
                for (idx, field) in record.iter().enumerate() {
                    if let Some(header) = headers.get(idx) {
                        metadata.insert(header.to_string(), field.to_string());
                    }
                }
            }

            items.push(SourceItem {
                uri,
                title,
                content: row_content,
                hash,
                source_type: "csv".to_string(),
                metadata,
            });
        }

        Ok(items)
    }

    /// Scan directory for CSV files matching pattern
    fn scan_directory(&self, base_path: &Path, pattern: &str) -> Result<Vec<PathBuf>> {
        let glob_pattern = glob::Pattern::new(pattern)?;
        let mut csv_files = Vec::new();

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
                if ext.eq_ignore_ascii_case("csv") {
                    if let Ok(relative) = path.strip_prefix(base_path) {
                        let relative_str = relative.to_string_lossy();
                        if glob_pattern.matches(&relative_str) {
                            csv_files.push(path.to_path_buf());
                        }
                    }
                }
            }
        }

        Ok(csv_files)
    }
}

#[async_trait]
impl SourceProvider for CSVProvider {
    fn provider_type(&self) -> &'static str {
        "csv"
    }

    async fn list_items(&self, config: &ProviderConfig) -> Result<Vec<SourceItem>> {
        let base_path = Path::new(&config.base_path);

        if base_path.is_file() {
            if base_path
                .extension()
                .map(|e| e.eq_ignore_ascii_case("csv"))
                .unwrap_or(false)
            {
                return self.parse_csv_file(base_path, config);
            } else {
                return Err(AgentRootError::Parse(format!(
                    "File {:?} is not a CSV file",
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

        let csv_files = self.scan_directory(base_path, &config.pattern)?;
        let mut all_items = Vec::new();

        for csv_file in csv_files {
            match self.parse_csv_file(&csv_file, config) {
                Ok(items) => all_items.extend(items),
                Err(e) => {
                    tracing::warn!("Failed to parse CSV file {:?}: {}", csv_file, e);
                }
            }
        }

        Ok(all_items)
    }

    async fn fetch_item(&self, uri: &str) -> Result<SourceItem> {
        if !uri.starts_with("csv://") {
            return Err(AgentRootError::Parse(format!(
                "Invalid CSV URI: {}. Expected format: csv://path/to/file.csv/row_N",
                uri
            )));
        }

        let uri_path = &uri[6..];
        let parts: Vec<&str> = uri_path.rsplitn(2, '/').collect();
        if parts.len() != 2 || !parts[0].starts_with("row_") {
            return Err(AgentRootError::Parse(format!(
                "Invalid CSV URI format: {}. Expected: csv://path/to/file.csv/row_N",
                uri
            )));
        }

        let row_str = &parts[0][4..];
        let row_num: usize = row_str
            .parse()
            .map_err(|_| AgentRootError::Parse(format!("Invalid row number in URI: {}", uri)))?;

        let file_path = Path::new(parts[1]);
        let config =
            ProviderConfig::new(file_path.to_string_lossy().to_string(), "**/*".to_string());

        let all_items = self.parse_csv_file(file_path, &config)?;

        all_items
            .into_iter()
            .find(|item| item.uri == uri)
            .ok_or_else(|| {
                AgentRootError::Parse(format!(
                    "Row {} not found in CSV file {:?}",
                    row_num, file_path
                ))
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_type() {
        let provider = CSVProvider::new();
        assert_eq!(provider.provider_type(), "csv");
    }

    #[tokio::test]
    async fn test_parse_csv_with_headers() {
        let provider = CSVProvider::new();
        let csv_content = "name,age,city\nAlice,30,NYC\nBob,25,LA\n";

        let temp_dir = tempfile::tempdir().unwrap();
        let csv_path = temp_dir.path().join("test.csv");
        fs::write(&csv_path, csv_content).unwrap();

        let config = ProviderConfig::new(
            csv_path.to_string_lossy().to_string(),
            "**/*.csv".to_string(),
        );
        let items = provider.parse_csv_file(&csv_path, &config).unwrap();

        assert_eq!(items.len(), 2);
        assert!(items[0].content.contains("name: Alice"));
        assert!(items[0].content.contains("age: 30"));
        assert!(items[0].metadata.get("name").unwrap() == "Alice");
    }

    #[tokio::test]
    async fn test_parse_csv_custom_delimiter() {
        let provider = CSVProvider::new();
        let csv_content = "name;age;city\nAlice;30;NYC\n";

        let temp_dir = tempfile::tempdir().unwrap();
        let csv_path = temp_dir.path().join("test.csv");
        fs::write(&csv_path, csv_content).unwrap();

        let mut config = ProviderConfig::new(
            csv_path.to_string_lossy().to_string(),
            "**/*.csv".to_string(),
        );
        config
            .options
            .insert("delimiter".to_string(), ";".to_string());

        let items = provider.parse_csv_file(&csv_path, &config).unwrap();

        assert_eq!(items.len(), 1);
        assert!(items[0].content.contains("name: Alice"));
    }

    #[tokio::test]
    async fn test_fetch_item_by_uri() {
        let provider = CSVProvider::new();
        let csv_content = "name,age\nAlice,30\nBob,25\n";

        let temp_dir = tempfile::tempdir().unwrap();
        let csv_path = temp_dir.path().join("test.csv");
        fs::write(&csv_path, csv_content).unwrap();

        let uri = format!("csv://{}/row_1", csv_path.display());
        let item = provider.fetch_item(&uri).await.unwrap();

        assert!(item.content.contains("Alice"));
        assert_eq!(item.metadata.get("row_number").unwrap(), "1");
    }
}
