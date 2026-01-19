//! PDF Provider for extracting text from PDF files

use crate::db::hash_content;
use crate::error::{AgentRootError, Result};
use crate::providers::{ProviderConfig, SourceItem, SourceProvider};
use async_trait::async_trait;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Provider for extracting text from PDF files
pub struct PDFProvider;

impl Default for PDFProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl PDFProvider {
    /// Create a new PDFProvider
    pub fn new() -> Self {
        Self
    }

    /// Extract text from a PDF file
    fn extract_text_from_pdf(&self, path: &Path) -> Result<String> {
        let bytes = fs::read(path).map_err(|e| {
            AgentRootError::Io(std::io::Error::new(
                e.kind(),
                format!("Failed to read PDF file {:?}: {}", path, e),
            ))
        })?;

        let text = pdf_extract::extract_text_from_mem(&bytes).map_err(|e| {
            AgentRootError::Parse(format!("Failed to extract text from PDF {:?}: {}", path, e))
        })?;

        if text.trim().is_empty() {
            return Err(AgentRootError::Parse(format!(
                "PDF file {:?} contains no extractable text (may be image-based)",
                path
            )));
        }

        Ok(text)
    }

    /// Extract title from PDF text content
    fn extract_title(&self, content: &str, filename: &str) -> String {
        let first_line = content
            .lines()
            .map(|l| l.trim())
            .find(|l| !l.is_empty())
            .unwrap_or("");

        if !first_line.is_empty() && first_line.len() < 200 {
            return first_line.to_string();
        }

        Path::new(filename)
            .file_stem()
            .and_then(|s| s.to_str())
            .map(|s| s.replace('_', " ").replace('-', " "))
            .unwrap_or_else(|| "Untitled PDF".to_string())
    }

    /// Scan directory for PDF files matching pattern
    fn scan_directory(&self, base_path: &Path, pattern: &str) -> Result<Vec<PathBuf>> {
        let glob_pattern = glob::Pattern::new(pattern)?;
        let mut pdf_files = Vec::new();

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
                if ext.eq_ignore_ascii_case("pdf") {
                    if let Ok(relative) = path.strip_prefix(base_path) {
                        let relative_str = relative.to_string_lossy();
                        if glob_pattern.matches(&relative_str) {
                            pdf_files.push(path.to_path_buf());
                        }
                    }
                }
            }
        }

        Ok(pdf_files)
    }
}

#[async_trait]
impl SourceProvider for PDFProvider {
    fn provider_type(&self) -> &'static str {
        "pdf"
    }

    async fn list_items(&self, config: &ProviderConfig) -> Result<Vec<SourceItem>> {
        let base_path = Path::new(&config.base_path);
        if !base_path.exists() {
            return Err(AgentRootError::InvalidInput(format!(
                "Path does not exist: {}",
                config.base_path
            )));
        }

        let pdf_files = if base_path.is_file() {
            if base_path.extension().and_then(|e| e.to_str()) == Some("pdf") {
                vec![base_path.to_path_buf()]
            } else {
                return Err(AgentRootError::InvalidInput(format!(
                    "File is not a PDF: {}",
                    config.base_path
                )));
            }
        } else {
            self.scan_directory(base_path, &config.pattern)?
        };

        let mut items = Vec::new();
        for pdf_path in pdf_files {
            match self.extract_text_from_pdf(&pdf_path) {
                Ok(content) => {
                    let filename = pdf_path.to_string_lossy().to_string();
                    let title = self.extract_title(&content, &filename);
                    let hash = hash_content(&content);

                    let mut item =
                        SourceItem::new(filename.clone(), title, content, hash, "pdf".to_string());
                    item.metadata
                        .insert("file_path".to_string(), filename.clone());
                    if let Some(stem) = pdf_path.file_stem() {
                        item.metadata
                            .insert("filename".to_string(), stem.to_string_lossy().to_string());
                    }

                    items.push(item);
                }
                Err(e) => {
                    tracing::warn!("Skipping PDF {:?}: {}", pdf_path, e);
                }
            }
        }

        Ok(items)
    }

    async fn fetch_item(&self, uri: &str) -> Result<SourceItem> {
        let path = Path::new(uri);
        let content = self.extract_text_from_pdf(path)?;
        let title = self.extract_title(&content, uri);
        let hash = hash_content(&content);

        let mut item = SourceItem::new(uri.to_string(), title, content, hash, "pdf".to_string());
        item.metadata
            .insert("file_path".to_string(), uri.to_string());
        if let Some(stem) = path.file_stem() {
            item.metadata
                .insert("filename".to_string(), stem.to_string_lossy().to_string());
        }

        Ok(item)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_type() {
        let provider = PDFProvider::new();
        assert_eq!(provider.provider_type(), "pdf");
    }

    #[test]
    fn test_extract_title_from_content() {
        let provider = PDFProvider::new();
        let content = "   \n\nDocument Title\n\nSome content here...";
        let title = provider.extract_title(content, "test.pdf");
        assert_eq!(title, "Document Title");
    }

    #[test]
    fn test_extract_title_from_filename() {
        let provider = PDFProvider::new();
        let content = "";
        let title = provider.extract_title(content, "my_important_document.pdf");
        assert_eq!(title, "my important document");
    }

    #[test]
    fn test_extract_title_with_dashes() {
        let provider = PDFProvider::new();
        let content = "";
        let title = provider.extract_title(content, "user-guide-v2.pdf");
        assert_eq!(title, "user guide v2");
    }

    #[test]
    fn test_extract_title_long_first_line() {
        let provider = PDFProvider::new();
        let long_line = "a".repeat(250);
        let content = format!("{}\n\nMore content", long_line);
        let title = provider.extract_title(&content, "document.pdf");
        assert_eq!(title, "document");
    }
}
