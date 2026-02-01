# Agentic Extensions Architecture

This document specifies the system architecture for extending Agentroot with three new capabilities: **Tiered Context Loading**, **Hierarchical/Directory-Aware Retrieval**, and **Session Memory**. These extensions position Agentroot as a first-class backend for AI agents by reducing token consumption, improving retrieval relevance through structural awareness, and enabling stateful multi-turn interactions.

## Table of Contents

- [Design Principles](#design-principles)
- [Current State Summary](#current-state-summary)
- [Feature 1: Tiered Context Loading (L0/L1/L2)](#feature-1-tiered-context-loading-l0l1l2)
- [Feature 2: Hierarchical/Directory-Aware Retrieval](#feature-2-hierarchicaldirectory-aware-retrieval)
- [Feature 3: Session Memory](#feature-3-session-memory)
- [Feature 4: Agentic Interface Extensions](#feature-4-agentic-interface-extensions)
- [Database Migrations](#database-migrations)
- [MCP Tool Extensions](#mcp-tool-extensions)
- [Implementation Order](#implementation-order)

---

## Design Principles

All extensions follow these constraints:

1. **Additive, not breaking** -- New tables, new columns, new MCP tools. No existing API changes.
2. **KISS** -- Minimal abstractions. Flat module structure. No deep trait hierarchies.
3. **DRY** -- Reuse existing `Database`, `SearchResult`, `SearchOptions` types. Extend, don't duplicate.
4. **SOLID** -- Each new module has a single responsibility. New functionality is injected via composition, not inheritance.
5. **Offline-first** -- All features work without LLM. LLM enhances quality but is never required.
6. **SQLite-native** -- All state in SQLite. No external dependencies (Redis, filesystem state, etc.).

---

## Current State Summary

Key existing building blocks this design extends:

| Component | Location | Relevant State |
|-----------|----------|----------------|
| Document metadata | `db/documents.rs` | `llm_summary`, `llm_title`, `llm_keywords`, `llm_category` already stored per document |
| Chunk metadata | `db/chunks.rs` | `llm_summary`, `llm_purpose`, `llm_concepts`, `llm_labels` already stored per chunk |
| Search results | `search/mod.rs` | `SearchResult` has `body`, `context` (snippet), `llm_summary`, `is_chunk`, chunk fields |
| SearchOptions | `search/mod.rs` | Has `full_content: bool`, `limit`, `min_score`, `collection`, `metadata_filters` |
| MCP server | `agentroot-mcp/src/server.rs` | Stateless `McpServer { db: &Database }`. 16 tools registered. |
| FTS index | `db/schema.rs` | `documents_fts` indexes `filepath`, `title`, `body`, `llm_summary`, `llm_title`, `llm_keywords` |
| Chunk FTS | `db/schema.rs` | `chunks_fts` indexes `content`, `breadcrumb`, `llm_summary`, `llm_purpose` |
| Hierarchical context | `db/context.rs` | `contexts` table with `path TEXT PRIMARY KEY, context TEXT` -- prefix-matched |
| Config | `config/mod.rs` | `get_context_for_path()` already does prefix-based context inheritance |
| PageRank | `graph/pagerank.rs` | `importance_score` on documents, `document_links` table |
| Provider | `providers/file.rs` | `FileProvider` walks directories, stores relative paths |

---

## Feature 1: Tiered Context Loading (L0/L1/L2)

### Problem

MCP consumers (Claude, agents) receive full document content or fixed-length snippets. There is no way to request "give me just enough to decide if this is relevant" vs "give me the full content." This wastes tokens on irrelevant results and forces agents to over-fetch.

### Design

Three tiers of detail, requested via a `detail` parameter on search and retrieval tools:

| Tier | Name | Token Budget | Content Returned | Source |
|------|------|-------------|------------------|--------|
| L0 | Abstract | ~50-100 tokens | Title + category + difficulty + 1-sentence summary | `llm_summary` (first sentence) or `llm_title` + metadata |
| L1 | Overview | ~500-2000 tokens | Summary + keywords + concepts + snippet | `llm_summary` (full) + `llm_keywords` + `context` snippet |
| L2 | Full | Unlimited | Complete document or chunk content | `body` (full content from `content` table) |

### Data Model

No new tables needed. The data already exists:

- **L0**: Derived from `documents.llm_title`, `documents.llm_category`, `documents.llm_difficulty`, first sentence of `documents.llm_summary`. For chunks: `chunks.llm_summary` (first sentence), `chunks.chunk_type`, `chunks.breadcrumb`.
- **L1**: `documents.llm_summary` (full), `documents.llm_keywords`, `documents.llm_concepts`, snippet from `content.doc`. For chunks: `chunks.llm_summary`, `chunks.llm_purpose`, `chunks.llm_concepts`, `chunks.content` (truncated).
- **L2**: `content.doc` (full document). For chunks: `chunks.content` (full).

### SearchOptions Extension

```rust
// In search/mod.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DetailLevel {
    L0,         // Abstract: title + category + 1-sentence summary
    #[default]
    L1,         // Overview: summary + keywords + snippet
    L2,         // Full: complete content
}

// Add to existing SearchOptions:
pub struct SearchOptions {
    // ... existing fields ...
    pub detail: DetailLevel,  // New field, defaults to L1
}
```

### SearchResult Changes

No struct changes needed. The existing `SearchResult` fields are populated differently based on `DetailLevel`:

| Field | L0 | L1 | L2 |
|-------|----|----|-----|
| `title` | Populated | Populated | Populated |
| `llm_summary` | First sentence only | Full summary | Full summary |
| `llm_category` | Populated | Populated | Populated |
| `llm_difficulty` | Populated | Populated | Populated |
| `llm_keywords` | `None` | Populated | Populated |
| `context` | `None` | Snippet (~500 chars) | `None` (use `body`) |
| `body` | `None` | `None` | Full content |
| `body_length` | Populated (for budget estimation) | Populated | Populated |
| `chunk_summary` | First sentence | Full | Full |
| `chunk_purpose` | `None` | Populated | Populated |
| `chunk_concepts` | Empty | Populated | Populated |

### Implementation: Detail Projection

A single function in `search/mod.rs` that projects a `SearchResult` to a given `DetailLevel`:

```rust
impl SearchResult {
    pub fn project(&mut self, detail: DetailLevel) {
        match detail {
            DetailLevel::L0 => {
                self.body = None;
                self.context = None;
                self.llm_keywords = None;
                self.llm_summary = self.llm_summary.as_ref().map(|s| first_sentence(s));
                self.chunk_summary = self.chunk_summary.as_ref().map(|s| first_sentence(s));
                self.chunk_purpose = None;
                self.chunk_concepts = vec![];
            }
            DetailLevel::L1 => {
                self.body = None;
                // context (snippet) already populated by search
                // llm_summary, llm_keywords already populated
            }
            DetailLevel::L2 => {
                // body loaded from content table
                self.context = None; // redundant when body is present
            }
        }
    }
}
```

For L2, the body must be loaded. This is handled in the search functions by checking `detail == L2` the same way `full_content == true` is currently handled. In fact, `detail: L2` replaces the `full_content` flag:

```rust
// full_content is now derived:
let full_content = options.detail == DetailLevel::L2;
```

### Fallback When LLM Metadata Missing

If `llm_summary` is `None` (metadata not yet generated):
- **L0**: Falls back to `title` + first 100 chars of content
- **L1**: Falls back to first 500 chars of content as snippet
- **L2**: Always works (raw content)

This is handled in `project()` with simple fallback logic.

### MCP Tool Parameter

All search tools gain an optional `detail` parameter:

```json
{
  "name": "detail",
  "type": "string",
  "enum": ["L0", "L1", "L2"],
  "default": "L1",
  "description": "Context detail level. L0=abstract (~100 tokens), L1=overview (~2K tokens), L2=full content."
}
```

The `get` and `multi_get` tools also gain this parameter, replacing the need for `maxLines` in many cases.

---

## Feature 2: Hierarchical/Directory-Aware Retrieval

### Problem

Agentroot indexes documents as flat entries. A search for "authentication" returns individual files but ignores that `src/auth/` contains 5 related files, or that `docs/security/` is a coherent topic cluster. Directory structure carries semantic meaning in codebases that is currently lost.

### Design

Two complementary mechanisms:

1. **Directory index** -- A lightweight table that stores per-directory summaries, enabling directory-level search and browsing.
2. **Structural boost** -- Search results from the same directory as high-scoring results get a relevance boost.

### Data Model: Directory Index

New table (schema migration v10):

```sql
CREATE TABLE IF NOT EXISTS directories (
    path TEXT PRIMARY KEY,              -- relative path from collection root (e.g., "src/auth")
    collection TEXT NOT NULL,           -- which collection this belongs to
    depth INTEGER NOT NULL,             -- directory depth (0 = root, 1 = first level, etc.)
    file_count INTEGER NOT NULL DEFAULT 0,
    child_dir_count INTEGER NOT NULL DEFAULT 0,
    summary TEXT,                       -- LLM-generated or auto-derived directory summary
    dominant_language TEXT,             -- most common file extension
    dominant_category TEXT,             -- most common llm_category among children
    concepts TEXT,                      -- JSON array: union of child document concepts
    updated_at TEXT NOT NULL
);

CREATE INDEX idx_directories_collection ON directories(collection);
CREATE INDEX idx_directories_depth ON directories(depth);
```

### Directory Summary Generation

Summaries are generated bottom-up after document indexing:

1. For each directory, collect all child document `llm_summary` values.
2. If LLM is available: send the concatenated summaries (truncated) to LLM with prompt "Summarize what this directory contains in 1-2 sentences."
3. If LLM is unavailable: concatenate the first sentence of each child's `llm_summary`, capped at 200 chars.
4. `dominant_language`: most common file extension (by count).
5. `dominant_category`: most common `llm_category` (by count).
6. `concepts`: union of all child `llm_concepts`, deduplicated.

### Directory FTS Index

```sql
CREATE VIRTUAL TABLE IF NOT EXISTS directories_fts USING fts5(
    path,
    summary,
    concepts,
    tokenize='porter unicode61'
);
```

With standard insert/update/delete triggers.

### Implementation: Directory Indexing

New module `db/directories.rs`:

```rust
pub struct DirectoryInfo {
    pub path: String,
    pub collection: String,
    pub depth: usize,
    pub file_count: usize,
    pub child_dir_count: usize,
    pub summary: Option<String>,
    pub dominant_language: Option<String>,
    pub dominant_category: Option<String>,
    pub concepts: Vec<String>,
    pub updated_at: String,
}
```

Methods on `Database`:
- `upsert_directory(info: &DirectoryInfo) -> Result<()>`
- `get_directory(collection: &str, path: &str) -> Result<Option<DirectoryInfo>>`
- `list_directories(collection: &str, parent: Option<&str>) -> Result<Vec<DirectoryInfo>>`
- `get_sibling_files(collection: &str, dir_path: &str) -> Result<Vec<Document>>`
- `rebuild_directory_index(collection: &str) -> Result<usize>` -- scans all documents, builds directory tree

### Implementation: Structural Boost in Search

In `search/vector.rs` and `search/bm25.rs`, after initial scoring:

```rust
fn apply_directory_boost(results: &mut [SearchResult]) {
    if results.len() < 2 {
        return;
    }

    // Collect directories of top-3 results
    let top_dirs: Vec<String> = results.iter()
        .take(3)
        .filter_map(|r| parent_dir(&r.filepath))
        .collect();

    // Boost results sharing a directory with top results
    for result in results.iter_mut().skip(3) {
        if let Some(dir) = parent_dir(&result.filepath) {
            if top_dirs.contains(&dir) {
                result.score *= 1.15; // 15% boost for directory co-location
            }
        }
    }
}

fn parent_dir(filepath: &str) -> Option<String> {
    filepath.rsplit_once('/').map(|(dir, _)| dir.to_string())
}
```

This is applied after RRF fusion in hybrid search, and after scoring in BM25/vector search.

### MCP Tools

Two new tools:

**browse_directory** -- Navigate the directory tree:

```json
{
  "name": "browse_directory",
  "description": "Browse the directory structure of a collection. Returns directory summaries, file counts, and dominant topics.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "collection": { "type": "string", "description": "Collection name" },
      "path": { "type": "string", "description": "Directory path to browse (empty for root)" },
      "depth": { "type": "integer", "default": 1, "description": "How many levels deep to list (1=immediate children)" }
    },
    "required": ["collection"]
  }
}
```

**search_directories** -- Search for directories by topic:

```json
{
  "name": "search_directories",
  "description": "Search for directories by topic or concept. Returns directory-level results with summaries and file counts.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "query": { "type": "string", "description": "Search query" },
      "collection": { "type": "string", "description": "Filter by collection" },
      "limit": { "type": "integer", "default": 10 }
    },
    "required": ["query"]
  }
}
```

---

## Feature 3: Session Memory

### Problem

Each MCP request is independent. An agent searching for "authentication" then asking "what about the JWT implementation?" has no continuity. The MCP server cannot deduplicate previously returned results, boost follow-up relevance, or track what the agent has already seen.

### Design

A lightweight session layer stored in SQLite. Sessions are created on-demand, expire after inactivity, and store three things:

1. **Query log** -- What queries were executed and what results were returned.
2. **Seen documents** -- Which documents/chunks the agent has already retrieved (for dedup/boost).
3. **Context hints** -- Key-value pairs the agent can store (e.g., "current_topic=authentication").

### Data Model

New tables (schema migration v10):

```sql
CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY,                -- UUID v4
    created_at TEXT NOT NULL,
    last_active_at TEXT NOT NULL,
    ttl_seconds INTEGER NOT NULL DEFAULT 3600,  -- 1 hour default
    context TEXT                         -- JSON object for agent-set key-value pairs
);

CREATE TABLE IF NOT EXISTS session_queries (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    query TEXT NOT NULL,
    result_count INTEGER NOT NULL,
    top_results TEXT,                    -- JSON array of top-5 document hashes
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS session_seen (
    session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    document_hash TEXT NOT NULL,
    chunk_hash TEXT,                     -- NULL for document-level views
    detail_level TEXT NOT NULL DEFAULT 'L1',  -- What tier was retrieved
    seen_at TEXT NOT NULL,
    PRIMARY KEY (session_id, document_hash, chunk_hash)
);

CREATE INDEX idx_session_queries_session ON session_queries(session_id);
CREATE INDEX idx_session_seen_session ON session_seen(session_id);
CREATE INDEX idx_sessions_last_active ON sessions(last_active_at);
```

### Implementation: Session Manager

New module `db/sessions.rs`:

```rust
pub struct SessionInfo {
    pub id: String,
    pub created_at: String,
    pub last_active_at: String,
    pub ttl_seconds: i64,
    pub context: HashMap<String, String>,
}

pub struct SessionQuery {
    pub query: String,
    pub result_count: usize,
    pub top_results: Vec<String>,  // document hashes
    pub created_at: String,
}
```

Methods on `Database`:
- `create_session(ttl_seconds: Option<i64>) -> Result<String>` -- Returns session ID (UUID)
- `get_session(session_id: &str) -> Result<Option<SessionInfo>>`
- `touch_session(session_id: &str) -> Result<()>` -- Updates `last_active_at`
- `set_session_context(session_id: &str, key: &str, value: &str) -> Result<()>`
- `get_session_context(session_id: &str) -> Result<HashMap<String, String>>`
- `log_session_query(session_id: &str, query: &str, results: &[SearchResult]) -> Result<()>`
- `get_session_queries(session_id: &str) -> Result<Vec<SessionQuery>>`
- `mark_seen(session_id: &str, doc_hash: &str, chunk_hash: Option<&str>, detail: DetailLevel) -> Result<()>`
- `get_seen_hashes(session_id: &str) -> Result<HashSet<String>>`
- `cleanup_expired_sessions() -> Result<usize>` -- Deletes sessions past TTL
- `delete_session(session_id: &str) -> Result<()>`

### Session-Aware Search

When a `session_id` is provided in search options, the search pipeline applies two modifications:

1. **Seen deduplication**: Results already seen at the same or higher detail level are demoted (score *= 0.3) rather than removed, so the agent can still find them if nothing new matches.

2. **Context-aware boosting**: If `session_context` contains a `topic` key, it is appended to the query as additional search terms with reduced weight.

```rust
// In SearchOptions:
pub struct SearchOptions {
    // ... existing fields ...
    pub detail: DetailLevel,
    pub session_id: Option<String>,  // New field
}
```

The session-aware logic is applied as a post-processing step, not embedded in individual search methods:

```rust
pub fn apply_session_awareness(
    db: &Database,
    results: &mut Vec<SearchResult>,
    session_id: &str,
) -> Result<()> {
    let seen = db.get_seen_hashes(session_id)?;

    for result in results.iter_mut() {
        let hash = result.chunk_hash.as_deref().unwrap_or(&result.hash);
        if seen.contains(hash) {
            result.score *= 0.3; // Demote, don't remove
        }
    }

    // Re-sort by score
    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    Ok(())
}
```

### MCP Integration

Sessions are opt-in. MCP tools gain an optional `session_id` parameter. The MCP server adds three session management tools:

**session_start**:
```json
{
  "name": "session_start",
  "description": "Start a new search session for multi-turn context tracking. Returns a session ID to pass to subsequent search calls.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "ttl_seconds": { "type": "integer", "default": 3600, "description": "Session timeout in seconds" }
    }
  }
}
```

**session_context**:
```json
{
  "name": "session_context",
  "description": "Get or set session context. When called with key/value, stores context. When called with only session_id, returns all context and query history.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "session_id": { "type": "string", "description": "Session ID from session_start" },
      "key": { "type": "string", "description": "Context key to set (omit to read all)" },
      "value": { "type": "string", "description": "Context value to set" }
    },
    "required": ["session_id"]
  }
}
```

**session_end**:
```json
{
  "name": "session_end",
  "description": "End a search session and clean up resources.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "session_id": { "type": "string", "description": "Session ID to end" }
    },
    "required": ["session_id"]
  }
}
```

All existing search tools gain an optional `session_id` string parameter. When provided:
1. The session's `last_active_at` is updated.
2. Results are processed through `apply_session_awareness`.
3. Returned results are logged to `session_queries`.
4. Retrieved document/chunk hashes are added to `session_seen`.

---

## Feature 4: Agentic Interface Extensions

### Problem

Agentroot serves AI agents but doesn't optimize for agentic workflows: multi-step reasoning, tool chaining, progressive refinement, and autonomous exploration. The current tools are designed for single-shot queries.

### Design

Four extensions that make Agentroot a first-class agentic backend:

### 4.1 Suggest Next Actions

After returning search results, provide the agent with suggested follow-up actions based on what was found:

```rust
pub struct SearchSuggestions {
    pub related_directories: Vec<String>,     // Directories containing related files
    pub related_concepts: Vec<String>,         // Concepts from glossary linked to results
    pub refinement_queries: Vec<String>,       // Suggested refined queries
    pub unseen_related: usize,                 // Count of related docs not yet seen (session-aware)
}
```

This is computed as a lightweight post-processing step and included in structured search results:

```rust
pub fn compute_suggestions(
    db: &Database,
    results: &[SearchResult],
    query: &str,
    session_id: Option<&str>,
) -> Result<SearchSuggestions> {
    // 1. Collect unique parent directories from results
    let related_directories = results.iter()
        .filter_map(|r| parent_dir(&r.filepath))
        .collect::<HashSet<_>>()
        .into_iter()
        .take(5)
        .collect();

    // 2. Collect concepts from result metadata
    let related_concepts = results.iter()
        .filter_map(|r| r.llm_keywords.as_ref())
        .flatten()
        .collect::<HashSet<_>>()
        .into_iter()
        .take(10)
        .collect();

    // 3. Generate refinement queries from top result concepts
    let refinement_queries = generate_refinements(query, &related_concepts);

    // 4. Count unseen related (session-aware)
    let unseen_related = if let Some(sid) = session_id {
        count_unseen_in_directories(db, sid, &related_directories)?
    } else {
        0
    };

    Ok(SearchSuggestions { related_directories, related_concepts, refinement_queries, unseen_related })
}
```

Refinement queries are generated without LLM -- they combine the original query with top concepts:
```rust
fn generate_refinements(query: &str, concepts: &[String]) -> Vec<String> {
    concepts.iter()
        .take(3)
        .map(|c| format!("{} {}", query, c))
        .collect()
}
```

### 4.2 Batch Operations

Agents often need to search for multiple things in parallel. A batch tool avoids N round-trips:

**batch_search** MCP tool:
```json
{
  "name": "batch_search",
  "description": "Execute multiple search queries in a single call. Useful for parallel exploration of related topics.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "queries": {
        "type": "array",
        "items": {
          "type": "object",
          "properties": {
            "query": { "type": "string" },
            "limit": { "type": "integer", "default": 5 },
            "detail": { "type": "string", "enum": ["L0", "L1", "L2"], "default": "L0" }
          },
          "required": ["query"]
        },
        "maxItems": 10,
        "description": "List of queries to execute (max 10)"
      },
      "session_id": { "type": "string" },
      "collection": { "type": "string" },
      "deduplicate": { "type": "boolean", "default": true, "description": "Remove duplicate results across queries" }
    },
    "required": ["queries"]
  }
}
```

Implementation: Iterate over queries, run BM25 for each, deduplicate across results by hash, return grouped by query.

### 4.3 Exploration Mode

An `explore` tool for agents to systematically explore a topic area:

```json
{
  "name": "explore",
  "description": "Explore a topic area starting from a query. Returns L0 abstracts of relevant documents and directories, organized by relevance clusters. Designed for agents to build a mental map before deep-diving.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "query": { "type": "string", "description": "Topic to explore" },
      "collection": { "type": "string" },
      "session_id": { "type": "string" },
      "max_results": { "type": "integer", "default": 20 }
    },
    "required": ["query"]
  }
}
```

Implementation:
1. Run hybrid search with `detail: L0` and `limit: max_results`.
2. Search directories matching the query.
3. Group document results by parent directory.
4. Return: directory summaries + grouped L0 abstracts + suggestions.

This gives agents a "map" of the codebase for a topic before they commit to reading specific files.

### 4.4 Progressive Retrieval Pattern

The combination of tiered loading + sessions enables a natural progressive retrieval pattern:

```
Agent workflow:
1. explore("authentication") -> L0 abstracts of 20 docs + directory summaries
2. search("JWT token validation", detail=L1, session_id=X) -> overviews of top results
3. get(file="src/auth/jwt.rs", detail=L2, session_id=X) -> full content of chosen file
4. search("JWT refresh token", session_id=X) -> results demote already-seen docs
```

No special implementation needed -- this emerges from the combination of L0/L1/L2 + session memory.

---

## Database Migrations

All new tables are added in a single schema migration (v10):

```sql
-- Migration v10: Agentic extensions

-- Directory index
CREATE TABLE IF NOT EXISTS directories (
    path TEXT PRIMARY KEY,
    collection TEXT NOT NULL,
    depth INTEGER NOT NULL,
    file_count INTEGER NOT NULL DEFAULT 0,
    child_dir_count INTEGER NOT NULL DEFAULT 0,
    summary TEXT,
    dominant_language TEXT,
    dominant_category TEXT,
    concepts TEXT,
    updated_at TEXT NOT NULL
);
CREATE INDEX idx_directories_collection ON directories(collection);
CREATE INDEX idx_directories_depth ON directories(depth);

CREATE VIRTUAL TABLE IF NOT EXISTS directories_fts USING fts5(
    path,
    summary,
    concepts,
    tokenize='porter unicode61'
);

-- Directory FTS triggers
CREATE TRIGGER directories_ai AFTER INSERT ON directories BEGIN
    INSERT INTO directories_fts(rowid, path, summary, concepts)
    VALUES (new.rowid, new.path, COALESCE(new.summary, ''), COALESCE(new.concepts, ''));
END;
CREATE TRIGGER directories_au AFTER UPDATE ON directories BEGIN
    DELETE FROM directories_fts WHERE rowid = old.rowid;
    INSERT INTO directories_fts(rowid, path, summary, concepts)
    VALUES (new.rowid, new.path, COALESCE(new.summary, ''), COALESCE(new.concepts, ''));
END;
CREATE TRIGGER directories_ad AFTER DELETE ON directories BEGIN
    DELETE FROM directories_fts WHERE rowid = old.rowid;
END;

-- Session tables
CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY,
    created_at TEXT NOT NULL,
    last_active_at TEXT NOT NULL,
    ttl_seconds INTEGER NOT NULL DEFAULT 3600,
    context TEXT
);

CREATE TABLE IF NOT EXISTS session_queries (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    query TEXT NOT NULL,
    result_count INTEGER NOT NULL,
    top_results TEXT,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS session_seen (
    session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    document_hash TEXT NOT NULL,
    chunk_hash TEXT NOT NULL DEFAULT '',
    detail_level TEXT NOT NULL DEFAULT 'L1',
    seen_at TEXT NOT NULL,
    PRIMARY KEY (session_id, document_hash, chunk_hash)
);

CREATE INDEX idx_session_queries_session ON session_queries(session_id);
CREATE INDEX idx_session_seen_session ON session_seen(session_id);
CREATE INDEX idx_sessions_last_active ON sessions(last_active_at);
```

---

## MCP Tool Extensions

### Summary of New Tools

| Tool | Purpose | Category |
|------|---------|----------|
| `browse_directory` | Navigate directory tree with summaries | Hierarchical retrieval |
| `search_directories` | Search directories by topic | Hierarchical retrieval |
| `session_start` | Create a new session | Session memory |
| `session_context` | Read/write session context | Session memory |
| `session_end` | End and cleanup a session | Session memory |
| `batch_search` | Multiple queries in one call | Agentic |
| `explore` | Topic exploration with L0 abstracts | Agentic |

### Modified Existing Tools

All 4 search tools (`search`, `vsearch`, `query`, `smart_search`) gain:
- `detail` parameter (string enum: "L0", "L1", "L2", default "L1")
- `session_id` parameter (optional string)
- `suggestions` field in structured response (when session_id provided)

The `get` and `multi_get` tools gain:
- `detail` parameter (replaces `maxLines` for most use cases; `maxLines` still supported for backward compatibility)
- `session_id` parameter (marks documents as seen)

The `search_chunks`, `get_chunk`, `navigate_chunks` tools gain:
- `detail` parameter
- `session_id` parameter

Total tool count: 16 existing + 7 new = **23 tools**.

---

## Implementation Order

The features build on each other. Recommended implementation sequence:

### Phase 1: Tiered Context Loading

1. Add `DetailLevel` enum to `search/mod.rs`
2. Add `detail` field to `SearchOptions` (default L1)
3. Implement `SearchResult::project()` method
4. Update BM25 search to respect `detail` (L2 loads body, L0 truncates summary)
5. Update vector search to respect `detail`
6. Update hybrid/unified/smart/orchestrated search to pass through `detail`
7. Add `detail` parameter to all MCP search tools
8. Add `detail` parameter to MCP `get`/`multi_get` tools
9. Tests: verify L0/L1/L2 produce correct field populations

**Dependencies**: None. Pure additive.

### Phase 2: Directory Index

1. Schema migration v10 (directories tables only -- sessions added later in same migration)
2. Implement `db/directories.rs` (DirectoryInfo, CRUD methods)
3. Implement `rebuild_directory_index()` -- scans documents, builds directory entries
4. Implement directory summary generation (with/without LLM)
5. Wire directory rebuild into `collection_update` and `collection_add` CLI commands
6. Implement `apply_directory_boost()` in search post-processing
7. Add `browse_directory` MCP tool
8. Add `search_directories` MCP tool
9. Tests: directory CRUD, boost behavior, MCP tool responses

**Dependencies**: None. Pure additive.

### Phase 3: Session Memory

1. Schema migration v10 (sessions tables -- combined with phase 2 migration)
2. Implement `db/sessions.rs` (SessionInfo, CRUD methods)
3. Implement `apply_session_awareness()` post-processing
4. Add `session_id` to `SearchOptions`
5. Wire session logging into MCP search tool handlers
6. Wire session `mark_seen` into MCP `get`/`multi_get` handlers
7. Add `session_start`, `session_context`, `session_end` MCP tools
8. Implement `cleanup_expired_sessions()` (called on `session_start`)
9. Tests: session lifecycle, seen dedup, context storage, TTL cleanup

**Dependencies**: Benefits from Phase 1 (`detail_level` stored in `session_seen`).

### Phase 4: Agentic Extensions

1. Implement `compute_suggestions()` function
2. Wire suggestions into MCP search responses (when `session_id` present)
3. Implement `batch_search` MCP tool
4. Implement `explore` MCP tool
5. Tests: suggestions accuracy, batch dedup, explore output format

**Dependencies**: Requires Phase 1 (tiered loading for explore) and Phase 3 (session for suggestions).

---

## File Impact Summary

### New Files

| File | Purpose |
|------|---------|
| `crates/agentroot-core/src/db/directories.rs` | Directory index CRUD |
| `crates/agentroot-core/src/db/sessions.rs` | Session management |
| `crates/agentroot-core/src/search/tiered.rs` | `DetailLevel` enum and `project()` logic |
| `crates/agentroot-core/src/search/directory_boost.rs` | Structural boost post-processing |
| `crates/agentroot-core/src/search/session_aware.rs` | Session-aware dedup/boost |
| `crates/agentroot-core/src/search/suggestions.rs` | Suggestion computation |

### Modified Files

| File | Changes |
|------|---------|
| `crates/agentroot-core/src/db/schema.rs` | Migration v10 |
| `crates/agentroot-core/src/db/mod.rs` | Export new modules |
| `crates/agentroot-core/src/search/mod.rs` | Add `DetailLevel` to `SearchOptions`, export new modules |
| `crates/agentroot-core/src/search/bm25.rs` | Respect `detail` level |
| `crates/agentroot-core/src/search/vector.rs` | Respect `detail` level |
| `crates/agentroot-core/src/search/hybrid.rs` | Apply directory boost, pass through detail |
| `crates/agentroot-core/src/search/unified.rs` | Pass through detail |
| `crates/agentroot-core/src/search/smart.rs` | Pass through detail |
| `crates/agentroot-core/src/search/orchestrated.rs` | Pass through detail |
| `crates/agentroot-mcp/src/server.rs` | Register 7 new tools, add session state |
| `crates/agentroot-mcp/src/tools.rs` | Implement 7 new tool handlers, add detail/session params to existing |
| `crates/agentroot-cli/src/commands/search.rs` | Add `--detail` flag |

### Unchanged

All provider modules, AST chunker, LLM traits, graph modules, config modules remain untouched.
