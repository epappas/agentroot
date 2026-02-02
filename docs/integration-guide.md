# Agentroot Integration Guide

This guide covers three ways to integrate with Agentroot: as a Rust library (SDK), via the CLI, and through the MCP server for AI assistants.

## 1. SDK Library Integration

Add `agentroot-core` as a dependency in your Rust project:

```toml
[dependencies]
agentroot-core = { path = "../agentroot/crates/agentroot-core" }
chrono = "0.4"
```

### In-Memory Database (Testing / Ephemeral)

```rust
use agentroot_core::{Database, SearchOptions};
use agentroot_core::db::hash_content;
use chrono::Utc;

fn main() -> agentroot_core::Result<()> {
    let db = Database::open_in_memory()?;
    db.initialize()?;
    db.add_collection("docs", ".", "**/*.md", "file", None)?;

    let now = Utc::now().to_rfc3339();
    let content = "Rust ownership rules prevent data races at compile time.";
    let hash = hash_content(content);
    db.insert_content(&hash, content)?;
    db.insert_document("docs", "ownership.md", "Ownership", &hash, &now, &now, "file", None)?;

    let results = db.search_fts("ownership", &SearchOptions::default())?;
    for r in &results {
        println!("{} (score: {:.2})", r.display_path, r.score);
    }
    Ok(())
}
```

### Persistent Database

```rust
let db = Database::open(std::path::Path::new("/path/to/index.sqlite"))?;
db.initialize()?;
```

Or use the default system path:

```rust
let db = Database::open(&Database::default_path())?;
// Reads from ~/.cache/agentroot/index.sqlite
```

### SDK Examples

Run any example with:

```bash
cargo run --example <name> -p agentroot-core
```

| Example | What It Demonstrates |
|---------|---------------------|
| `basic_search` | Insert documents, BM25 search, retrieve by path |
| `user_metadata` | MetadataBuilder, MetadataFilter, find_by_metadata, merge/remove |
| `chunk_navigation` | insert_chunk, FTS chunk search, label search, surrounding chunks |
| `workflow_search` | Workflow/WorkflowStep, execute_workflow, fallback_workflow |
| `vector_hybrid_search` | Embeddings, cosine similarity, BM25+vector RRF fusion |
| `semantic_chunking` | AST-aware chunking for Rust and Python |
| `custom_index` | Scan directory, chunk files, index with hashes |
| `custom_provider` | Implement SourceProvider trait for a JSON API |
| `test_glossary_search` | Glossary concept upsert and search |

### Key API Reference

#### Documents

```rust
// Insert content (content-addressed by SHA-256)
let hash = agentroot_core::db::hash_content(content);
db.insert_content(&hash, content)?;

// Insert document metadata
db.insert_document(collection, path, title, &hash, &created, &modified, "file", None)?;

// Or use DocumentInsert for LLM metadata
use agentroot_core::db::DocumentInsert;
db.insert_doc(
    &DocumentInsert::new("coll", "path.md", "Title", &hash, &now, &now)
        .with_llm_metadata_strings(summary, title, keywords_json, category, intent, concepts_json, difficulty, queries_json, model, generated_at),
)?;
```

#### Search

```rust
use agentroot_core::{SearchOptions, SearchResult};

let opts = SearchOptions {
    limit: 10,
    min_score: 0.0,
    collection: Some("docs".into()),
    provider: None,
    full_content: false,
    metadata_filters: vec![],
};

// BM25 full-text search
let results: Vec<SearchResult> = db.search_fts("query", &opts)?;

// Chunk-level BM25
let chunk_results = db.search_chunks_bm25("query", &opts)?;
```

#### User Metadata

```rust
use agentroot_core::{MetadataBuilder, MetadataFilter};

// Build typed metadata
let meta = MetadataBuilder::new()
    .text("author", "Alice")
    .tags("topics", vec!["rust", "async"])
    .integer("difficulty", 2)
    .boolean("published", true)
    .build();

// Attach to document (docid = first 6 chars of hash)
let docid = &hash[..6];
db.add_metadata(docid, &meta)?;

// Query by metadata
let filter = MetadataFilter::And(vec![
    MetadataFilter::IntegerGt("difficulty".into(), 1),
    MetadataFilter::TagsContain("topics".into(), "rust".into()),
]);
let doc_ids: Vec<String> = db.find_by_metadata(&filter, 20)?;
```

