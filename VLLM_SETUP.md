# vLLM Setup Guide for AgentRoot

This guide shows you how to configure AgentRoot to use your external vLLM endpoints for LLM and embedding services.

## Quick Start

### Option 1: Environment Variables (Recommended)

Add these to your shell profile (`~/.bashrc`, `~/.zshrc`, or `~/.profile`):

```bash
# vLLM LLM Service (for query parsing and metadata generation)
export AGENTROOT_LLM_URL="https://68e9761a-6912-4f90-bb50-45f5520ba743.deployments.basilica.ai"
export AGENTROOT_LLM_MODEL="Qwen/Qwen2.5-7B-Instruct"

# vLLM Embedding Service (for vector search)
export AGENTROOT_EMBEDDING_URL="https://1ff15927-4101-43e5-869b-929925b34083.deployments.basilica.ai"
export AGENTROOT_EMBEDDING_MODEL="intfloat/e5-mistral-7b-instruct"
export AGENTROOT_EMBEDDING_DIMS="4096"
```

Then reload your shell:
```bash
source ~/.bashrc  # or ~/.zshrc
```

### Option 2: Per-Session Setup

Create a setup script you can source before using agentroot:

```bash
# Create setup script
cat > ~/agentroot_env.sh << 'EOF'
#!/bin/bash
export AGENTROOT_LLM_URL="https://68e9761a-6912-4f90-bb50-45f5520ba743.deployments.basilica.ai"
export AGENTROOT_LLM_MODEL="Qwen/Qwen2.5-7B-Instruct"
export AGENTROOT_EMBEDDING_URL="https://1ff15927-4101-43e5-869b-929925b34083.deployments.basilica.ai"
export AGENTROOT_EMBEDDING_MODEL="intfloat/e5-mistral-7b-instruct"
export AGENTROOT_EMBEDDING_DIMS="4096"
echo "✅ vLLM endpoints configured"
EOF

chmod +x ~/agentroot_env.sh
```

Use it before running agentroot:
```bash
source ~/agentroot_env.sh
agentroot smart "your query here"
```

### Option 3: Inline (For Testing)

```bash
AGENTROOT_LLM_URL="https://68e9761a-6912-4f90-bb50-45f5520ba743.deployments.basilica.ai" \
AGENTROOT_LLM_MODEL="Qwen/Qwen2.5-7B-Instruct" \
AGENTROOT_EMBEDDING_URL="https://1ff15927-4101-43e5-869b-929925b34083.deployments.basilica.ai" \
AGENTROOT_EMBEDDING_MODEL="intfloat/e5-mistral-7b-instruct" \
AGENTROOT_EMBEDDING_DIMS="4096" \
agentroot smart "test query"
```

## Configuration Reference

### Required Environment Variables

| Variable | Description | Example |
|----------|-------------|---------|
| `AGENTROOT_LLM_URL` | vLLM endpoint for text generation | `https://your-llm.deployments.basilica.ai` |
| `AGENTROOT_LLM_MODEL` | Model name for LLM service | `Qwen/Qwen2.5-7B-Instruct` |
| `AGENTROOT_EMBEDDING_URL` | vLLM endpoint for embeddings | `https://your-embed.deployments.basilica.ai` |
| `AGENTROOT_EMBEDDING_MODEL` | Model name for embedding service | `intfloat/e5-mistral-7b-instruct` |
| `AGENTROOT_EMBEDDING_DIMS` | Embedding dimensions | `4096` |

### Optional Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `AGENTROOT_LLM_TIMEOUT` | Request timeout in seconds | `120` |
| `AGENTROOT_DB` | Custom database path | `~/.cache/agentroot/index.sqlite` |

## Verifying Setup

### 1. Check Status
```bash
agentroot status
```

Should show your collections and documents without errors.

### 2. Test Search
```bash
# Simple keyword search (no LLM/embedding needed)
agentroot search "test"

# Smart search (uses LLM + embeddings)
agentroot smart "files edited recently"
```

