//! Document parsing utilities

use regex::Regex;
use lazy_static::lazy_static;
use std::path::Path;

lazy_static! {
    static ref HEADING_RE: Regex = Regex::new(r"^##?\s+(.+)$").unwrap();
    static ref SECOND_HEADING_RE: Regex = Regex::new(r"^##\s+(.+)$").unwrap();
}

/// Generic headings to skip
const SKIP_TITLES: &[&str] = &["Notes", "README", "Index"];

/// Extract title from markdown content
pub fn extract_title(content: &str, filename: &str) -> String {
    for line in content.lines().take(50) {
        if let Some(caps) = HEADING_RE.captures(line) {
            let title = caps.get(1).map(|m| m.as_str().trim()).unwrap_or("");

            if SKIP_TITLES.iter().any(|&s| title == s || title.contains("Notes")) {
                for line2 in content.lines().skip(1).take(50) {
                    if let Some(caps2) = SECOND_HEADING_RE.captures(line2) {
                        if let Some(title2) = caps2.get(1) {
                            return title2.as_str().trim().to_string();
                        }
                    }
                }
            }

            if !title.is_empty() {
                return title.to_string();
            }
        }
    }

    Path::new(filename)
        .file_stem()
        .and_then(|s| s.to_str())
        .map(|s| s.replace(['-', '_'], " "))
        .unwrap_or_else(|| filename.to_string())
}

/// Normalize path for storage (handelize)
pub fn handelize(path: &str) -> String {
    path.to_lowercase()
        .replace("___", "/")
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '/' || c == '.' || c == '-' { c } else { '-' })
        .collect::<String>()
        .replace("--", "-")
        .trim_matches('-')
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_title_heading() {
        let content = "# My Document\n\nSome content here.";
        assert_eq!(extract_title(content, "doc.md"), "My Document");
    }

    #[test]
    fn test_extract_title_fallback() {
        let content = "No heading here, just text.";
        assert_eq!(extract_title(content, "my-doc.md"), "my doc");
    }

    #[test]
    fn test_handelize() {
        assert_eq!(handelize("My Docs/2024/Report.md"), "my-docs/2024/report.md");
        assert_eq!(handelize("foo___bar.md"), "foo/bar.md");
    }
}