#### Chunks

```rust
use std::collections::HashMap;

let chunk_hash = agentroot_core::db::hash_content(chunk_content);
let labels: HashMap<String, String> = [("layer".into(), "service".into())].into();
let concepts: Vec<String> = vec!["validation".into()];

db.insert_chunk(
    &chunk_hash, &doc_hash, /*seq=*/0, /*pos=*/0,
    chunk_content, Some("Function"), Some("Config::validate"),
    /*start_line=*/10, /*end_line=*/20, Some("rust"),
    Some("Validates config"), Some("Validation logic"),
    &concepts, &labels, &[],
    None, None, &chrono::Utc::now().to_rfc3339(),
)?;

// Retrieve chunks
let chunks = db.get_chunks_for_document(&doc_hash)?;
let fts_hits = db.search_chunks_fts("validate", 10)?;
let label_hits = db.search_chunks_by_label("layer", "service")?;
let (prev, next) = db.get_surrounding_chunks(&chunk_hash)?;
```

#### Workflows

```rust
use agentroot_core::llm::{Workflow, WorkflowStep, MergeStrategy, fallback_workflow};
use agentroot_core::search::{execute_workflow, SearchOptions};

// Build a custom workflow
let wf = Workflow {
    steps: vec![
        WorkflowStep::Bm25Search { query: "error handling".into(), limit: 10 },
        WorkflowStep::Bm25ChunkSearch { query: "error handling".into(), limit: 10 },
        WorkflowStep::Merge { strategy: MergeStrategy::Rrf },
        WorkflowStep::Deduplicate,
        WorkflowStep::Limit { count: 5 },
    ],
    reasoning: "Merge doc + chunk results".into(),
    expected_results: 5,
    complexity: "moderate".into(),
};

let results = execute_workflow(&db, &wf, "error handling", &SearchOptions::default()).await?;

// Or use the heuristic fallback
let auto_wf = fallback_workflow("fn validate", /*has_embeddings=*/false);
```

#### Vector Search

```rust
use agentroot_core::db::vectors::cosine_similarity;

// Setup
db.ensure_vec_table(768)?;

// Insert embedding
db.insert_embedding(&hash, /*seq=*/0, /*pos=*/0, "model-name", &embedding_vec)?;

// Manual similarity search
let all = db.get_all_embeddings()?;
for (hash_seq, emb) in &all {
    let sim = cosine_similarity(&query_embedding, emb);
    println!("{}: {:.4}", hash_seq, sim);
}
```

## 2. CLI Integration

### Installation

```bash
git clone https://github.com/epappas/agentroot
cd agentroot
cargo install --path crates/agentroot-cli
```

### Core Workflow

```bash
# 1. Add a collection
agentroot collection add /path/to/code --name myproject --mask '**/*.rs'

# 2. Index files
agentroot update

# 3. Search
agentroot search "error handling"          # BM25
agentroot query "error handling"           # Hybrid (BM25 + vector)
agentroot smart "how do we handle errors"  # LLM-powered

# 4. Retrieve documents
agentroot get "#a1b2c3"                    # By docid
agentroot get myproject/src/main.rs        # By path
agentroot multi-get "myproject/src/*.rs"   # Glob pattern
```

### Output Formats

All commands support `--format`:

```bash
agentroot search "query" --format json   # For programmatic use
agentroot search "query" --format csv    # For spreadsheets
agentroot search "query" --format files  # File paths only (pipe to xargs)
```

### Metadata via CLI

```bash
# Add metadata
agentroot metadata add "#docid" --text "author=Alice" --tags "topics=rust,async" --integer "difficulty=2"

# Query by metadata
agentroot metadata query "author:eq=Alice"
agentroot metadata query "difficulty:gt=2"
agentroot metadata query "topics:has=rust"

# Retrieve
agentroot metadata get "#docid"

# Remove fields
agentroot metadata remove "#docid" difficulty
```

