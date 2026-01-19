# Performance

Performance characteristics and optimization guide for Agentroot.

## Overview

Agentroot is designed for fast local search across large codebases. Performance depends on:

- Hardware (CPU, RAM, disk I/O)
- Corpus size (number of files, total size)
- Query complexity
- Cache hit rates

## Indexing Performance

### File Scanning

Directory scanning uses `walkdir` with minimal overhead:

- Speed: ~1000-5000 files/second (disk I/O bound)
- Memory: ~10KB per 1000 files in memory at once
- Bottleneck: Disk I/O and filesystem metadata access

Factors affecting scan speed:
- SSD vs HDD: 5-10x difference
- File count: More files = slower (filesystem overhead)
- Network filesystems: 10-100x slower than local

### AST Parsing

tree-sitter parsing for semantic chunking:

- Speed: ~1-5ms per file (depends on file size)
- Memory: Bounded by tree-sitter streaming parser
- Supported languages: Rust, Python, JavaScript, TypeScript, Go

Parsing overhead:
- Small files (<5KB): 0.5-2ms
- Medium files (5-50KB): 2-10ms
- Large files (>50KB): 10-50ms

Fallback character-based chunking (unsupported languages):
- Speed: ~0.1-1ms per file
- No parsing overhead

### Content Hashing

SHA-256 hashing for document deduplication:

- Speed: ~500MB/s on modern CPUs
- Overhead: <1ms per typical file
- Benefit: Eliminates duplicate content

blake3 hashing for chunk cache:

- Speed: ~2GB/s on modern CPUs
- Overhead: <0.5ms per chunk
- Benefit: 80-90% cache hit rates

### Database Operations

SQLite with WAL mode:

- Insert: ~1-2ms per document
- Transaction batch: ~100-500 documents/transaction
- FTS5 index: Built incrementally, ~10-20% overhead

### Embedding Generation

Bottleneck for initial indexing:

- Model: nomic-embed-text-v1.5 (768 dimensions)
- Speed: 50-100 chunks/second (CPU-dependent)
- Batch size: 32 chunks at a time
- Memory: ~1-2GB for model

Factors affecting embedding speed:
- CPU: Modern x86-64 with AVX2 (2-3x faster than old CPUs)
- Batch size: Larger batches = better throughput
- Chunk size: Longer text = more computation

Expected times for full corpus embedding:
- 1,000 chunks: ~10-20 seconds
- 10,000 chunks: ~2-3 minutes
- 100,000 chunks: ~20-30 minutes

### Cache Performance

Content-addressable caching dramatically improves re-indexing:

| Change Type | Cache Hit Rate | Expected Speedup |
|-------------|----------------|------------------|
| No changes | 100% | Instant (cache only) |
| Minor bug fix (1-2 files) | 95-98% | 10-20x faster |
| Feature addition (10-20 files) | 80-90% | 5-10x faster |
| Major refactoring (50+ files) | 60-80% | 2-5x faster |
| Complete rewrite | 0-20% | No speedup |

Re-indexing with cache:
- Cache lookup: <0.1ms per chunk
- Only changed chunks re-embedded
- Typical edit: 90% cache hit = 10x faster

### Provider Performance

Different providers have different performance characteristics:

#### FileProvider (Local Files)

- **Scanning**: ~1000-5000 files/second (disk I/O bound)
- **Latency**: <1ms per file (local filesystem)
- **Bottleneck**: Disk I/O, filesystem metadata
- **Scalability**: Handles 100K+ files easily
- **Cache-friendly**: High hit rates on re-index (80-90%)

Best for:
- Local development
- Quick iteration
- Large codebases on SSD

#### GitHubProvider (GitHub API)

- **API Rate Limits**:
  - Without auth: 60 requests/hour
  - With auth: 5,000 requests/hour
