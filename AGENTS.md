# Agent Guidelines for Agentroot

This document provides comprehensive guidelines for AI coding agents working in the Agentroot codebase, a Rust-based local semantic search engine for codebases and knowledge bases.

## Project Structure

This is a **Cargo workspace** with 4 crates:
- `agentroot-core` - Core library with db, index, search, and llm modules
- `agentroot-cli` - Command-line interface binary
- `agentroot-mcp` - MCP server for AI assistant integration
- `agentroot-tui` - Terminal UI (experimental)

### Repository Structure
```
agentroot/
├── crates/
│   ├── agentroot-core/      # Core library
│   │   ├── src/
│   │   │   ├── db/          # Database layer (schema, collections, documents, vectors)
│   │   │   ├── index/       # Indexing (scanner, parser, chunker, ast_chunker)
│   │   │   ├── search/      # Search (bm25, vector, hybrid)
│   │   │   ├── llm/         # LLM integration (embedder, reranker, expander)
│   │   │   └── lib.rs
│   │   └── Cargo.toml
│   ├── agentroot-cli/       # CLI binary
│   │   ├── src/
│   │   │   ├── commands/    # CLI commands (index, search, query, etc.)
│   │   │   └── main.rs
│   │   └── Cargo.toml
│   ├── agentroot-mcp/       # MCP server
│   │   ├── src/
│   │   │   ├── protocol.rs  # MCP protocol types
│   │   │   ├── tools.rs     # MCP tool definitions
│   │   │   └── main.rs
│   │   └── Cargo.toml
│   └── agentroot-tui/       # TUI (experimental)
├── examples/                # Code examples (at root for visibility)
│   ├── basic_search.rs
│   ├── semantic_chunking.rs
│   ├── custom_index.rs
│   └── README.md
├── docs/                    # User documentation
│   ├── getting-started.md
│   ├── mcp-server.md
│   ├── troubleshooting.md
│   └── performance.md
├── Cargo.toml              # Workspace manifest
├── LICENSE                 # MIT
├── README.md
├── CONTRIBUTING.md
├── CHANGELOG.md
└── AGENTS.md              # This file
```

## Technical Constants

### Chunking Configuration
Located in `crates/agentroot-core/src/index/chunker.rs`:
```rust
pub const CHUNK_SIZE_TOKENS: usize = 800;
pub const CHUNK_OVERLAP_TOKENS: usize = 120;
pub const CHUNK_SIZE_CHARS: usize = 3200;
pub const CHUNK_OVERLAP_CHARS: usize = 480;
```

### Search Configuration
Located in `crates/agentroot-core/src/search/hybrid.rs`:
```rust
const RRF_K: f64 = 60.0;                  // Reciprocal Rank Fusion constant
const MAX_RERANK_DOCS: usize = 40;         // Maximum docs sent to reranker
const STRONG_SIGNAL_SCORE: f64 = 0.85;     // High confidence threshold
const STRONG_SIGNAL_GAP: f64 = 0.15;       // Score gap for strong signal
```

### LLM Configuration
Located in `crates/agentroot-core/src/llm/llama.rs`:
```rust
pub const DEFAULT_EMBED_MODEL: &str = "nomic-embed-text-v1.5.Q4_K_M.gguf";
```

### Database Schema Version
Located in `crates/agentroot-core/src/db/schema.rs`:
```rust
const SCHEMA_VERSION: i32 = 2;
```

## Core Data Models

### SemanticChunk
Located in `crates/agentroot-core/src/index/ast_chunker/types.rs`:
```rust
pub struct SemanticChunk {
    pub text: String,              // The chunk text content
    pub chunk_type: ChunkType,     // Type of semantic unit (NOT in metadata)
    pub chunk_hash: String,        // blake3 hash (32 chars)
    pub position: usize,           // Byte position in source
    pub token_count: Option<usize>,// Token count (if computed)
    pub metadata: ChunkMetadata,   // Additional metadata
}

pub struct ChunkMetadata {
    pub leading_trivia: String,    // Comments/docs above
    pub trailing_trivia: String,   // Comments after
    pub breadcrumb: Option<String>,// Hierarchical path (e.g., "MyClass::my_method")
    pub language: Option<&'static str>, // Source language
    pub start_line: usize,         // Starting line (1-indexed)
    pub end_line: usize,           // Ending line (1-indexed)
}

pub enum ChunkType {
    Function, Method, Class, Struct, Enum, Trait, 
    Interface, Module, Import, Text
}
```

**CRITICAL**: `chunk_type` is a direct field on `SemanticChunk`, NOT inside `metadata`. Always access as `chunk.chunk_type`, never as `chunk.metadata.chunk_type`.

