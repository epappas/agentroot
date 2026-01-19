//! Error types for agentroot

use thiserror::Error;

/// Result type alias using AgentRootError
pub type Result<T> = std::result::Result<T, AgentRootError>;

/// Error type alias for convenience
pub type Error = AgentRootError;

/// Exit codes for CLI
pub mod exit_codes {
    pub const SUCCESS: i32 = 0;
    pub const GENERAL_ERROR: i32 = 1;
    pub const NOT_FOUND: i32 = 2;
    pub const INVALID_INPUT: i32 = 3;
}

/// Main error type for agentroot
#[derive(Debug, Error)]
pub enum AgentRootError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Walk directory error: {0}")]
    WalkDir(#[from] walkdir::Error),

    #[error("Collection not found: {0}")]
    CollectionNotFound(String),

    #[error("Document not found: {0}")]
    DocumentNotFound(String),

    #[error("Invalid virtual path: {0}")]
    InvalidVirtualPath(String),

    #[error("LLM error: {0}")]
    Llm(String),

    #[error("Model not found: {0}")]
    ModelNotFound(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Index error: {0}")]
    Index(String),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Search error: {0}")]
    Search(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("YAML error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Regex error: {0}")]
    Regex(#[from] regex::Error),

    #[error("Glob pattern error: {0}")]
    GlobPattern(#[from] glob::PatternError),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("External service error: {0}")]
    ExternalError(String),

    #[error("{0}")]
    Other(#[from] anyhow::Error),
}

impl AgentRootError {
    /// Get the exit code for this error
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::CollectionNotFound(_) | Self::DocumentNotFound(_) => exit_codes::NOT_FOUND,
            Self::InvalidVirtualPath(_) | Self::Config(_) => exit_codes::INVALID_INPUT,
            _ => exit_codes::GENERAL_ERROR,
        }
    }
}