### 3. Test Caching
Run the same query twice to see caching in action:
```bash
agentroot smart "rust programming" -n 2
agentroot smart "rust programming" -n 2  # Second run should be faster
```

### 4. Run Cache Demo
```bash
cargo run --release --example test_cache
```

Expected output:
```
=== Test 1: Embed same text twice ===
First embed (cache miss expected):
Time: 600-800ms

Second embed (cache hit expected):
Time: 50-100µs  (7,000-10,000x faster!)

=== Metrics ===
Cache Hit Rate: 33.3%
```

## Troubleshooting

### Error: "Cannot assign requested address (os error 99)"

**Cause**: Environment variables not set, trying to connect to default localhost:8000

**Solution**: Export the environment variables (see Option 1 or 2 above)

### Error: "HTTP error: error sending request"

**Possible causes**:
1. Wrong URL - check `AGENTROOT_LLM_URL` and `AGENTROOT_EMBEDDING_URL`
2. Network issue - verify endpoints are accessible
3. Authentication required - add `AGENTROOT_LLM_API_KEY` if needed

### Slow Performance

**Expected behavior**:
- First query: 1-2 seconds (API call + processing)
- Repeated query: <100ms (cached)

**If all queries are slow**:
- Check network latency to vLLM endpoints
- Verify embedding dimensions match (`AGENTROOT_EMBEDDING_DIMS=4096`)
- Try smaller batch sizes

### Cache Not Working

**Verify caching is enabled** (it is by default):
```bash
cargo run --release --example test_cache
```

**Check cache behavior**:
- First run: "cache miss" (slower)
- Second run: "cache hit" (much faster)
- Different text: "cache miss" (slower)

## Features Enabled with vLLM

With vLLM endpoints configured, you get:

✅ **Smart Natural Language Search**
```bash
agentroot smart "show me recent changes to authentication code"
```

✅ **LLM-Generated Metadata**
```bash
agentroot metadata refresh my-collection
```

✅ **Query Expansion**
- Automatically expands queries with related terms
- Improves search recall

✅ **Hybrid Search**
```bash
agentroot query "complex technical query"
```
Combines BM25 + vector similarity + reranking

✅ **Response Caching**
- 7,000-10,000x speedup for repeated queries
- Automatic cache management (1 hour TTL)

✅ **Batch Optimization**
- Efficient embedding generation
- Parallel processing for large collections

## Performance Tuning

### Cache TTL (Default: 1 hour)
Currently configured in code, can be customized by:
1. Using `LLMCache::with_ttl(Duration::from_secs(3600))`
2. Or accepting the default

### Batch Size (Default: 32)
For embedding large collections:
```rust
// In code
client.embed_batch_optimized(texts, 64, None).await?  // Larger batches
```

### Concurrent Batches (Default: 4)
```rust
// In code
client.embed_batch_parallel(texts, 32, 8).await?  // More parallelism
```

## API Compatibility

AgentRoot uses the **OpenAI-compatible API** format, which vLLM supports:

### Chat Completions
```
POST /v1/chat/completions
{
  "model": "Qwen/Qwen2.5-7B-Instruct",
  "messages": [...],
  "temperature": 0.7,
  "max_tokens": 512
}
```

### Embeddings
```
POST /v1/embeddings
{
  "model": "intfloat/e5-mistral-7b-instruct",
  "input": ["text1", "text2", ...]
}
```

## Additional Resources

- **vLLM Documentation**: https://docs.vllm.ai/
- **AgentRoot Documentation**: See `docs/` directory
- **Example Code**: See `examples/` directory
- **Cache Demo**: `examples/test_cache.rs`

## Support

If you encounter issues:

1. Verify environment variables: `env | grep AGENTROOT`
2. Test network connectivity: `curl $AGENTROOT_LLM_URL/health` (if available)
3. Check logs: `RUST_LOG=debug agentroot smart "test"`
4. Run example: `cargo run --release --example test_cache`

For more help, see `README.md` and `CONTRIBUTING.md`.
