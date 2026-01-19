//! File scanning for indexing

use glob::Pattern;
use std::path::{Path, PathBuf};
use walkdir::{DirEntry, WalkDir};
use crate::error::Result;

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

/// Scan result
#[derive(Debug, Clone)]
pub struct ScanResult {
    pub path: PathBuf,
    pub relative_path: String,
}

/// Scan options
#[derive(Debug, Clone)]
pub struct ScanOptions {
    pub pattern: String,
    pub follow_symlinks: bool,
    pub exclude_dirs: Vec<String>,
    pub exclude_hidden: bool,
}

impl Default for ScanOptions {
    fn default() -> Self {
        Self {
            pattern: "**/*.md".to_string(),
            follow_symlinks: true,
            exclude_dirs: EXCLUDE_DIRS.iter().map(|s| s.to_string()).collect(),
            exclude_hidden: true,
        }
    }
}

/// Scan directory for files matching pattern
pub fn scan_files(root: &Path, options: &ScanOptions) -> Result<Vec<ScanResult>> {
    let pattern = Pattern::new(&options.pattern)?;
    let mut results = Vec::new();

    let walker = WalkDir::new(root)
        .follow_links(options.follow_symlinks)
        .into_iter()
        .filter_entry(|e| !should_skip(e, options));

    for entry in walker {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path();
        let relative = path.strip_prefix(root)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| path.to_string_lossy().to_string());

        if pattern.matches(&relative) {
            results.push(ScanResult {
                path: path.to_path_buf(),
                relative_path: relative,
            });
        }
    }

    Ok(results)
}

fn should_skip(entry: &DirEntry, options: &ScanOptions) -> bool {
    let name = entry.file_name().to_string_lossy();

    if options.exclude_hidden && name.starts_with('.') {
        return true;
    }

    if entry.file_type().is_dir() && options.exclude_dirs.iter().any(|d| name == *d) {
        return true;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_options() {
        let opts = ScanOptions::default();
        assert_eq!(opts.pattern, "**/*.md");
        assert!(opts.exclude_hidden);
    }
}
