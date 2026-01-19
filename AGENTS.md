# Agent Guidelines for Agentroot

This document provides comprehensive guidelines for AI coding agents working in the Agentroot codebase, a Rust-based local semantic search engine for codebases and knowledge bases.

## Project Structure

This is a **Cargo workspace** with 4 crates:
- `agentroot-core` - Core library with db, index, search, and llm modules
- `agentroot-cli` - Command-line interface binary
- `agentroot-mcp` - MCP server for AI assistant integration
- `agentroot-tui` - Terminal UI (experimental)

## Build, Test, and Lint Commands

### Build Commands
```bash
# Build all workspace members
cargo build

# Build release version (optimized with LTO)
cargo build --release

# Build specific crate
cargo build -p agentroot-core
cargo build -p agentroot-cli

# Check compilation without building
cargo check
```

### Test Commands
```bash
# Run all tests
cargo test

# Run tests for specific package
cargo test -p agentroot-core

# Run single test by exact name
cargo test test_chunk_hash_stability

# Run tests matching a pattern
cargo test chunk_hash

# Run tests in a specific module
cargo test db::vectors
cargo test index::ast_chunker

# Run tests with output visible
cargo test test_name -- --nocapture

# Run tests single-threaded with output
cargo test test_name -- --nocapture --test-threads=1
```

### Lint and Format Commands
```bash
# Run clippy linter (with all warnings)
cargo clippy --all-targets --all-features

# Format code (uses default rustfmt)
cargo fmt

# Check formatting without modifying files
cargo fmt --check
```

### Documentation
```bash
# Build and open documentation
cargo doc --open

# Build docs with private items
cargo doc --document-private-items
```

## Code Style Guidelines

### General Principles
- **NEVER FAKE, STUB, MOCK, or use TODO**: Production-ready code only. Zero tolerance for placeholders.
- **Keep functions small**: Favor functions under 50 lines of code.
- **Modular design**: Follow SOLID principles, prefer composition over complex abstractions.
- **Early returns**: Prefer guard clauses and fail-fast patterns over nested if-else.
- **Type safety**: Always use explicit types. Never use type inference where clarity suffers.
- **No emoji**: Never use emojis or emoticons in code, comments, or commit messages.

### Import Organization
Organize imports in this order (separated by blank lines):
```rust
// 1. External crates
use tokio::sync::Mutex;
use serde::{Deserialize, Serialize};

// 2. Workspace crates
use agentroot_core::{Database, SemanticChunk};

// 3. Standard library
use std::path::{Path, PathBuf};
use std::sync::Arc;

// 4. Crate-internal modules
use crate::error::{Result, AgentRootError};
use crate::index::ChunkType;
```

### Module Structure and Re-exports
Follow the facade pattern for clean public APIs:

**lib.rs (crate root):**
```rust
pub mod config;
pub mod db;
pub mod error;

pub use config::{Config, CollectionConfig};
pub use db::Database;
pub use error::{AgentRootError, Result};
```

**Module-level mod.rs:**
```rust
mod scanner;
mod parser;
mod chunker;

pub use scanner::*;
pub use parser::*;
pub use chunker::*;
```

### Naming Conventions
- `snake_case` for functions, variables, modules, and file names
- `PascalCase` for types, traits, and enum variants
- `SCREAMING_SNAKE_CASE` for constants
- `'a, 'b` for lifetime parameters
- Prefix boolean functions with `is_`, `has_`, or `should_`

### Error Handling
```rust
// Use thiserror for library errors
#[derive(Debug, Error)]
pub enum AgentRootError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),
    
    #[error("Collection not found: {0}")]
    CollectionNotFound(String),
}

// Define type aliases for convenience
pub type Result<T> = std::result::Result<T, AgentRootError>;
pub type Error = AgentRootError;

// Always return Result, never unwrap in library code
pub fn do_something() -> Result<()> {
    // Use ? operator for error propagation
    let value = fallible_operation()?;
    Ok(())
}

// Use anyhow only in binaries (CLI) for convenience
// Use thiserror in libraries for proper error types
```

