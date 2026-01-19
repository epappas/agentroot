# Smart Embedding Cache

Agentroot uses content-addressable hashing to cache embeddings at the chunk level, achieving 80-90% cache hit rates when re-indexing after typical code changes.

## The Problem

Embedding computation is expensive:
- GPU/CPU intensive (~10-50ms per chunk)
- Adds up quickly for large codebases (1000s of chunks)
- Re-embedding unchanged content wastes resources

Naive approaches:
- **Document-level caching**: Any change invalidates the entire document
- **No caching**: Re-compute everything every time

## The Solution: Chunk-Level Content Hashing

Each chunk gets a content-addressable hash using blake3:

```rust
pub fn compute_chunk_hash(text: &str, leading: &str, trailing: &str) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(leading.as_bytes());
    hasher.update(text.as_bytes());
    hasher.update(trailing.as_bytes());
    hasher.finalize().to_hex()[..32].to_string()
}
```

### Why Include Context?

The hash includes leading/trailing trivia (comments, docstrings) because:

1. **Docstrings affect semantics**: A function's docstring describes what it does
2. **Comment changes matter**: Updated comments should update the embedding
3. **Context provides meaning**: Surrounding context affects interpretation

```rust
/// Processes user input.          ← Leading trivia (included in hash)
fn process(input: &str) -> String {
    input.trim().to_uppercase()
}                                   // cleanup  ← Trailing trivia (included)
```

## Cache Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    Embedding Pipeline                            │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Document                                                        │
│     │                                                            │
│     ▼                                                            │
│  ┌─────────────┐                                                │
│  │ AST Chunker │  Produces chunks with chunk_hash               │
│  └──────┬──────┘                                                │
│         │                                                        │
│         ▼                                                        │
│  For each chunk:                                                 │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │                                                          │    │
│  │  chunk_hash ──▶ Cache Lookup ──▶ Hit? ──▶ Use cached    │    │
│  │                      │                                   │    │
│  │                      ▼                                   │    │
│  │                    Miss?                                 │    │
│  │                      │                                   │    │
│  │                      ▼                                   │    │
│  │              Queue for Embedding                         │    │
│  │                                                          │    │
│  └─────────────────────────────────────────────────────────┘    │
│         │                                                        │
│         ▼                                                        │
│  ┌─────────────┐                                                │
│  │ Batch Embed │  Compute embeddings for uncached chunks        │
│  └──────┬──────┘                                                │
│         │                                                        │
│         ▼                                                        │
│  ┌─────────────┐                                                │
│  │ Store All   │  Store embeddings + update cache               │
│  └─────────────┘                                                │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

## Database Schema

### chunk_embeddings Table

Global cache keyed by (chunk_hash, model):

```sql
CREATE TABLE chunk_embeddings (
    chunk_hash TEXT NOT NULL,    -- blake3 hash of chunk content
    model TEXT NOT NULL,         -- embedding model name
    embedding BLOB NOT NULL,     -- vector as little-endian f32 bytes
    created_at TEXT NOT NULL,
    PRIMARY KEY (chunk_hash, model)
);
```

### content_vectors Table

Per-document chunk tracking:

```sql
CREATE TABLE content_vectors (
    hash TEXT NOT NULL,          -- document content hash
    seq INTEGER NOT NULL,        -- chunk sequence number
    pos INTEGER NOT NULL,        -- byte position in document
    model TEXT NOT NULL,
    chunk_hash TEXT,             -- references chunk_embeddings
    created_at TEXT NOT NULL,
    PRIMARY KEY (hash, seq)
);
```

### model_metadata Table

Tracks model dimensions for compatibility:

```sql
CREATE TABLE model_metadata (
    model TEXT PRIMARY KEY,
    dimensions INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    last_used_at TEXT NOT NULL
);
```

## Cache Lookup Flow

```rust
pub enum CacheLookupResult {
    Hit(Vec<f32>),      // Cached embedding found
    Miss,               // Need to compute
    ModelMismatch,      // Model dimensions changed
}

impl Database {
    pub fn get_cached_embedding(
        &self,
        chunk_hash: &str,
        model: &str,
        expected_dims: usize
    ) -> Result<CacheLookupResult> {
        // 1. Check model compatibility
        if !self.check_model_compatibility(model, expected_dims)? {
            return Ok(CacheLookupResult::ModelMismatch);
        }

        // 2. Look up by chunk_hash + model
        match self.query_chunk_embedding(chunk_hash, model) {
            Some(embedding) => Ok(CacheLookupResult::Hit(embedding)),
            None => Ok(CacheLookupResult::Miss),
        }
    }
}
```

## Model Compatibility

When the embedding model changes dimensions, all cached embeddings become invalid:

```rust
pub fn check_model_compatibility(&self, model: &str, expected_dims: usize) -> Result<bool> {
    match self.get_model_dimensions(model)? {
        Some(stored_dims) => Ok(stored_dims == expected_dims),
        None => Ok(true),  // New model, compatible by default
    }
}
```

If dimensions change:
1. Return `CacheLookupResult::ModelMismatch` for all lookups
2. All chunks get re-embedded
3. New embeddings stored with correct dimensions

## Embedding Pipeline Integration

