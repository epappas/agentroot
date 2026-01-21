# AgentRoot Quick Reference

## 30-Second Start

```bash
# 1. Build
cargo build --release

# 2. Add code
agentroot collection add ~/my-project --name myapp

# 3. Index
agentroot update && agentroot embed

# 4. Search
agentroot query "what you're looking for"
```

## 2-Minute Start (with Basilica AI)

```bash
# 1. Get Basilica endpoints at https://basilica.ai
# 2. Configure endpoints
export AGENTROOT_LLM_URL="https://your-id.deployments.basilica.ai"
export AGENTROOT_LLM_MODEL="Qwen/Qwen2.5-7B-Instruct"
export AGENTROOT_EMBEDDING_URL="https://your-id.deployments.basilica.ai"
export AGENTROOT_EMBEDDING_MODEL="intfloat/e5-mistral-7b-instruct"
export AGENTROOT_EMBEDDING_DIMS="4096"

# 2. Build
cargo build --release

# 3. Add code
agentroot collection add ~/my-project --name myapp

# 4. Index with AI
agentroot update && agentroot embed

# 5. Generate metadata (optional)
agentroot metadata refresh myapp

# 6. Smart search
agentroot smart "show me error handling code"
```

## Essential Commands

```bash
# Manage Collections
agentroot collection add <path> --name <name>
agentroot collection list
agentroot collection remove <name>

# Index & Embed
agentroot update              # Re-index files
agentroot embed               # Generate embeddings
agentroot metadata refresh    # Generate AI metadata (vLLM)

# Search
agentroot search "keyword"    # Fast keyword search (<10ms)
agentroot vsearch "query"     # Semantic search (~100ms)
agentroot query "query"       # Hybrid search (~150ms)
agentroot smart "query"       # AI search (vLLM, ~150ms cached)

# View Results
agentroot get "#docid"        # Get document
agentroot ls myapp            # List files
agentroot status              # Check status
```

## Search Comparison

| Command | Speed | Quality | Requires |
|---------|-------|---------|----------|
| `search` | ⚡⚡⚡ <10ms | ⭐⭐⭐ | Nothing |
| `vsearch` | ⚡⚡ ~100ms | ⭐⭐⭐⭐ | Embeddings |
| `query` | ⚡⚡ ~150ms | ⭐⭐⭐⭐⭐ | Embeddings |
| `smart` | ⚡ ~1.5s first, ⚡⚡ ~150ms cached | ⭐⭐⭐⭐⭐ | vLLM |

## Environment Variables

```bash
# Basilica Configuration (get endpoints at https://basilica.ai)
export AGENTROOT_LLM_URL="https://your-id.deployments.basilica.ai"
export AGENTROOT_LLM_MODEL="Qwen/Qwen2.5-7B-Instruct"
export AGENTROOT_EMBEDDING_URL="https://your-id.deployments.basilica.ai"
export AGENTROOT_EMBEDDING_MODEL="intfloat/e5-mistral-7b-instruct"
export AGENTROOT_EMBEDDING_DIMS="4096"

# Optional
export AGENTROOT_DB="~/.cache/agentroot/index.sqlite"
export RUST_LOG="debug"  # For troubleshooting
```

**Note**: Basilica provides trustless GPU compute on Bittensor's decentralized network. See [basilica.ai](https://basilica.ai) or [github.com/one-covenant/basilica](https://github.com/one-covenant/basilica).

## Common Patterns

### Daily Sync
```bash
agentroot update && agentroot embed
```

### Find Similar Code
```bash
agentroot smart "code similar to src/handler.rs"
```

### Export Results
```bash
agentroot search "TODO" --format csv > todos.csv
agentroot query "deprecated" --format json | jq
```

### Multi-Source Search
```bash
# Search across all sources
agentroot query "database pooling"

# Search specific collection
agentroot query "async traits" --collection rust-docs
```

## Performance Tips

1. **Use vLLM for speed**: 10x faster with GPU acceleration
2. **Enable caching**: Automatic, 7,000x speedup for repeated queries
3. **Filter collections**: Use `--collection` for faster targeted search
4. **Use right search**: `search` for keywords, `smart` for natural language
5. **Batch operations**: `agentroot update && agentroot embed` at once

## Troubleshooting

### Error: "Cannot assign requested address"
**Fix**: Set Basilica environment variables
```bash
# Get endpoints at https://basilica.ai first
source ~/agentroot_basilica.sh
```

### Slow Performance
**Check**:
```bash
agentroot status  # Verify embeddings generated
cargo run --example test_cache  # Test caching works
```

### Outdated Results
**Fix**:
```bash
agentroot update  # Re-index changed files
agentroot embed   # Re-generate embeddings (fast with cache)
```

## Full Documentation

- **Complete Guide**: [WORKFLOW.md](WORKFLOW.md)
- **vLLM Setup**: [VLLM_SETUP.md](VLLM_SETUP.md)
- **All Docs**: [docs/README.md](docs/README.md)
- **Main README**: [README.md](README.md)

## Examples

```bash
# See what caching does
cargo run --release --example test_cache

# GitHub repository indexing
cargo run --example github_provider

# Multi-source indexing
cargo run --example url_provider
cargo run --example csv_provider
```

---

**Quick Help**: `agentroot --help`
**Command Help**: `agentroot <command> --help`
**Issues**: https://github.com/epappas/agentroot/issues