- **Latency**: 100-500ms per API call
- **Bottleneck**: Network latency and API rate limits
- **Batch operations**: List files once, fetch individually
- **ETags**: GitHub returns ETags for efficient updates

Performance tips:
- Always use GITHUB_TOKEN for authentication
- Index during off-peak hours if hitting rate limits
- Consider caching locally for frequent updates
- Use specific file patterns to reduce API calls

Example timings (with auth):
- Small repo (<100 files): 2-5 minutes
- Medium repo (100-1K files): 10-30 minutes
- Large repo (>1K files): May hit rate limits

Best for:
- Public documentation
- Reference implementations
- Occasional updates

#### Performance Comparison

| Provider | Speed | Latency | Scalability | Cache Efficiency |
|----------|-------|---------|-------------|------------------|
| FileProvider | ⚡ Very Fast | <1ms | 100K+ files | ✅ Excellent (90%) |
| GitHubProvider | 🐌 Slow | 100-500ms | Limited by API | ⚠️ Good (ETags) |
| Future: URLProvider | 🐢 Moderate | 50-200ms | Varies | ⚠️ Moderate |
| Future: PDFProvider | 🐢 Moderate | 10-100ms | Good | ✅ Good |

#### Optimization Strategies

**For FileProvider**:
- Use SSD for best performance
- Exclude large binary directories (node_modules, target)
- Use specific glob patterns to reduce scanning
- Enable symlink following only when needed

**For GitHubProvider**:
- Set GITHUB_TOKEN environment variable
- Use specific file patterns (**/*.md vs **/*)
- Index once, update periodically
- Consider local caching for frequent access

**For Mixed Sources**:
- Index local files frequently (fast)
- Update GitHub collections periodically (slow)
- Use separate update commands when needed
- Monitor API rate limits

## Search Performance

### BM25 Full-Text Search

Using SQLite FTS5 with Porter stemming:

- Latency: <10ms for typical queries
- Throughput: >100 queries/second
- Bottleneck: Disk I/O for large result sets

Query complexity impact:
- Simple keyword: 1-5ms
- Phrase search: 2-10ms
- Complex boolean: 5-20ms
- Prefix wildcard: 10-50ms

Corpus size impact:
- 1K documents: <5ms
- 10K documents: <10ms
- 100K documents: <20ms
- 1M documents: <50ms (needs testing)

### Vector Similarity Search

Cosine similarity computed in Rust:

- Latency: 10-200ms (depends on corpus size)
- Algorithm: Brute-force cosine similarity
- Memory: Loads embeddings into RAM

Corpus size impact (768-dim embeddings):
- 1K chunks: ~10ms
- 10K chunks: ~50-100ms
- 100K chunks: ~500ms-1s
- 1M chunks: ~5-10s (impractical)

Memory usage:
- Per chunk: 768 dims × 4 bytes = 3KB
- 10K chunks: ~30MB
- 100K chunks: ~300MB

Optimization for large corpora:
- Filter by collection: Reduces search space
- Use approximate nearest neighbor (not implemented)
- Shard by collection/topic

### Hybrid Search

Combines BM25 and vector search:

- Latency: Max of both methods + RRF overhead
- RRF overhead: <1ms
- Typically: 50-150ms total

Strong signal optimization:
- If BM25 top result has score ≥0.85 and gap ≥0.15
- Skip vector search entirely
- Saves ~50-100ms

Expected latencies:
- Small corpus (<1K docs): 20-50ms
- Medium corpus (1-10K docs): 50-150ms
- Large corpus (>10K docs): 150-500ms

## Memory Usage

### Indexing

Memory consumption during indexing:

- Base process: ~10-20MB
- tree-sitter parser: ~5-10MB
- Embedding model: ~1-2GB
- Batch buffer: ~10MB
- Peak usage: ~1.5-2GB

### Search Operations

Memory for search:

- BM25 search: ~50MB (query processing)
- Vector search: ~30MB per 10K chunks (embedding storage)
- Hybrid search: Sum of both