### Document
Located in `crates/agentroot-core/src/db/documents.rs`:
```rust
pub struct Document {
    pub id: i64,
    pub collection: String,
    pub path: String,
    pub title: String,
    pub hash: String,
    pub created_at: String,
    pub modified_at: String,
    pub active: bool,
}

pub struct DocumentResult {
    pub filepath: String,
    pub display_path: String,
    pub title: String,
    pub context: Option<String>,
    pub hash: String,
    pub docid: String,
    pub collection_name: String,
    pub modified_at: String,
    pub body_length: usize,
    pub body: Option<String>,
}
```

### CollectionInfo
Located in `crates/agentroot-core/src/db/collections.rs`:
```rust
pub struct CollectionInfo {
    pub name: String,
    pub path: String,
    pub pattern: String,        // Default: "**/*.md"
    pub document_count: usize,
    pub created_at: String,
    pub updated_at: String,
}
```

## Database API Signatures

### Database Lifecycle
Located in `crates/agentroot-core/src/db/schema.rs` and `mod.rs`:
```rust
// Create/open database
Database::open(path: &Path) -> Result<Self>

// Initialize tables and schema
db.initialize() -> Result<()>

// Get default path (~/.cache/agentroot/index.sqlite)
Database::default_path() -> PathBuf
```

**CRITICAL**: Always call `db.initialize()` after `Database::open()`. The database will not work without initialization.

### Collection Operations
Located in `crates/agentroot-core/src/db/collections.rs`:
```rust
// Add a new collection
db.add_collection(name: &str, path: &str, pattern: &str) -> Result<()>

// Remove collection and its documents
db.remove_collection(name: &str) -> Result<bool>

// Rename collection
db.rename_collection(old_name: &str, new_name: &str) -> Result<bool>

// List all collections
db.list_collections() -> Result<Vec<CollectionInfo>>

// Get collection by name
db.get_collection(name: &str) -> Result<Option<CollectionInfo>>
```

**CRITICAL**: Use `add_collection()`, NOT `create_collection()`. The latter does not exist.

### Document Operations
Located in `crates/agentroot-core/src/db/documents.rs`:
```rust
// Insert new document (6 parameters)
db.insert_document(
    collection: &str,
    path: &str,
    title: &str,
    hash: &str,
    created_at: &str,    // ISO 8601 timestamp
    modified_at: &str,   // ISO 8601 timestamp
) -> Result<i64>

// Get document by various methods
db.get_document(file: &str) -> Result<Option<DocumentResult>>
db.get_documents_by_pattern(pattern: &str) -> Result<Vec<DocumentResult>>
```

**CRITICAL**: `insert_document()` requires 6 parameters including both `created_at` and `modified_at` timestamps in ISO 8601 format (RFC 3339).

### Search Operations
Located in `crates/agentroot-core/src/search/`:
```rust
// BM25 full-text search
db.search_fts(query: &str, options: &SearchOptions) -> Result<Vec<SearchResult>>

// Vector similarity search
db.search_vector(query: &str, options: &SearchOptions) -> Result<Vec<SearchResult>>

// Hybrid search (BM25 + Vector + Reranking)
db.search_hybrid(query: &str, options: &SearchOptions) -> Result<Vec<SearchResult>>

pub struct SearchOptions {
    pub limit: usize,
    pub min_score: f64,
    pub collection: Option<String>,
    pub full_content: bool,
}
```

## Database Schema

Located in `crates/agentroot-core/src/db/schema.rs`:

### Tables
```sql
-- Content storage (content-addressable by SHA-256 hash)
CREATE TABLE content (
    hash TEXT PRIMARY KEY,
    doc TEXT NOT NULL,
    created_at TEXT NOT NULL
);

-- Document metadata
CREATE TABLE documents (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    collection TEXT NOT NULL,
    path TEXT NOT NULL,
    title TEXT NOT NULL,
    hash TEXT NOT NULL REFERENCES content(hash),
    created_at TEXT NOT NULL,
    modified_at TEXT NOT NULL,
    active INTEGER NOT NULL DEFAULT 1,
    UNIQUE(collection, path)
);

-- Full-text search index (FTS5)
CREATE VIRTUAL TABLE documents_fts USING fts5(
    filepath,
    title,
    body,
    tokenize='porter unicode61'
);

-- Vector embeddings metadata
CREATE TABLE content_vectors (
    hash TEXT NOT NULL,
    seq INTEGER NOT NULL,
    pos INTEGER NOT NULL,
    model TEXT NOT NULL,
    chunk_hash TEXT,
    created_at TEXT NOT NULL,
    PRIMARY KEY (hash, seq)
);

-- Model metadata for dimension validation
CREATE TABLE model_metadata (
    model TEXT PRIMARY KEY,
    dimensions INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    last_used_at TEXT NOT NULL
);

-- Global chunk embeddings cache
CREATE TABLE chunk_embeddings (
    chunk_hash TEXT NOT NULL,
    model TEXT NOT NULL,
    embedding BLOB NOT NULL,
    created_at TEXT NOT NULL,
    PRIMARY KEY (chunk_hash, model)
);

-- Collections metadata
CREATE TABLE collections (
    name TEXT PRIMARY KEY,
    path TEXT NOT NULL,
    pattern TEXT NOT NULL DEFAULT '**/*.md',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Context metadata (hierarchical context for paths)
CREATE TABLE contexts (
    path TEXT PRIMARY KEY,
    context TEXT NOT NULL,
    created_at TEXT NOT NULL
);

-- LLM response cache
CREATE TABLE llm_cache (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    model TEXT NOT NULL,
    created_at TEXT NOT NULL
);
```

