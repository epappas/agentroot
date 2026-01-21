# Performance Tuning Guide

This guide covers performance optimization strategies for Agentroot when working with large codebases (10K+ files).

## Table of Contents

- [Indexing Performance](#indexing-performance)
- [Search Performance](#search-performance)
- [Memory Optimization](#memory-optimization)
- [Database Tuning](#database-tuning)
- [Benchmarking](#benchmarking)
- [Large Codebase Best Practices](#large-codebase-best-practices)

## Indexing Performance

### Chunking Configuration

Agentroot uses semantic chunking with the following defaults:

```rust
// Located in: crates/agentroot-core/src/index/chunker.rs
pub const CHUNK_SIZE_TOKENS: usize = 800;
pub const CHUNK_OVERLAP_TOKENS: usize = 120;
pub const CHUNK_SIZE_CHARS: usize = 3200;
pub const CHUNK_OVERLAP_CHARS: usize = 480;
```

**For Large Files** (>10KB):
- Increase `CHUNK_SIZE_TOKENS` to 1000 for better context
- Reduce `CHUNK_OVERLAP_TOKENS` to 100 to speed up processing

**For Many Small Files** (<1KB):
- Decrease `CHUNK_SIZE_TOKENS` to 500 for finer granularity
- Keep overlap at default for better search recall

### AST Parsing Overhead

AST-aware chunking has overhead (~1-5ms per file). For large codebases:

**Option 1: Selective AST Chunking**
```bash
# Use AST for core code
agentroot collection add ./src --mask '**/*.rs' --name core

# Use character chunking for docs
agentroot collection add ./docs --mask '**/*.md' --name docs
```

**Option 2: Parallel Processing**
```rust
// Already implemented in scanner.rs
// Processes files in parallel using walkdir
```

### Cache Hit Rates

Typical cache performance:
- **Initial indexing**: 0% cache hits (all chunks computed)
- **Minor edits**: 90-95% cache hits
- **Feature additions**: 80-90% cache hits
- **Major refactor**: 60-80% cache hits

**Impact**: Re-indexing 10,000 files with 80% cache hit rate:
- Without cache: ~30 minutes
- With cache: ~6 minutes (5x faster)

## Search Performance

### BM25 Full-Text Search

**Performance**: <10ms for typical queries on 10K documents

**Optimization Tips**:
1. Use specific keywords over broad terms
2. Limit results with `--limit` flag
3. Use collection filters to narrow scope

```bash
# Slow: searches all collections
agentroot search "function"

# Fast: searches specific collection
agentroot search "function" --collection myproject
```

### Vector Similarity Search

**Performance**: ~100ms for 10K chunks

**Optimization Tips**:
1. **Reduce embedding dimensions** (requires model change)
2. **Use min-score threshold** to filter low-quality matches
3. **Enable provider filtering** for targeted search

```bash
# Filter by provider for faster search
agentroot vsearch "authentication" --provider file --min-score 0.5
```

### Hybrid Search

**Performance**: ~150ms (combines BM25 + Vector + Reranking)

**Cost Breakdown**:
- BM25: ~10ms
- Vector search: ~100ms
- RRF fusion: ~5ms
- Reranking (if enabled): ~35ms

**Tuning Constants** (in `crates/agentroot-core/src/search/hybrid.rs`):
```rust
const RRF_K: f64 = 60.0;               // Lower = more emphasis on top results
const MAX_RERANK_DOCS: usize = 40;      // Reduce to 20 for faster reranking
const STRONG_SIGNAL_SCORE: f64 = 0.85;  // Increase to 0.9 for stricter filtering
```

## Memory Optimization

### Embedding Cache

**Memory Usage**:
- Per chunk: ~1.5KB (embedding) + metadata
- 10K chunks: ~15MB memory
- 100K chunks: ~150MB memory

**Configuration**:
```sql
-- Vacuum database to reclaim space
agentroot cleanup

-- Check database size
ls -lh ~/.cache/agentroot/index.sqlite
```

### SQLite Pragma Settings

For large databases, configure SQLite for performance:

```sql
-- Located in: crates/agentroot-core/src/db/schema.rs
PRAGMA journal_mode = WAL;         -- Write-Ahead Logging for concurrency
PRAGMA synchronous = NORMAL;       -- Balance durability and speed
PRAGMA cache_size = -64000;        -- 64MB cache
PRAGMA temp_store = MEMORY;        -- Use RAM for temp tables
```

## Database Tuning

### Index Optimization

**FTS5 Index Size**:
- Documents table: ~500 bytes per document
- FTS5 index: ~2-5x document size
- Total for 10K docs: ~15-30MB

**Optimize Index**:
```bash
# Run after major updates
agentroot cleanup
```

### Content-Addressable Storage

Agentroot uses SHA-256 hashing for deduplication:

**Benefits**:
- Duplicate content stored once
- Incremental updates only process changed files
- Reduced disk I/O

**Trade-offs**:
- Hash computation: ~1ms per document
- Worth it for >1000 documents

## Benchmarking

### Running Benchmarks Locally

```bash
# Run all benchmarks
cargo bench

# Run specific benchmark
cargo bench --bench indexing

# Save baseline
cargo bench --bench indexing -- --save-baseline main

# Compare against baseline
cargo bench --bench indexing -- --baseline main
```

### Benchmark Script

```bash
# Compare current performance against main branch
./scripts/bench-compare.sh main
```

### Interpreting Results

```
scan_1000_files         time:   [45.234 ms 45.891 ms 46.612 ms]
                        change: [-2.3421% -0.9234% +0.5123%]
```

- **time**: Current performance (min, avg, max)
- **change**: Percentage change from baseline (negative = faster)
- **Regression threshold**: >5% slower indicates potential issue

### CI/CD Integration

Benchmarks run automatically on:
- Push to `master` branch
- Pull requests
- Weekly schedule (Sundays at midnight)

**Access results**:
1. Go to GitHub Actions tab
2. Click "Benchmark Performance" workflow
3. Download artifacts for detailed graphs

## Large Codebase Best Practices

### For 10K+ Files

**1. Use Pattern-Based Exclusions**
```bash
agentroot collection add ./repo \
  --mask '**/*.rs' \
  --exclude '**/target/**' \
  --exclude '**/node_modules/**'
```

**2. Split into Multiple Collections**
```bash
# Core codebase
agentroot collection add ./src --name core

# Documentation
agentroot collection add ./docs --name docs

# Tests (optional, separate)
agentroot collection add ./tests --name tests
```

**3. Incremental Updates**
```bash
# First run: indexes everything
agentroot update

# Subsequent runs: only changed files (much faster)
agentroot update
```

**4. Selective Embedding**
```bash
# Embed only core code for semantic search
agentroot embed --collection core

# Use BM25 for documentation (no embeddings needed)
agentroot search "configuration" --collection docs
```

### For 100K+ Files

**Additional Optimizations**:

1. **Database Sharding**: Create multiple databases per project/module
```bash
# Frontend database
AGENTROOT_DB=~/.cache/agentroot/frontend.sqlite agentroot collection add ./frontend

# Backend database
AGENTROOT_DB=~/.cache/agentroot/backend.sqlite agentroot collection add ./backend
```

2. **Batch Processing**: Process files in batches during off-peak hours
```bash
# Schedule large updates during low-activity periods
0 2 * * * agentroot update && agentroot embed
```

3. **Provider Filtering**: Index different sources in separate collections
```bash
# Local files
agentroot collection add ./local --provider file --name local

# GitHub repos
agentroot collection add https://github.com/org/repo --provider github --name remote

# Search specific provider
agentroot search "feature" --provider file
```

## Performance Metrics

### Expected Throughput

| Operation | Small (1K files) | Medium (10K files) | Large (100K files) |
|-----------|------------------|--------------------|--------------------|
| **Scanning** | 100ms | 1s | 10s |
| **AST Parsing** | 1-5s | 10-50s | 100-500s |
| **Embedding** | 20s | 200s | 2000s |
| **BM25 Search** | <10ms | <10ms | <50ms |
| **Vector Search** | 10ms | 100ms | 1s |
| **Hybrid Search** | 15ms | 150ms | 1.5s |

### Memory Usage

| Database Size | RAM Usage | Disk Usage |
|---------------|-----------|------------|
| 1K documents | ~5MB | ~15MB |
| 10K documents | ~50MB | ~150MB |
| 100K documents | ~500MB | ~1.5GB |

## Troubleshooting Performance Issues

### Slow Indexing

**Symptom**: Update takes >1 minute for 1000 files

**Solutions**:
1. Check file patterns exclude unnecessary directories
2. Verify disk I/O is not bottleneck (use `iostat`)
3. Ensure cache directory is on fast storage (SSD preferred)
4. Reduce chunk overlap for faster processing

### Slow Search

**Symptom**: Queries take >1 second

**Solutions**:
1. Use `--min-score` to reduce result set
2. Add `--collection` filter to search subset
3. Run `agentroot cleanup` to optimize database
4. Check SQLite pragma settings

### High Memory Usage

**Symptom**: Process uses >1GB RAM

**Solutions**:
1. Split large databases into multiple collections
2. Clear embedding cache periodically
3. Reduce `MAX_RERANK_DOCS` in hybrid search
4. Use streaming API for large result sets

## Advanced Configuration

### Environment Variables

```bash
# Override database path
export AGENTROOT_DB=/fast/ssd/agentroot.sqlite

# Override models directory
export AGENTROOT_MODELS=/fast/ssd/models

# Set log level for debugging
export RUST_LOG=debug
```

### Profiling

```bash
# Profile with cargo flamegraph
cargo install flamegraph
cargo flamegraph --bench indexing

# Profile with perf (Linux)
perf record cargo bench --bench search
perf report
```

## Benchmark Baseline Storage

### Storing Baselines

```bash
# Create baseline for current commit
cargo bench --bench indexing --bench search -- --save-baseline $(git rev-parse --short HEAD)

# Compare feature branch against main
git checkout feature-branch
cargo bench --bench indexing -- --baseline main
```

### CI Baseline Management

Baselines are automatically stored in GitHub Actions cache:
- **Key**: `criterion-{branch}-{sha}`
- **Retention**: 30 days
- **Size limit**: 400MB per cache

## Monitoring

### Metrics to Track

1. **Indexing time** (per file, per collection)
2. **Cache hit rate** (percentage)
3. **Search latency** (p50, p95, p99)
4. **Database size** (growth rate)
5. **Memory usage** (peak, average)

### Example Monitoring

```bash
# Check current performance
time agentroot update
time agentroot search "test"

# Monitor database size
du -sh ~/.cache/agentroot/index.sqlite

# Check cache effectiveness
sqlite3 ~/.cache/agentroot/index.sqlite "SELECT COUNT(*) FROM chunk_embeddings;"
```

## Further Reading

- [Architecture Documentation](architecture.md) - System design
- [Semantic Chunking](semantic-chunking.md) - Chunking algorithm details
- [Embedding Cache](embedding-cache.md) - Cache implementation
- [Provider System](providers.md) - Multi-source indexing

## Contributing

Found a performance issue? Please [open an issue](https://github.com/epappas/agentroot/issues) with:
1. Benchmark results showing regression
2. System information (OS, CPU, RAM, disk type)
3. Dataset size (number of files, total size)
4. Reproduction steps
