# Architecture

This document describes the architecture of Agentroot, a local semantic search system for codebases.

## System Overview

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              Agentroot                                       │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐    │
│  │ agentroot-cli│  │agentroot-tui │  │ agentroot-mcp│  │   External   │    │
│  │   (CLI)      │  │   (TUI)      │  │  (MCP Server)│  │   Clients    │    │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘    │
│         │                 │                 │                 │             │
│         └─────────────────┴────────┬────────┴─────────────────┘             │
│                                    │                                         │
│                          ┌─────────▼─────────┐                              │
│                          │   agentroot-core   │                              │
│                          │    (Core Library)  │                              │
│                          └─────────┬─────────┘                              │
│                                    │                                         │
│         ┌──────────────────────────┼──────────────────────────┐             │
│         │                          │                          │             │
│  ┌──────▼──────┐          ┌───────▼───────┐          ┌───────▼───────┐     │
│  │   Index     │          │    Search     │          │      LLM      │     │
│  │  Pipeline   │          │    Engine     │          │   (Embedder)  │     │
│  └──────┬──────┘          └───────┬───────┘          └───────┬───────┘     │
│         │                         │                          │             │
│         └─────────────────────────┼──────────────────────────┘             │
│                                   │                                         │
│                          ┌────────▼────────┐                                │
│                          │     SQLite      │                                │
│                          │    Database     │                                │
│                          └─────────────────┘                                │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Crate Structure

### agentroot-core

The core library providing all search and indexing functionality.

```
agentroot-core/src/
├── lib.rs              # Public API exports
├── error.rs            # Error types (AgentRootError)
├── config/
│   ├── mod.rs          # Configuration management
│   └── virtual_path.rs # Virtual path handling (qmd://)
├── db/
│   ├── mod.rs          # Database connection and initialization
│   ├── schema.rs       # Schema definitions and migrations
│   ├── collections.rs  # Collection CRUD operations
│   ├── documents.rs    # Document storage
│   ├── content.rs      # Content storage and hashing
│   ├── vectors.rs      # Embedding storage and similarity
│   ├── context.rs      # Context/metadata storage
│   └── stats.rs        # Statistics queries
├── index/
│   ├── mod.rs          # Index pipeline orchestration
│   ├── scanner.rs      # File system scanning
│   ├── parser.rs       # Document parsing (title extraction)
│   ├── chunker.rs      # Character-based chunking
│   ├── embedder.rs     # Embedding generation with caching
│   └── ast_chunker/    # AST-aware semantic chunking
│       ├── mod.rs      # SemanticChunker API
│       ├── types.rs    # ChunkType, ChunkMetadata, SemanticChunk
│       ├── language.rs # Language detection from file paths
│       ├── parser.rs   # Tree-sitter parsing wrapper
│       ├── oversized.rs# Large chunk splitting
│       └── strategies/ # Language-specific extraction
│           ├── mod.rs  # ChunkingStrategy trait
│           ├── rust.rs
│           ├── python.rs
│           ├── javascript.rs
│           └── go.rs
├── search/
│   ├── mod.rs          # Search module exports
│   ├── bm25.rs         # BM25 full-text search
│   ├── vector.rs       # Vector similarity search
│   ├── hybrid.rs       # RRF-based result fusion
│   └── snippet.rs      # Result snippet generation
└── llm/
    ├── mod.rs          # LLM module exports
    ├── traits.rs       # Embedder trait definition
    └── llama.rs        # llama.cpp integration
```

### agentroot-cli

Command-line interface for all operations.

```
agentroot-cli/src/
├── main.rs             # Entry point and CLI definition
├── app.rs              # Application state
├── commands/
│   ├── mod.rs          # Command exports
│   ├── collection.rs   # collection add/list/remove
│   ├── update.rs       # update command
│   ├── embed.rs        # embed command
│   ├── search.rs       # search/vsearch/query commands
│   ├── get.rs          # get/multi-get commands
│   ├── ls.rs           # ls command
│   ├── status.rs       # status command
│   ├── context.rs      # context management
│   └── cleanup.rs      # database cleanup
└── output/
    ├── mod.rs          # Output format handling
    ├── terminal.rs     # CLI output formatting
    ├── json.rs         # JSON output
    ├── csv.rs          # CSV output
    ├── markdown.rs     # Markdown output
    └── files.rs        # Files-only output
```

### agentroot-mcp

Model Context Protocol server for AI assistant integration.

```
agentroot-mcp/src/
├── lib.rs              # Library exports
├── server.rs           # MCP server implementation
├── protocol.rs         # MCP protocol types
├── tools.rs            # MCP tool definitions
└── resources.rs        # MCP resource handling
```

### agentroot-tui

Terminal user interface (experimental).

```
agentroot-tui/src/
├── main.rs             # Entry point
├── app.rs              # Application state
├── event.rs            # Event handling
└── ui/
    └── mod.rs          # UI components
```

## Database Schema

Agentroot uses SQLite with FTS5 for full-text search.

### Core Tables

```sql
-- Collections: groups of indexed files
CREATE TABLE collections (
    name TEXT PRIMARY KEY,
    path TEXT NOT NULL,
    mask TEXT,           -- glob pattern
    exclude TEXT,        -- exclusion pattern
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Documents: indexed files
CREATE TABLE documents (
    id INTEGER PRIMARY KEY,
    collection TEXT NOT NULL,
    path TEXT NOT NULL,
    hash TEXT NOT NULL,
    title TEXT,
    active INTEGER DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (collection) REFERENCES collections(name)
);

-- Content: document text content
CREATE TABLE content (
    hash TEXT PRIMARY KEY,
    doc TEXT NOT NULL
);

-- FTS5 index for full-text search
CREATE VIRTUAL TABLE content_fts USING fts5(
    doc,
    content='content',
    content_rowid='rowid'
);
```

