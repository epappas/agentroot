//! Virtual path utilities for agentroot:// URIs

use std::collections::HashMap;
use std::path::PathBuf;
use crate::error::{AgentRootError, Result};
use crate::VIRTUAL_PATH_PREFIX;

/// Check if a string is a virtual path
pub fn is_virtual_path(path: &str) -> bool {
    path.starts_with(VIRTUAL_PATH_PREFIX)
}

/// Normalize a virtual path (lowercase collection, normalize slashes)
pub fn normalize_virtual_path(path: &str) -> String {
    if !is_virtual_path(path) {
        return path.to_string();
    }

    let rest = &path[VIRTUAL_PATH_PREFIX.len()..];
    let parts: Vec<&str> = rest.splitn(2, '/').collect();

    if parts.is_empty() {
        return path.to_string();
    }

    let collection = parts[0].to_lowercase();
    let file_path = parts.get(1).unwrap_or(&"");

    // Normalize path separators
    let normalized_path = file_path
        .replace('\\', "/")
        .split('/')
        .filter(|s| !s.is_empty() && *s != ".")
        .collect::<Vec<_>>()
        .join("/");

    format!("{}{}/{}", VIRTUAL_PATH_PREFIX, collection, normalized_path)
}

/// Parse a virtual path into (collection, path) tuple
pub fn parse_virtual_path(vpath: &str) -> Result<(String, String)> {
    if !is_virtual_path(vpath) {
        return Err(AgentRootError::InvalidVirtualPath(
            format!("Not a virtual path: {}", vpath)
        ));
    }

    let rest = &vpath[VIRTUAL_PATH_PREFIX.len()..];
    let parts: Vec<&str> = rest.splitn(2, '/').collect();

    if parts.is_empty() || parts[0].is_empty() {
        return Err(AgentRootError::InvalidVirtualPath(
            format!("Missing collection in virtual path: {}", vpath)
        ));
    }

    let collection = parts[0].to_string();
    let path = parts.get(1).map(|s| s.to_string()).unwrap_or_default();

    Ok((collection, path))
}

/// Build a virtual path from collection and path
pub fn build_virtual_path(collection: &str, path: &str) -> String {
    let path = path.trim_start_matches('/');
    format!("{}{}/{}", VIRTUAL_PATH_PREFIX, collection, path)
}

/// Convert an absolute path to a virtual path
pub fn to_virtual_path(
    abs_path: &str,
    collection_name: &str,
    collection_root: &str,
) -> Result<String> {
    let abs = std::path::Path::new(abs_path);
    let root = std::path::Path::new(collection_root);

    let rel_path = abs.strip_prefix(root)
        .map_err(|_| AgentRootError::InvalidVirtualPath(
            format!("Path {} is not under collection root {}", abs_path, collection_root)
        ))?;

    let rel_str = rel_path.to_string_lossy();
    Ok(build_virtual_path(collection_name, &rel_str))
}

/// Resolve a virtual path to an absolute filesystem path
pub fn resolve_virtual_path(
    vpath: &str,
    collections: &HashMap<String, PathBuf>,
) -> Result<PathBuf> {
    let (collection, path) = parse_virtual_path(vpath)?;

    let collection_root = collections.get(&collection)
        .ok_or_else(|| AgentRootError::CollectionNotFound(collection.clone()))?;

    Ok(collection_root.join(path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_virtual_path() {
        assert!(is_virtual_path("agentroot://docs/readme.md"));
        assert!(!is_virtual_path("/home/user/docs/readme.md"));
        assert!(!is_virtual_path("docs/readme.md"));
    }

    #[test]
    fn test_parse_virtual_path() {
        let (coll, path) = parse_virtual_path("agentroot://docs/2024/notes.md").unwrap();
        assert_eq!(coll, "docs");
        assert_eq!(path, "2024/notes.md");
    }

    #[test]
    fn test_build_virtual_path() {
        assert_eq!(
            build_virtual_path("docs", "2024/notes.md"),
            "agentroot://docs/2024/notes.md"
        );
    }

    #[test]
    fn test_normalize_virtual_path() {
        assert_eq!(
            normalize_virtual_path("agentroot://DOCS/./foo//bar.md"),
            "agentroot://docs/foo/bar.md"
        );
    }
}
