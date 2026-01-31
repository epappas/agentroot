//! Link extraction from documents

use regex::Regex;
use std::path::{Path, PathBuf};

/// Extracted link from a document
#[derive(Debug, Clone)]
pub struct DocumentLink {
    pub link_type: LinkType,
    pub target_path: String,
}

#[derive(Debug, Clone, Copy)]
pub enum LinkType {
    MarkdownLink,
    CodeImport,
}

impl LinkType {
    pub fn as_str(&self) -> &'static str {
        match self {
            LinkType::MarkdownLink => "markdown_link",
            LinkType::CodeImport => "code_import",
        }
    }
}

/// Extract links from document content
pub fn extract_links(content: &str, source_path: &str, collection_path: &str) -> Vec<DocumentLink> {
    let mut links = Vec::new();

    links.extend(extract_markdown_links(
        content,
        source_path,
        collection_path,
    ));
    links.extend(extract_code_imports(content, source_path, collection_path));

    links
}

/// Extract markdown-style links: [text](path)
fn extract_markdown_links(
    content: &str,
    source_path: &str,
    collection_path: &str,
) -> Vec<DocumentLink> {
    let mut links = Vec::new();

    let re = Regex::new(r"\[([^\]]+)\]\(([^)]+)\)").expect("Invalid regex");

    for cap in re.captures_iter(content) {
        if let Some(target) = cap.get(2) {
            let target_str = target.as_str();

            if target_str.starts_with("http://") || target_str.starts_with("https://") {
                continue;
            }

            if target_str.starts_with('#') {
                continue;
            }

            if let Some(normalized) = normalize_path(target_str, source_path, collection_path) {
                links.push(DocumentLink {
                    link_type: LinkType::MarkdownLink,
                    target_path: normalized,
                });
            }
        }
    }

    links
}

/// Extract code imports (Rust, Python, JavaScript)
fn extract_code_imports(
    content: &str,
    source_path: &str,
    collection_path: &str,
) -> Vec<DocumentLink> {
    let mut links = Vec::new();

    if source_path.ends_with(".rs") {
        links.extend(extract_rust_imports(content, source_path, collection_path));
    } else if source_path.ends_with(".py") {
        links.extend(extract_python_imports(
            content,
            source_path,
            collection_path,
        ));
    } else if source_path.ends_with(".js") || source_path.ends_with(".ts") {
        links.extend(extract_js_imports(content, source_path, collection_path));
    }

    links
}

fn extract_rust_imports(
    content: &str,
    source_path: &str,
    collection_path: &str,
) -> Vec<DocumentLink> {
    let mut links = Vec::new();

    let mod_re = Regex::new(r"mod\s+([a-zA-Z_][a-zA-Z0-9_]*);").expect("Invalid regex");
    for cap in mod_re.captures_iter(content) {
        if let Some(module) = cap.get(1) {
            let module_name = module.as_str();
            let parent = Path::new(source_path).parent().unwrap_or(Path::new(""));
            let target = parent.join(format!("{}.rs", module_name));

            if let Some(normalized) =
                normalize_path(target.to_str().unwrap_or(""), source_path, collection_path)
            {
                links.push(DocumentLink {
                    link_type: LinkType::CodeImport,
                    target_path: normalized,
                });
            }
        }
    }

    links
}

fn extract_python_imports(
    content: &str,
    _source_path: &str,
    _collection_path: &str,
) -> Vec<DocumentLink> {
    let mut links = Vec::new();

    let import_re =
        Regex::new(r"from\s+([a-zA-Z_][a-zA-Z0-9_.]*)\s+import").expect("Invalid regex");
    for cap in import_re.captures_iter(content) {
        if let Some(module) = cap.get(1) {
            let module_path = module.as_str().replace('.', "/");
            links.push(DocumentLink {
                link_type: LinkType::CodeImport,
                target_path: format!("{}.py", module_path),
            });
        }
    }

    links
}

fn extract_js_imports(
    _content: &str,
    _source_path: &str,
    _collection_path: &str,
) -> Vec<DocumentLink> {
    Vec::new()
}

/// Normalize a relative path to collection-relative path
fn normalize_path(target: &str, source_path: &str, _collection_path: &str) -> Option<String> {
    let source = Path::new(source_path);
    let source_dir = source.parent().unwrap_or(Path::new(""));

    let target_path = Path::new(target);
    let resolved = if target_path.is_relative() {
        source_dir.join(target_path)
    } else {
        target_path.to_path_buf()
    };

    let normalized = normalize_pathbuf(&resolved);

    Some(normalized.to_string_lossy().to_string())
}

fn normalize_pathbuf(path: &Path) -> PathBuf {
    let mut components = Vec::new();

    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                components.pop();
            }
            std::path::Component::CurDir => {}
            _ => components.push(component.as_os_str()),
        }
    }

    components.iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_markdown_links() {
        let content = "See [docs](../README.md) and [guide](docs/guide.md)";
        let links = extract_markdown_links(content, "path/to/doc.md", "/collection");

        assert_eq!(links.len(), 2);
        assert_eq!(links[0].target_path, "path/README.md");
        assert_eq!(links[1].target_path, "path/to/docs/guide.md");
    }

    #[test]
    fn test_extract_rust_mod() {
        let content = "mod parser;\nmod scanner;";
        let links = extract_rust_imports(content, "src/index/mod.rs", "/collection");

        assert_eq!(links.len(), 2);
        assert!(links[0].target_path.ends_with("parser.rs"));
    }
}