### Environment Variables

| Variable | Purpose | Default |
|----------|---------|---------|
| `AGENTROOT_DB` | Database path | `~/.cache/agentroot/index.sqlite` |
| `AGENTROOT_LLM_URL` | vLLM/OpenAI-compatible endpoint | `http://localhost:8000` |
| `AGENTROOT_LLM_MODEL` | Chat model name | `meta-llama/Llama-3.1-8B-Instruct` |
| `AGENTROOT_EMBEDDING_URL` | Embedding endpoint | Falls back to LLM_URL |
| `AGENTROOT_EMBEDDING_MODEL` | Embedding model | `sentence-transformers/all-MiniLM-L6-v2` |
| `AGENTROOT_EMBEDDING_DIMS` | Embedding dimensions | Auto-detected |
| `AGENTROOT_LLM_API_KEY` | API key for LLM service | None |
| `GITHUB_TOKEN` | GitHub API token | None |

## 3. MCP Server Integration

The MCP server exposes 29 tools over JSON-RPC (stdin/stdout) for AI assistant integration.

### Starting the Server

```bash
agentroot mcp
```

### Claude Desktop

Edit the config file for your platform:

- macOS: `~/Library/Application Support/Claude/claude_desktop_config.json`
- Linux: `~/.config/Claude/claude_desktop_config.json`
- Windows: `%APPDATA%\Claude\claude_desktop_config.json`

```json
{
  "mcpServers": {
    "agentroot": {
      "command": "agentroot",
      "args": ["mcp"]
    }
  }
}
```

Restart Claude Desktop after editing.

### Claude Code

Add to your `.claude/settings.json` or project-level `.mcp.json`:

```json
{
  "mcpServers": {
    "agentroot": {
      "command": "agentroot",
      "args": ["mcp"]
    }
  }
}
```

### Custom Database Path

If your index lives at a non-default location:

```json
{
  "mcpServers": {
    "agentroot": {
      "command": "agentroot",
      "args": ["mcp"],
      "env": {
        "AGENTROOT_DB": "/path/to/index.sqlite"
      }
    }
  }
}
```

### Available MCP Tools

| Tool | Description |
|------|------------|
| `search` | BM25 full-text search with metadata filters |
| `vsearch` | Vector similarity search (requires embeddings) |
| `query` | Hybrid BM25 + vector search |
| `smart_search` | Intelligent NL search with auto query understanding |
| `get` | Retrieve document by path, docid, or URI |
| `multi_get` | Retrieve multiple documents by glob/list |
| `status` | Index status and collection info |
| `collection_add` | Add a new collection |
| `collection_remove` | Remove a collection |
| `collection_update` | Reindex a collection |
| `metadata_add` | Add user metadata to a document |
| `metadata_get` | Get user metadata for a document |
| `metadata_query` | Query documents by metadata filters |
| `search_chunks` | Search code chunks (functions, classes, etc.) |
| `get_chunk` | Retrieve a chunk by hash with context |
| `navigate_chunks` | Navigate to previous/next chunk in a document |
| `session_start` | Start a multi-turn search session |
| `session_get` | Get session context and query history |
| `session_set` | Set key-value context on a session |
| `session_end` | End a session and clean up |
| `browse_directory` | Browse directory structure of collections |
| `search_directories` | Search directories by name or concepts |
| `batch_search` | Execute multiple queries in one call |
| `explore` | Search with exploration suggestions |
| `memory_store` | Store a long-term memory |
| `memory_search` | Search memories via FTS |
| `memory_list` | List memories with pagination |
| `memory_extract` | LLM-extract memories from a session |
| `memory_delete` | Delete a memory by ID |

### MCP Tool Examples

**Search with metadata filter:**

```json
{
  "name": "search",
  "arguments": {
    "query": "error handling",
    "limit": 10,
    "category": "tutorial",
    "difficulty": "beginner"
  }
}
```

**Add metadata:**

```json
{
  "name": "metadata_add",
  "arguments": {
    "docid": "#e192a2",
    "metadata": {
      "author": "Alice",
      "difficulty": 3,
      "topics": ["rust", "async"]
    }
  }
}
```