### Vector Tables

```sql
-- Content vectors: chunk metadata
CREATE TABLE content_vectors (
    hash TEXT NOT NULL,
    seq INTEGER NOT NULL,
    pos INTEGER NOT NULL,
    model TEXT NOT NULL,
    chunk_hash TEXT,
    created_at TEXT NOT NULL,
    PRIMARY KEY (hash, seq)
);

-- Embeddings: vector storage as BLOBs
CREATE TABLE embeddings (
    hash_seq TEXT PRIMARY KEY,
    embedding BLOB NOT NULL
);

-- Model metadata: dimension tracking
CREATE TABLE model_metadata (
    model TEXT PRIMARY KEY,
    dimensions INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    last_used_at TEXT NOT NULL
);

-- Chunk embeddings cache: content-addressable storage
CREATE TABLE chunk_embeddings (
    chunk_hash TEXT NOT NULL,
    model TEXT NOT NULL,
    embedding BLOB NOT NULL,
    created_at TEXT NOT NULL,
    PRIMARY KEY (chunk_hash, model)
);
```

## Data Flow

### Indexing Pipeline

```
File System
    │
    ▼
┌─────────────────┐
│    Scanner      │  Walks directories, applies masks
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│     Parser      │  Extracts title, computes content hash
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│   AST Chunker   │  Language detection → tree-sitter parsing
└────────┬────────┘  → semantic unit extraction → chunk hashing
         │
         ▼
┌─────────────────┐
│    Embedder     │  Cache lookup → batch embedding → cache store
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│    Database     │  Store documents, content, vectors
└─────────────────┘
```

### Search Pipeline

```
Query String
    │
    ▼
┌──────────────────────────────────────────┐
│            Parallel Search               │
├─────────────────┬────────────────────────┤
│   BM25 Search   │    Vector Search       │
│   (FTS5)        │    (Cosine Similarity) │
└────────┬────────┴───────────┬────────────┘
         │                    │
         └────────┬───────────┘
                  ▼
         ┌───────────────┐
         │  RRF Fusion   │  Reciprocal Rank Fusion
         └───────┬───────┘
                 │
                 ▼
         ┌───────────────┐
         │   Results     │  Ranked, deduplicated results
         └───────────────┘
```

## Key Design Decisions

### 1. Content-Addressable Chunk Hashing

Each chunk is hashed using blake3 with its context:

```rust
fn compute_chunk_hash(text: &str, leading: &str, trailing: &str) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(leading.as_bytes());
    hasher.update(text.as_bytes());
    hasher.update(trailing.as_bytes());
    hasher.finalize().to_hex()[..32].to_string()
}
```

This enables:
- Deduplication across documents
- Cache reuse when content hasn't changed
- Fast invalidation when content changes

### 2. AST-Aware Semantic Chunking

Code files are parsed with tree-sitter and chunked by semantic units:

```rust
pub trait ChunkingStrategy {
    fn semantic_node_types(&self) -> &[&str];
    fn extract_chunks(&self, source: &str, root: Node) -> Result<Vec<SemanticChunk>>;
    fn chunk_type_for_node(&self, node: Node) -> ChunkType;
}
```

Benefits:
- Functions stay intact (not split mid-body)
- Context preserved (docstrings, comments)
- Better embedding quality for code

### 3. Transaction Safety

Multi-statement operations are wrapped in transactions:

```rust
self.conn.execute("BEGIN IMMEDIATE", [])?;
let result = (|| {
    // Multiple operations...
    Ok(())
})();
if result.is_ok() {
    self.conn.execute("COMMIT", [])?;
} else {
    let _ = self.conn.execute("ROLLBACK", []);
}
result
```

### 4. Static Language Strings

Language identifiers use `&'static str` to avoid heap allocation per chunk:

```rust
pub struct ChunkMetadata {
    pub language: Option<&'static str>,  // Not String
    // ...
}
```

### 5. Incremental Line Counting

Large chunk splitting uses O(n) incremental line counting instead of O(n^2):

```rust
let mut lines_to_prev_end = 0;
let mut prev_end = 0;

// In loop:
lines_to_prev_end += text[prev_end..end].matches('\n').count();
let end_line = base_line + lines_to_prev_end;
```

## Error Handling

All operations return `Result<T, AgentRootError>`:

```rust
#[derive(Debug, thiserror::Error)]
pub enum AgentRootError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("LLM error: {0}")]
    Llm(String),

    #[error("{0}")]
    Other(String),
}
```

## Testing

Tests are organized by module:

```bash
# Run all tests
cargo test

# Run specific module tests
cargo test ast_chunker
cargo test vectors

# Run with output
cargo test -- --nocapture
```

## Performance Considerations

1. **Batch Embedding**: Chunks are batched (32 at a time) to maximize GPU utilization
2. **Cache First**: Always check cache before computing embeddings
3. **Lazy Loading**: Embeddings loaded only when needed for vector search
4. **FTS5 Optimization**: BM25 search uses SQLite's optimized FTS5 implementation
5. **Connection Pooling**: Single connection with WAL mode for concurrent reads
