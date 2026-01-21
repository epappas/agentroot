# End-to-End Workflow Guide

This guide walks you through a complete real-world workflow using AgentRoot, from installation to advanced AI-powered search.

## Table of Contents

1. [Installation & Setup](#installation--setup)
2. [Basic Workflow: Local Files](#basic-workflow-local-files)
3. [Advanced Workflow: Multi-Source Indexing](#advanced-workflow-multi-source-indexing)
4. [AI-Powered Workflow: vLLM Integration](#ai-powered-workflow-vllm-integration)
5. [Daily Usage Patterns](#daily-usage-patterns)
6. [Troubleshooting](#troubleshooting)

---

## Installation & Setup

### Step 1: Build from Source

```bash
# Clone repository
git clone https://github.com/epappas/agentroot
cd agentroot

# Build release binary
cargo build --release

# Install to PATH (optional)
cargo install --path crates/agentroot-cli

# Or use directly
alias agentroot='./target/release/agentroot'
```

### Step 2: Verify Installation

```bash
agentroot --version
# Output: agentroot 0.1.0
```

### Step 3: Check Initial Status

```bash
agentroot status
```

Expected output (first run):
```
Collections:     0
Documents:       0

Embeddings:
  Embedded:      0
  Pending:       0

Metadata:
  Generated:     0
  Pending:       0
```

---

## Basic Workflow: Local Files

This workflow demonstrates indexing and searching a local codebase.

### Scenario: Index Your Rust Project

**Goal**: Make your Rust project searchable with semantic understanding.

#### Step 1: Add Collection

```bash
# Add your project directory as a collection
agentroot collection add ~/projects/my-rust-app \
  --name my-app \
  --mask '**/*.rs'

# Verify collection was added
agentroot collection list
```

Output:
```
Collections:
  my-app
    Path: /home/user/projects/my-rust-app
    Pattern: **/*.rs
    Provider: file
    Documents: 0 (not indexed yet)
```

#### Step 2: Index Files

```bash
# Scan and index all matching files
agentroot update

# Check progress
agentroot status
```

Output:
```
Collections:     1
Documents:       127  ← Files found and indexed

Embeddings:
  Embedded:      0    ← Not generated yet
  Pending:       127

Metadata:
  Generated:     0
  Pending:       127
```

#### Step 3: Generate Embeddings

```bash
# Generate vector embeddings for semantic search
agentroot embed

# This downloads the embedding model on first run (~100MB)
# Progress will be shown
```

Output (first run):
```
📥 Downloading nomic-embed-text-v1.5.Q4_K_M.gguf (100.3 MB)
⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿ 100%

✅ Model loaded
🔄 Embedding 127 documents...
Progress: [████████████████████] 127/127

✅ Embeddings complete (15.2s)
```

#### Step 4: Search!

```bash
# Keyword search (BM25)
agentroot search "error handling"

# Semantic search (understands meaning)
agentroot vsearch "how to handle database errors"

# Hybrid search (best quality - combines both)
agentroot query "async error patterns"
```

Example output:
```
  92% src/database/error.rs #a1b2c3
  Database error handling utilities with retry logic

  87% src/api/handlers.rs #d4e5f6
  HTTP handler with comprehensive error mapping

  81% src/lib.rs #g7h8i9
  Core error types and conversion traits
```

#### Step 5: View Documents

```bash
# Get full document by docid (from search results)
agentroot get "#a1b2c3"

# Get by path
agentroot get src/database/error.rs

# Get multiple files with glob pattern
agentroot multi-get "src/**/*.rs" --max-lines 50
```

### Updating After Code Changes

```bash
# After editing files, re-index
agentroot update

# Re-generate embeddings (only changed files)
agentroot embed

# Smart caching means this is 5-10x faster!
# Only modified files are re-embedded
```

---

## Advanced Workflow: Multi-Source Indexing

This workflow demonstrates indexing content from multiple sources.

### Scenario: Create Unified Knowledge Base

**Goal**: Index local code, documentation, and external resources in one searchable database.

#### Step 1: Add Multiple Collections

```bash
# 1. Local project code
agentroot collection add ~/projects/my-app \
  --name my-app-code \
  --mask '**/*.rs'

# 2. Local documentation
agentroot collection add ~/projects/my-app/docs \
  --name my-app-docs \
  --mask '**/*.md'

# 3. GitHub repository (Rust stdlib docs)
agentroot collection add https://github.com/rust-lang/rust \
  --name rust-docs \
  --mask '**/*.md' \
  --provider github

# 4. PDF manuals
agentroot collection add ~/Documents/manuals \
  --name manuals \
  --mask '**/*.pdf' \
  --provider pdf

# List all collections
agentroot collection list
```

Output:
```
Collections:
  my-app-code    (file)    127 documents
  my-app-docs    (file)    23 documents
  rust-docs      (github)  0 documents (not indexed)
  manuals        (pdf)     0 documents (not indexed)
```

#### Step 2: Index All Collections

```bash
# Index everything at once
agentroot update

# Or index specific collection
agentroot update my-app --collection rust-docs
```

#### Step 3: Generate Embeddings

```bash
# Embed all collections
agentroot embed

# Check status
agentroot status
```

Output:
```
Collections:     4
Documents:       892

Embeddings:
  Embedded:      892
  Pending:       0

Metadata:
  Generated:     0
  Pending:       892
```

#### Step 4: Search Across All Sources

```bash
# Search everything
agentroot query "database connection pooling"

# Search specific collection
agentroot query "async traits" --collection rust-docs

# List what's in a collection
agentroot ls rust-docs
```

---

## AI-Powered Workflow: Basilica Integration

This workflow demonstrates the most advanced features using Basilica's decentralized GPU network.

### Scenario: AI-Enhanced Search with Basilica

**Goal**: Use Basilica's verified GPU compute for query understanding, metadata generation, and intelligent reranking.

#### Prerequisites

You need Basilica endpoints:
1. **Option A (Recommended)**: Sign up at [basilica.ai](https://basilica.ai) for instant access
2. **Option B**: Self-host using [github.com/one-covenant/basilica](https://github.com/one-covenant/basilica)
3. **Option C**: Contact team@basilica.ai for enterprise deployments

Basilica provides both LLM and embedding endpoints on its decentralized Bittensor network.

#### Step 1: Configure Basilica Endpoints

**Option A: Add to shell profile (permanent)**

Add to `~/.bashrc` or `~/.zshrc`:

```bash
# Basilica Configuration
# Get your deployment IDs from https://basilica.ai/deployments
export AGENTROOT_LLM_URL="https://your-id.deployments.basilica.ai"
export AGENTROOT_LLM_MODEL="Qwen/Qwen2.5-7B-Instruct"
export AGENTROOT_EMBEDDING_URL="https://your-id.deployments.basilica.ai"
export AGENTROOT_EMBEDDING_MODEL="intfloat/e5-mistral-7b-instruct"
export AGENTROOT_EMBEDDING_DIMS="4096"
```

Then reload:
```bash
source ~/.bashrc
```

**Option B: Use setup script**

```bash
# Create setup script
cat > ~/agentroot_basilica.sh << 'EOF'
#!/bin/bash
export AGENTROOT_LLM_URL="https://your-id.deployments.basilica.ai"
export AGENTROOT_LLM_MODEL="Qwen/Qwen2.5-7B-Instruct"
export AGENTROOT_EMBEDDING_URL="https://your-id.deployments.basilica.ai"
export AGENTROOT_EMBEDDING_MODEL="intfloat/e5-mistral-7b-instruct"
export AGENTROOT_EMBEDDING_DIMS="4096"
echo "✅ Basilica endpoints configured"
EOF

chmod +x ~/agentroot_basilica.sh

# Source before using agentroot
source ~/agentroot_basilica.sh
```

See [VLLM_SETUP.md](VLLM_SETUP.md) for detailed Basilica configuration options.

#### Step 2: Index with Basilica Embeddings

```bash
# Add collection
agentroot collection add ~/projects/my-app --name my-app

# Index files
agentroot update

# Generate embeddings using Basilica (automatically uses remote endpoint)
agentroot embed
```

**Benefits of Basilica Embeddings:**
- **10x Faster**: GPU acceleration on verified hardware
- **Higher Quality**: Larger models (e5-mistral-7b, 4096 dims)
- **Consistent**: Same model across your team
- **No Downloads**: No local model files needed
- **Trustless**: Binary verification of GPU compute
- **Reliable**: Automatic failover across 100+ nodes

#### Step 3: Generate AI Metadata

```bash
# Generate rich metadata for all documents using LLM
agentroot metadata refresh my-app
```

This uses the LLM to generate:
- **Summary**: 100-200 word overview
- **Semantic Title**: Improved, descriptive title
- **Keywords**: 5-10 relevant keywords
- **Category**: Document type classification
- **Intent**: Purpose and use case
- **Concepts**: Related technical concepts
- **Difficulty**: Beginner/Intermediate/Advanced
- **Suggested Queries**: Example search queries

Example output:
```
📝 Generating metadata for 127 documents...

[████████████████████] 127/127

✅ Metadata generated (45.2s)
   - Summary: 127 documents
   - Keywords: 635 total
   - Categories: 12 unique
```

#### Step 4: Smart Natural Language Search

```bash
# Use AI-powered query understanding
agentroot smart "show me files that deal with database connections"

# The LLM automatically:
# 1. Parses your natural language query
# 2. Expands it with related terms
# 3. Generates optimal embedding
# 4. Combines with keyword search
# 5. Reranks for relevance
```

Example output:
```
🤖 Parsed query: database connections
📊 Search type: Hybrid
🔍 Expanded terms: connection pool, database client, SQL connection

  94% src/database/pool.rs #a1b2c3
  Connection pool implementation with health checking

  91% src/database/client.rs #d4e5f6
  Database client wrapper with retry logic

  87% src/config/database.rs #g7h8i9
  Database configuration and connection string parsing
```

#### Step 5: View Generated Metadata

```bash
# Show metadata for a specific document
agentroot metadata show "#a1b2c3"
```

Output:
```
📄 Document: src/database/pool.rs

✅ METADATA GENERATED:

📋 Summary:
   Implements a thread-safe connection pool for PostgreSQL databases with
   configurable size, timeouts, and health checks. Provides automatic
   connection recycling and retry logic for failed connections.

🏷️  Semantic Title: PostgreSQL Connection Pool with Health Checking
📂 Category: Database Infrastructure
📊 Difficulty: Intermediate
🎯 Intent: Provide reliable database connection management for production apps

🏷️  Keywords:
   - connection pool
   - database
   - postgresql
   - health check
   - thread-safe
   - retry logic

💡 Related Concepts:
   - Connection pooling patterns
   - Database connection lifecycle
   - Resource management
   - Concurrent access control

🔍 Suggested Queries:
   - "how to configure database connection pool"
   - "postgresql connection management"
   - "database health checking"
```

#### Step 6: Experience Response Caching

```bash
# First query (cache miss - slower)
time agentroot smart "async error handling patterns"
# Time: ~1.5 seconds

# Same query again (cache hit - much faster!)
time agentroot smart "async error handling patterns"
# Time: ~0.15 seconds (10x faster!)
```

**Cache Performance:**
- Embeddings cached by content: 7,000-10,000x speedup (600ms → 80µs)
- Query results cached: 10x speedup
- 1 hour TTL (automatic expiration)
- Thread-safe for concurrent access

#### Step 7: Monitor Performance

```bash
# Run the cache demo to see metrics
cargo run --release --example test_cache
```

Output:
```
=== Test 1: Embed same text twice ===
First embed (cache miss expected):
Time: 632ms

Second embed (cache hit expected):
Time: 80µs  ← 7,900x faster!

Third embed (different text, cache miss expected):
Time: 275ms

=== Metrics ===
Total Requests: 3
Cache Hits: 1
Cache Misses: 2
Cache Hit Rate: 33.3%
Avg Latency: 302.3ms
```

---

## Daily Usage Patterns

### Pattern 1: Morning Sync

Start your day by updating your knowledge base:

```bash
#!/bin/bash
# Save as ~/bin/agentroot-sync

# Configure vLLM
source ~/agentroot_vllm.sh

# Update all collections
echo "📥 Updating collections..."
agentroot update

# Generate embeddings (fast due to caching)
echo "🔄 Generating embeddings..."
agentroot embed

# Check status
echo ""
agentroot status

echo "✅ Sync complete!"
```

Run:
```bash
chmod +x ~/bin/agentroot-sync
agentroot-sync
```

### Pattern 2: Quick Search

Create aliases for common searches:

```bash
# Add to ~/.bashrc or ~/.zshrc
alias as='agentroot smart'      # AI search
alias aq='agentroot query'      # Hybrid search
alias af='agentroot search'     # Fast keyword search

# Usage
as "database error handling"
aq "async patterns"
af "Result<T>"
```

### Pattern 3: Code Review Helper

Search for similar implementations:

```bash
# Find similar error handling
agentroot smart "error handling like src/api/errors.rs" --full

# Find all uses of a pattern
agentroot search "Result<T>" --format csv > results.csv

# Get context for review
agentroot get src/new_feature.rs --line-numbers
```

### Pattern 4: Documentation Writer

Find examples for documentation:

```bash
# Find usage examples
agentroot smart "examples of using the database pool"

# Get code snippets
agentroot multi-get "examples/**/*.rs" \
  --max-lines 30 \
  --format md > examples.md
```

### Pattern 5: Refactoring Assistant

Before refactoring, understand usage:

```bash
# Find all uses of a function
agentroot search "fn deprecated_function"

# Find similar patterns to refactor
agentroot smart "similar to src/old_pattern.rs"

# Check impact across collections
agentroot query "uses deprecated API" --all
```

---

## Troubleshooting

### Issue: "Cannot assign requested address"

**Cause**: vLLM environment variables not set

**Solution**:
```bash
# Verify variables are set
env | grep AGENTROOT

# If not set, configure them
source ~/agentroot_vllm.sh

# Or add to shell profile for permanence
```

See [VLLM_SETUP.md](VLLM_SETUP.md#troubleshooting) for details.

### Issue: Slow Search Performance

**Symptoms**: All queries take 1-2 seconds

**Causes & Solutions**:

1. **Cache not working**
   ```bash
   # Test caching
   cargo run --release --example test_cache
   
   # Should show 7,000x+ speedup for cached items
   ```

2. **No embeddings generated**
   ```bash
   agentroot status
   # Check: Embedded count should equal Documents count
   
   # If not, generate embeddings
   agentroot embed
   ```

3. **Network latency to vLLM**
   ```bash
   # Test endpoint speed
   curl -w "@-" -o /dev/null -s "$AGENTROOT_LLM_URL/health" <<'EOF'
   time_total: %{time_total}
   EOF
   
   # Should be < 100ms
   ```

### Issue: Outdated Results

**Cause**: Files changed but not re-indexed

**Solution**:
```bash
# Re-index changed files
agentroot update

# Re-generate embeddings (fast with caching)
agentroot embed

# Force full re-index if needed
agentroot update --force
agentroot embed --force
```

### Issue: Missing Documents

**Cause**: Glob pattern too restrictive

**Solution**:
```bash
# Check what files match pattern
agentroot ls my-collection

# Modify collection pattern
agentroot collection remove my-collection
agentroot collection add /path/to/code \
  --name my-collection \
  --mask '**/*.{rs,md,toml}'  # Multiple extensions
```

### Issue: Out of Memory

**Cause**: Embedding large collection with local model

**Solution**:
```bash
# Option 1: Use vLLM (remote embeddings)
source ~/agentroot_vllm.sh
agentroot embed

# Option 2: Process in batches
agentroot embed --collection collection1
agentroot embed --collection collection2

# Option 3: Increase system memory or use smaller model
```

---

## Advanced Tips

### Tip 1: Use Collection Filtering

```bash
# Search only in documentation
agentroot smart "installation guide" --collection docs

# Search only in code
agentroot query "error handling" --collection my-app-code
```

### Tip 2: Export Search Results

```bash
# Export as CSV
agentroot query "TODO" --format csv > todos.csv

# Export as JSON for processing
agentroot query "deprecated" --format json | jq '.[] | .filepath'

# Export as Markdown
agentroot search "API" --format md > api-references.md
```

### Tip 3: Combine with Other Tools

```bash
# Find files then edit
agentroot query "needs refactoring" --format files | xargs vim

# Search then grep for details
agentroot smart "database code" --format files | xargs grep -n "SELECT"

# Create task list
agentroot search "TODO\|FIXME" --format csv | \
  awk -F',' '{print "- [ ] " $1 ": " $2}'
```

### Tip 4: Automate Metadata Generation

```bash
#!/bin/bash
# Save as ~/bin/agentroot-auto-metadata

source ~/agentroot_vllm.sh

# Generate metadata for documents that don't have it
agentroot status | grep "Pending:" | while read line; do
  pending=$(echo $line | awk '{print $2}')
  if [ "$pending" -gt 0 ]; then
    echo "📝 Generating metadata for $pending documents..."
    agentroot metadata refresh --all
  fi
done
```

### Tip 5: Pre-commit Hook

```bash
# .git/hooks/pre-commit
#!/bin/bash

# Update agentroot index before committing
agentroot update --collection my-app 2>/dev/null
agentroot embed --collection my-app 2>/dev/null

exit 0
```

---

## Summary

### Basic Workflow
1. `agentroot collection add` - Add files
2. `agentroot update` - Index content
3. `agentroot embed` - Generate embeddings
4. `agentroot query` - Search

### AI-Powered Workflow
1. Configure vLLM endpoints
2. `agentroot update` - Index with remote embeddings
3. `agentroot metadata refresh` - Generate AI metadata
4. `agentroot smart` - Natural language search
5. Enjoy 10x faster cached queries

### Key Benefits
- ✅ **Smart Caching**: 10x faster repeated queries
- ✅ **AI Metadata**: Rich semantic understanding
- ✅ **Natural Language**: Search like you think
- ✅ **Multi-Source**: Unify all your knowledge
- ✅ **Local-First**: Privacy + Offline capability

---

## Next Steps

- **Read**: [VLLM_SETUP.md](VLLM_SETUP.md) for detailed vLLM configuration
- **Explore**: `examples/` directory for code examples
- **Integrate**: [MCP Server Documentation](docs/mcp-server.md) for AI assistant integration
- **Optimize**: [Performance Guide](docs/performance.md) for tuning tips

**Happy Searching!** 🚀