Database connection:
- SQLite: ~10-20MB base
- Page cache: Grows with queries (up to hundreds of MB)

### Database File Size

On-disk storage:

- Documents: ~10KB per file (metadata + hash)
- Content: Deduplicated by SHA-256 hash
- FTS5 index: ~30% of total content size
- Embeddings: ~3KB per chunk (768 × 4 bytes)
- Chunk cache: ~3KB per unique chunk

Example corpus (10,000 files, 5 chunks each):
- Document metadata: ~100MB
- Content (deduplicated): ~200MB
- FTS5 index: ~60MB
- Embeddings: ~150MB (50,000 chunks × 3KB)
- Total: ~510MB

## Optimization Strategies

### For Faster Indexing

1. **Exclude unnecessary files**:
   ```bash
   agentroot collection add ./src --name code \
     --exclude '**/target/**' \
     --exclude '**/node_modules/**' \
     --exclude '**/test/**'
   ```

2. **Use specific file masks**:
   ```bash
   # Only source files
   --mask '**/*.rs' --mask '**/*.py'
   ```

3. **Build release binary**:
   ```bash
   cargo build --release
   # 20-30% faster than debug build
   ```

4. **Use SSD for database**:
   ```bash
   # Place on fast disk
   export AGENTROOT_DB=/ssd/path/index.sqlite
   ```

5. **Leverage cache on re-index**:
   ```bash
   # Incremental updates are much faster
   agentroot update
   agentroot embed  # Only changed chunks re-embedded
   ```

### For Faster Search

1. **Use BM25 for keyword queries**:
   ```bash
   # BM25 is 10x faster than vector search
   agentroot search "function_name"
   ```

2. **Filter by collection**:
   ```bash
   # Reduces search space
   agentroot query "pattern" -c specific-collection
   ```

3. **Set appropriate limits**:
   ```bash
   # Fewer results = faster
   agentroot search "query" -n 10
   ```

4. **Use minimum score thresholds**:
   ```bash
   # Skip low-quality results
   agentroot search "query" --min-score 0.5
   ```

5. **Vacuum database periodically**:
   ```bash
   # Optimize database file
   agentroot cleanup --vacuum
   ```

### For Lower Memory Usage

1. **Filter searches by collection**:
   - Loads fewer embeddings into memory

2. **Close other applications**:
   - Embedding generation needs 1-2GB RAM

3. **Use smaller batch sizes** (not currently configurable):
   - Would reduce peak memory during embedding

4. **Index in stages**:
   ```bash
   # Index collections separately
   agentroot collection add ./src --name src
   agentroot update -c src
   agentroot embed -c src
   ```

### For Smaller Database

1. **Clean up orphaned data**:
   ```bash
   agentroot cleanup
   ```

2. **Vacuum database**:
   ```bash
   agentroot cleanup --vacuum
   # Reclaims unused space
   ```

3. **Remove old collections**:
   ```bash
   agentroot collection remove old-collection
   ```

4. **Exclude large files**:
   ```bash
   # Skip generated files, binaries, etc.
   --exclude '**/dist/**'
   --exclude '**/*.min.js'
   ```

## Benchmarking

### Measuring Indexing Time

```bash
# Time full indexing
time agentroot update
time agentroot embed

# With verbose output
time agentroot embed -v
```

### Measuring Search Latency

```bash
# Install hyperfine for benchmarking
cargo install hyperfine

# Benchmark BM25 search
hyperfine "agentroot search 'error handling'"

# Benchmark vector search
hyperfine "agentroot vsearch 'error handling'"

# Benchmark hybrid search
hyperfine "agentroot query 'error handling'"
```

### Measuring Cache Hit Rate

```bash
# First embedding (no cache)
agentroot embed -f

# Make small edit to one file
# Then re-embed
agentroot embed -v
# Output shows: Cached: X (Y%), Computed: Z (W%)
```