```rust
pub async fn embed_documents(
    db: &Database,
    embedder: &dyn Embedder,
    model: &str,
    force: bool,
) -> Result<EmbedStats> {
    let dimensions = embedder.dimensions();

    // Check model compatibility once upfront
    let cache_enabled = !force && db.check_model_compatibility(model, dimensions)?;
    db.register_model(model, dimensions)?;

    for (hash, content, path) in documents {
        let chunks = chunker.chunk(&content, path)?;

        // Partition by cache status
        let (cached, to_compute): (Vec<_>, Vec<_>) = chunks
            .into_iter()
            .map(|chunk| {
                let cached = if cache_enabled {
                    db.get_cached_embedding_fast(&chunk.chunk_hash, model)?
                } else {
                    None
                };
                (chunk, cached)
            })
            .partition(|(_, cached)| cached.is_some());

        // Use cached embeddings directly
        for (chunk, embedding) in cached {
            db.insert_chunk_embedding(hash, chunk, embedding)?;
            stats.cached_chunks += 1;
        }

        // Batch compute uncached embeddings
        for batch in to_compute.chunks(BATCH_SIZE) {
            let embeddings = embedder.embed_batch(texts).await?;
            for (chunk, embedding) in batch.zip(embeddings) {
                db.insert_chunk_embedding(hash, chunk, embedding)?;
                stats.computed_chunks += 1;
            }
        }
    }
}
```

## Cache Hit Scenarios

### Scenario 1: No Changes

```
Before: fn foo() { bar() }
After:  fn foo() { bar() }

Result: 100% cache hit (identical hash)
```

### Scenario 2: Minor Edit

```
Before: fn foo() { bar() }
After:  fn foo() { bar(); baz() }

Result: This chunk misses, others hit
        Overall: ~90% hit rate
```

### Scenario 3: Comment Change

```
Before: /// Old doc
        fn foo() { bar() }

After:  /// New doc
        fn foo() { bar() }

Result: Cache miss (leading trivia changed hash)
```

### Scenario 4: Add New Function

```
Before: fn foo() { }

After:  fn foo() { }
        fn new_func() { }  // Added

Result: foo() hits cache, new_func() misses
        Overall: ~50% hit rate
```

### Scenario 5: Refactor/Rename

```
Before: fn process_data() { ... }
After:  fn transform_input() { ... }

Result: Cache miss (different content)
```

## Expected Cache Hit Rates

| Change Type | Expected Hit Rate |
|-------------|-------------------|
| No changes | 100% |
| Bug fix (1-2 functions) | 90-98% |
| Feature addition | 80-95% |
| Refactoring | 60-80% |
| Major rewrite | 10-40% |
| Force re-embed | 0% |

## Garbage Collection

Orphaned cache entries are cleaned up periodically:

```rust
pub fn cleanup_orphaned_chunk_embeddings(&self) -> Result<usize> {
    self.conn.execute(
        "DELETE FROM chunk_embeddings WHERE chunk_hash NOT IN (
            SELECT DISTINCT chunk_hash FROM content_vectors
            WHERE chunk_hash IS NOT NULL
        )",
        [],
    )
}
```

This removes embeddings for chunks that are no longer referenced by any document.

## Transaction Safety

All cache operations use transactions to prevent partial updates:

```rust
self.conn.execute("BEGIN IMMEDIATE", [])?;
let result = (|| {
    // 1. Insert content_vectors entry
    self.conn.execute(
        "INSERT OR REPLACE INTO content_vectors ...",
        params![...],
    )?;

    // 2. Insert embeddings entry
    self.conn.execute(
        "INSERT OR REPLACE INTO embeddings ...",
        params![...],
    )?;

    // 3. Update chunk_embeddings cache
    self.conn.execute(
        "INSERT OR REPLACE INTO chunk_embeddings ...",
        params![...],
    )?;

    Ok(())
})();

if result.is_ok() {
    self.conn.execute("COMMIT", [])?;
} else {
    let _ = self.conn.execute("ROLLBACK", []);
}
```

## CLI Usage

```bash
# Normal embedding (uses cache)
agentroot embed
# Output: Cached: 850 (85.0%), Computed: 150 (15.0%)

# Force re-embedding (ignores cache)
agentroot embed --force
# Output: Cached: 0 (0.0%), Computed: 1000 (100.0%)

# Verbose output shows per-document stats
agentroot embed -v
```

## Performance Impact

| Operation | Without Cache | With Cache (85% hit) |
|-----------|--------------|---------------------|
| 1000 chunks | ~30s | ~5s |
| 10000 chunks | ~5min | ~45s |
| Re-index after minor edit | ~30s | ~3s |

The cache provides 5-10x speedup for typical incremental updates.

## Debugging

Check cache statistics:

```sql
-- Count cached embeddings by model
SELECT model, COUNT(*) FROM chunk_embeddings GROUP BY model;

-- Find orphaned cache entries
SELECT COUNT(*) FROM chunk_embeddings
WHERE chunk_hash NOT IN (
    SELECT DISTINCT chunk_hash FROM content_vectors
    WHERE chunk_hash IS NOT NULL
);

-- Check model dimensions
SELECT * FROM model_metadata;
```
