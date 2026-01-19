//! Language detection from file paths

use std::path::Path;

/// Supported programming languages for AST chunking
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    Rust,
    Python,
    JavaScript,
    TypeScript,
    TypeScriptTsx,
    Go,
}

impl Language {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Rust => "rust",
            Self::Python => "python",
            Self::JavaScript => "javascript",
            Self::TypeScript => "typescript",
            Self::TypeScriptTsx => "tsx",
            Self::Go => "go",
        }
    }

    /// Detect language from file path extension
    pub fn from_path(path: &Path) -> Option<Self> {
        let ext = path.extension()?.to_str()?;
        Self::from_extension(ext)
    }

    /// Detect language from file extension string
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "rs" => Some(Self::Rust),
            "py" | "pyi" => Some(Self::Python),
            "js" | "mjs" | "cjs" | "jsx" => Some(Self::JavaScript),
            "ts" | "mts" | "cts" => Some(Self::TypeScript),
            "tsx" => Some(Self::TypeScriptTsx),
            "go" => Some(Self::Go),
            _ => None,
        }
    }
}

/// Check if a file path is supported for AST chunking
pub fn is_supported(path: &Path) -> bool {
    Language::from_path(path).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rust_detection() {
        assert_eq!(
            Language::from_path(Path::new("foo.rs")),
            Some(Language::Rust)
        );
        assert_eq!(
            Language::from_path(Path::new("src/lib.rs")),
            Some(Language::Rust)
        );
    }

    #[test]
    fn test_python_detection() {
        assert_eq!(
            Language::from_path(Path::new("foo.py")),
            Some(Language::Python)
        );
        assert_eq!(
            Language::from_path(Path::new("foo.pyi")),
            Some(Language::Python)
        );
    }

    #[test]
    fn test_javascript_detection() {
        assert_eq!(
            Language::from_path(Path::new("foo.js")),
            Some(Language::JavaScript)
        );
        assert_eq!(
            Language::from_path(Path::new("foo.mjs")),
            Some(Language::JavaScript)
        );
        assert_eq!(
            Language::from_path(Path::new("foo.jsx")),
            Some(Language::JavaScript)
        );
    }

    #[test]
    fn test_typescript_detection() {
        assert_eq!(
            Language::from_path(Path::new("foo.ts")),
            Some(Language::TypeScript)
        );
        assert_eq!(
            Language::from_path(Path::new("foo.tsx")),
            Some(Language::TypeScriptTsx)
        );
    }

    #[test]
    fn test_go_detection() {
        assert_eq!(Language::from_path(Path::new("foo.go")), Some(Language::Go));
    }

    #[test]
    fn test_unsupported() {
        assert_eq!(Language::from_path(Path::new("foo.md")), None);
        assert_eq!(Language::from_path(Path::new("foo.txt")), None);
        assert_eq!(Language::from_path(Path::new("foo")), None);
    }

    #[test]
    fn test_is_supported() {
        assert!(is_supported(Path::new("foo.rs")));
        assert!(is_supported(Path::new("foo.py")));
        assert!(!is_supported(Path::new("foo.md")));
    }
}