**Query metadata:**

```json
{
  "name": "metadata_query",
  "arguments": {
    "field": "author",
    "operator": "eq",
    "value": "Alice"
  }
}
```

**Search chunks:**

```json
{
  "name": "search_chunks",
  "arguments": {
    "query": "validate config",
    "limit": 5,
    "label": "layer:service"
  }
}
```

**Navigate chunks:**

```json
{
  "name": "navigate_chunks",
  "arguments": {
    "chunk_hash": "abc123...",
    "direction": "next"
  }
}
```

**Start a session:**

```json
{
  "name": "session_start",
  "arguments": { "ttl_seconds": 3600 }
}
```

**Batch search:**

```json
{
  "name": "batch_search",
  "arguments": {
    "queries": [
      { "query": "error handling", "limit": 5 },
      { "query": "authentication", "limit": 5 }
    ]
  }
}
```

**Store a memory:**

```json
{
  "name": "memory_store",
  "arguments": {
    "content": "Project uses SQLite with FTS5",
    "category": "fact",
    "confidence": 0.95
  }
}
```

**Search memories:**

```json
{
  "name": "memory_search",
  "arguments": { "query": "database", "limit": 10 }
}
```

### Manual Testing

Test the MCP server from the command line:

```bash
# Initialize
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}' \
  | agentroot mcp 2>/dev/null

# List tools
printf '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}\n{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}\n' \
  | agentroot mcp 2>/dev/null

# Search
printf '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}\n{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"search","arguments":{"query":"error handling"}}}\n' \
  | agentroot mcp 2>/dev/null
```

## 4. LLM Service Setup (Optional)

Agentroot can use an external vLLM-compatible service for metadata generation, query expansion, and reranking. This is optional -- BM25 search and all SDK features work without it.

### With vLLM

```bash
# Start vLLM
python -m vllm.entrypoints.openai.api_server \
  --model Qwen/Qwen2.5-7B-Instruct \
  --port 8000

# Configure agentroot
export AGENTROOT_LLM_URL="http://localhost:8000/v1"
export AGENTROOT_LLM_MODEL="Qwen/Qwen2.5-7B-Instruct"
```

### With OpenAI-Compatible APIs

```bash
export AGENTROOT_LLM_URL="https://api.openai.com/v1"
export AGENTROOT_LLM_MODEL="gpt-4"
export AGENTROOT_LLM_API_KEY="sk-..."
```

### With Embeddings

```bash
export AGENTROOT_EMBEDDING_URL="http://localhost:8001/v1"
export AGENTROOT_EMBEDDING_MODEL="intfloat/e5-mistral-7b-instruct"
export AGENTROOT_EMBEDDING_DIMS=4096
```

After configuring, generate embeddings and metadata:

```bash
agentroot update      # Indexes + generates metadata via LLM
agentroot embed       # Generates vector embeddings
agentroot vsearch "semantic query"  # Now works
```

## Architecture Overview

```
+------------------+     +------------------+     +------------------+
|   CLI Binary     |     |   MCP Server     |     |  Your Rust App   |
|  (agentroot)     |     |  (agentroot mcp) |     |  (SDK library)   |
+--------+---------+     +--------+---------+     +--------+---------+
         |                         |                        |
         +-------------------------+------------------------+
                                   |
                          +--------+---------+
                          |  agentroot-core  |
                          |    (library)     |
                          +--------+---------+
                                   |
              +--------------------+--------------------+
              |                    |                    |
     +--------+-------+  +--------+-------+  +--------+-------+
     |   Database      |  |   Search       |  |   LLM          |
     |   (SQLite +     |  |   (BM25, Vec,  |  |   (External    |
     |    FTS5 +       |  |    Hybrid,     |  |    HTTP via    |
     |    sqlite-vec)  |  |    Workflows)  |  |    vLLM/OAI)   |
     +--------+--------+  +----------------+  +----------------+
```

All three interfaces share the same `agentroot-core` library and SQLite database. Data indexed via the CLI is immediately searchable via MCP and vice versa.