### Type Patterns
```rust
// Use &'static str for language identifiers and constants
pub language: Option<&'static str>,

// Use owned String for user-provided or dynamic data
pub content: String,

// Use PathBuf for owned paths, &Path for borrowed
pub fn scan_directory(path: &Path) -> Result<Vec<PathBuf>> { }

// Type aliases for complex types
pub type ChunkId = String;
pub type DocId = i64;
```

### Documentation
```rust
//! Module-level documentation using //!
//! 
//! Describes the purpose and usage of this module.

/// Function documentation using ///
/// 
/// # Arguments
/// 
/// * `path` - The path to scan
/// * `recursive` - Whether to scan subdirectories
/// 
/// # Returns
/// 
/// Returns a vector of file paths found
/// 
/// # Errors
/// 
/// Returns `Error::Io` if directory cannot be read
pub fn scan_directory(path: &Path, recursive: bool) -> Result<Vec<PathBuf>> {
    // Implementation
}
```

### Testing Patterns
```rust
// Tests are inline with #[cfg(test)]
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_chunk_hash_stability() {
        let chunk = SemanticChunk::new("content");
        let hash1 = chunk.compute_hash();
        let hash2 = chunk.compute_hash();
        assert_eq!(hash1, hash2);
    }
    
    // Use descriptive test names with test_ prefix
    #[test]
    fn test_empty_query_returns_no_results() {
        // Arrange
        let db = setup_test_db();
        
        // Act
        let results = db.search("").unwrap();
        
        // Assert
        assert!(results.is_empty());
    }
}
```

### Async Patterns
```rust
// Use tokio for async runtime
#[tokio::main]
async fn main() -> Result<()> {
    // Async code
}

// Mark async functions clearly
pub async fn embed_text(text: &str) -> Result<Vec<f32>> {
    // Implementation
}

// Use Arc<Mutex<T>> for shared mutable state in async
use std::sync::Arc;
use tokio::sync::Mutex;

let shared = Arc::new(Mutex::new(data));
```

## Git Commit Guidelines

- Write clear, concise commit messages focusing on WHY, not WHAT
- Use conventional commits format: `feat:`, `fix:`, `refactor:`, `docs:`, `test:`
- NEVER add "Generated with Claude Code" or similar attributions
- NEVER use `git add -A` - stage files explicitly with `git add <file>`
- NEVER add your name in commits (use project author/email from git config)
- Examples:
  - `feat: add AST-aware chunking for Python files`
  - `fix: prevent cache invalidation on whitespace-only changes`
  - `refactor: extract chunk hashing into separate module`

## Key Dependencies and Patterns

- **Database**: rusqlite for SQLite (bundled, with FTS5 and blob support)
- **Async**: tokio with full features
- **CLI**: clap with derive macros
- **Serialization**: serde with derive macros, serde_json, serde_yaml
- **Error handling**: thiserror for libs, anyhow for binaries
- **AST parsing**: tree-sitter with language-specific parsers
- **Hashing**: blake3 for content-addressable chunk hashing
- **Logging**: tracing with tracing-subscriber

## Critical Rules

1. **NEVER fake, stub, mock, or use placeholders** - Production code only
2. **NEVER rewrite or skip tests** - Fix the actual issue
3. **NEVER delete code to make tests pass** - Understand and fix the root cause
4. **Keep functions under 50 lines** - Extract helper functions for clarity
5. **Use explicit error handling** - No unwrap/expect in library code
6. **Follow Rust conventions** - Use rustfmt and clippy defaults
7. **Write real tests** - Test actual functionality, not mocks
8. **Make informed decisions** - Base edits on analysis, not assumptions
9. **Keep comments concise** - Let code be self-documenting where possible

## When Making Changes

1. **Read before editing** - Always read files before modifying them
2. **Understand context** - When asked to analyze/reflect, provide contextual answers before editing
3. **Run tests** - After changes, run relevant tests to verify
4. **Check formatting** - Run `cargo fmt` before committing
5. **Run clippy** - Address any clippy warnings introduced
6. **Commit atomically** - Stage specific files, not everything at once