### Profiling

For detailed profiling:

```bash
# Install flamegraph
cargo install flamegraph

# Profile indexing
sudo flamegraph -- agentroot update

# Profile search
sudo flamegraph -- agentroot query "pattern"

# View flamegraph.svg in browser
```

## Scalability Limits

### Tested Configurations

Agentroot has been tested with:
- Up to 100,000 files
- Up to 500,000 chunks
- Database size up to 2GB
- RAM usage up to 2GB (during embedding)

### Known Limitations

1. **Vector search is O(n)**:
   - Brute-force cosine similarity
   - Linear with corpus size
   - Becomes impractical beyond 100K chunks
   - Solution: Filter by collection, or use approximate nearest neighbor (not implemented)

2. **Embeddings in memory**:
   - Vector search loads all embeddings into RAM
   - ~3KB per chunk
   - 100K chunks = ~300MB
   - Solution: Filter by collection to reduce working set

3. **SQLite concurrency**:
   - Single writer at a time
   - Multiple readers OK (WAL mode)
   - Concurrent indexing not supported

4. **Model size**:
   - nomic-embed-text-v1.5: 768 dimensions
   - ~1-2GB RAM during inference
   - Cannot be reduced without changing model

### Recommendations by Scale

**Small (<1K files)**:
- All operations fast (<1s)
- No optimization needed

**Medium (1-10K files)**:
- Indexing: 10-60 seconds
- Search: <100ms
- Exclude test files

**Large (10-100K files)**:
- Indexing: 1-10 minutes
- Search: 100-500ms
- Use collection filters
- Exclude generated files

**Very Large (>100K files)**:
- Consider splitting into multiple databases
- Use collection filters heavily
- May need approximate nearest neighbor for vector search (not implemented)

## Hardware Recommendations

### Minimum Requirements

- CPU: Modern x86-64 (2015+)
- RAM: 4GB (2GB for Agentroot + 2GB for OS)
- Disk: 1GB free space
- OS: Linux, macOS, Windows

### Recommended

- CPU: Modern x86-64 with AVX2 (2-3x faster embedding)
- RAM: 8GB (comfortable headroom)
- Disk: SSD (5-10x faster indexing)
- OS: Linux or macOS (best tested)

### Optimal

- CPU: Modern x86-64 with AVX-512 or ARM with NEON
- RAM: 16GB+ (large corpora)
- Disk: NVMe SSD (minimal I/O wait)
- OS: Linux (best performance)

## Comparison to Alternatives

Performance comparison (approximate, hardware-dependent):

| Tool | Keyword Search | Semantic Search | Indexing | Cache |
|------|---------------|----------------|----------|-------|
| Agentroot | <10ms | ~100ms | ~5min/10K files | 80-90% |
| ripgrep | <10ms | N/A | Instant (no index) | N/A |
| GitHub Search | ~100ms | N/A | N/A (cloud) | N/A |
| Basic vector DB | N/A | ~100ms | ~10min/10K files | No |

Agentroot's advantage:
- Hybrid search for best quality
- Smart cache for fast re-indexing
- Local-first (no network latency)

## Future Improvements

Potential optimizations (not implemented):

1. **Approximate Nearest Neighbor (ANN)**:
   - Replace brute-force with HNSW or FAISS
   - 10-100x faster vector search
   - Slight accuracy trade-off

2. **Parallel indexing**:
   - Multi-threaded file scanning
   - Parallel embedding generation
   - 2-4x faster indexing

3. **Incremental FTS5 updates**:
   - Only rebuild changed documents
   - Faster `update` command

4. **Streaming vector search**:
   - Don't load all embeddings
   - Lower memory usage
   - Slightly slower

5. **GPU acceleration**:
   - Use GPU for embeddings
   - 5-10x faster generation
   - Requires CUDA/Metal support

These may be added in future versions based on demand.