## MCP Tools

Located in `crates/agentroot-mcp/src/tools.rs`, 6 tools are defined:

### 1. search (BM25 Full-Text Search)
Line 8-37
```rust
Parameters:
- query: string (required) - Search query (keywords/phrases)
- limit: integer (default: 20) - Maximum results
- minScore: number (default: 0) - Minimum relevance 0-1
- collection: string (optional) - Filter by collection
```

### 2. vsearch (Vector Similarity Search)
Line 39-68
```rust
Parameters:
- query: string (required) - Natural language query
- limit: integer (default: 20) - Maximum results
- minScore: number (default: 0.3) - Minimum similarity 0-1
- collection: string (optional) - Filter by collection
```

### 3. query (Hybrid Search)
Line 70-94
```rust
Parameters:
- query: string (required) - Search query
- limit: integer (default: 20) - Maximum results
- collection: string (optional) - Filter by collection

Best quality: Combines BM25, vectors, and reranking with RRF
```

### 4. get (Get Single Document)
Line 96-124
```rust
Parameters:
- file: string (required) - File path, docid (#abc123), or agentroot:// URI
- fromLine: integer (optional) - Start line number
- maxLines: integer (optional) - Maximum lines
- lineNumbers: boolean (default: false) - Include line numbers
```

### 5. multi_get (Get Multiple Documents)
Line 126-155
```rust
Parameters:
- pattern: string (required) - Glob pattern or comma-separated paths/docids
- maxLines: integer (optional) - Maximum lines per file
- maxBytes: integer (default: 10240) - Skip files larger than this
- lineNumbers: boolean (default: false) - Include line numbers
```

### 6. status (Index Status)
Line 157-166
```rust
Parameters: none

Returns: Collection info, document counts, cache stats
```

## Supported Languages

Located in `crates/agentroot-core/src/index/ast_chunker/language.rs`:

AST-aware chunking supports:
- **Rust**: `.rs`
- **Python**: `.py`, `.pyi`
- **JavaScript**: `.js`, `.mjs`, `.cjs`, `.jsx`
- **TypeScript**: `.ts`, `.mts`, `.cts`
- **TypeScript JSX**: `.tsx`
- **Go**: `.go`

Files without these extensions fall back to character-based chunking using `CHUNK_SIZE_CHARS` and `CHUNK_OVERLAP_CHARS`.

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

## Common Pitfalls

Based on verified bugs found during development:

1. **Wrong API Method**:
   - ❌ `db.create_collection()` - Does NOT exist
   - ✅ `db.add_collection(name, path, pattern)` - Correct method

2. **Missing Database Initialization**:
   - ❌ `let db = Database::open(path)?;` - Database won't work
   - ✅ `let db = Database::open(path)?; db.initialize()?;` - Required

3. **Wrong Field Access on SemanticChunk**:
   - ❌ `chunk.metadata.chunk_type` - Field does NOT exist in metadata
   - ✅ `chunk.chunk_type` - Direct field on SemanticChunk

4. **Missing Parameters in insert_document()**:
   - ❌ `db.insert_document(collection, path, title, hash)` - Missing 2 params
   - ✅ `db.insert_document(collection, path, title, hash, created_at, modified_at)` - All 6 required

5. **Wrong Constants in Documentation**:
   - ❌ `CHUNK_SIZE_CHARS = 2000` - Old incorrect value
   - ✅ `CHUNK_SIZE_CHARS = 3200` - Actual value in chunker.rs:6

6. **Timestamp Format**:
   - ❌ `created_at: "2024-01-01"` - Wrong format
   - ✅ `created_at: Utc::now().to_rfc3339()` - ISO 8601/RFC 3339 required

## Git Configuration

**Remote Configuration**:
- Remote name: `github` (NOT `origin`)
- URL: `git@github.com:epappas/agentroot.git`
- Branch: `master`

**When Pushing**:
```bash
# Correct
git push github master

# Wrong - remote 'origin' may not exist
git push origin master
```

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
