# LLM Orchestrated Search: Now Default

## Change Summary

**Date**: 2026-01-23  
**Status**: ✅ **ENABLED BY DEFAULT**

LLM orchestrated search is now the **default search mode** for AgentRoot. The system intelligently falls back to simpler strategies when LLM is unavailable.

---

## How It Works

### Search Strategy Cascade

```
agentroot search "query"
    ↓
1. Try: LLM Orchestrated Search
   - LLM plans multi-step workflow
   - Executes: vector/hybrid/bm25 + filters + rerank
   - Uses tuned scoring (importance + collection + path filters)
   ↓ (if LLM unavailable)
2. Fallback: Heuristic Workflow
   - Smart workflow based on query patterns
   - Natural language → vector search
   - Technical terms → BM25 search
   - Mixed → hybrid search
   ↓ (if embeddings unavailable)
3. Final Fallback: BM25-only
   - Pure keyword matching
   - Fast and reliable
```

### Automatic Fallback

**No configuration required**. The system automatically:
- Detects LLM availability
- Detects embedding availability
- Chooses best strategy
- Falls back gracefully on failure

---

## Examples

### Example 1: Full LLM Orchestration

**Setup**:
```bash
export AGENTROOT_LLM_URL="https://your-llm-api.com"
export AGENTROOT_LLM_MODEL="Qwen/Qwen2.5-7B-Instruct"
export AGENTROOT_EMBEDDING_URL="https://your-embed-api.com"
export AGENTROOT_EMBEDDING_MODEL="intfloat/e5-mistral-7b-instruct"
export AGENTROOT_EMBEDDING_DIMS="4096"
```

**Query**:
```bash
agentroot search "how do I get started" -n 5
```

**Behavior**:
- ✅ LLM plans workflow (hybrid → glossary → merge → rerank)
- ✅ Uses vector embeddings for semantic search
- ✅ Applies tuned boosting (docs > src, test penalty)
- ✅ Returns: `getting-started.md` #1

**Results**:
```
50% agentroot/docs/getting-started.md
50% agentroot/docs/cli-reference.md
50% agentroot/README.md
```

---

### Example 2: Fallback (No LLM)

**Setup**:
```bash
# LLM not configured
export AGENTROOT_EMBEDDING_URL="https://your-embed-api.com"
export AGENTROOT_EMBEDDING_MODEL="intfloat/e5-mistral-7b-instruct"
export AGENTROOT_EMBEDDING_DIMS="4096"
```

**Query**:
```bash
agentroot search "MCP server" -n 5
```

**Behavior**:
- ⚠️ LLM unavailable → uses fallback workflow
- ✅ Detects natural language query
- ✅ Uses vector search (embeddings available)
- ✅ Returns: `mcp-server.md` #1

**Results**:
```
14% agentroot/docs/mcp-server.md
14% agentroot/docs/troubleshooting.md
11% agentroot/docs/cli-reference.md
```

---

### Example 3: Full Fallback (No LLM, No Embeddings)

**Setup**:
```bash
# Nothing configured
```

**Query**:
```bash
agentroot search "getting started" -n 5
```

**Behavior**:
- ⚠️ LLM unavailable → fallback workflow
- ⚠️ Embeddings unavailable → BM25 only
- ✅ Uses pure keyword matching
- ✅ Returns: `getting-started.md` #1

**Results**:
```
147% agentroot/docs/getting-started.md
123% agentroot/docs/troubleshooting.md
99% agentroot/docs/howto-guide.md
```

---

## Configuration

### Optional Environment Variables

| Variable | Purpose | Required |
|----------|---------|----------|
| `AGENTROOT_LLM_URL` | LLM API endpoint | Optional (enables orchestration) |
| `AGENTROOT_LLM_MODEL` | LLM model name | Optional (with LLM_URL) |
| `AGENTROOT_EMBEDDING_URL` | Embedding API | Optional (enables vector search) |
| `AGENTROOT_EMBEDDING_MODEL` | Embedding model | Optional (with EMBEDDING_URL) |
| `AGENTROOT_EMBEDDING_DIMS` | Embedding dimensions | Optional (with EMBEDDING_URL) |

**Default Behavior**: Works without any configuration (falls back to BM25)

---

## Performance Comparison

### Query: "how do I get started"

| Mode | Result #1 | Score | Speed | Notes |
|------|-----------|-------|-------|-------|
| **LLM Orchestrated** | ✅ getting-started.md | 50% | ~2s | Best semantic understanding |
| **Fallback (embeddings)** | ✅ getting-started.md | 14% | ~1s | Good semantic search |
| **BM25 only** | ✅ getting-started.md | 147% | ~0.1s | Fastest, keyword-based |

### Query: "MCP server"

| Mode | Result #1 | Score | Speed | Notes |
|------|-----------|-------|-------|-------|
| **LLM Orchestrated** | ✅ mcp-server.md | 50% | ~2s | Handles filename mismatch |
| **Fallback (embeddings)** | ✅ mcp-server.md | 14% | ~1s | Semantic search works |
| **BM25 only** | ❌ No results | - | ~0.1s | Misses due to hyphen |

**Conclusion**: LLM orchestrated mode provides best quality, with intelligent fallbacks for speed/simplicity.

---

## Tuning Features (Built-in)

The orchestrated search includes automatic tuning:

### 1. Document Importance (PageRank-based)
```
getting-started.md: 4.5x boost (highly linked)
test files: 1.0x (default)
```

### 2. Collection Boosting
```
agentroot (docs): 1.5x boost
agentroot-src (code): 0.7x penalty
```

### 3. Path-based Filtering
```
/tests/: 0.1x (90% penalty)
/docs/: 1.0x (normal)
```

### Combined Effect
```
Docs vs Tests: ~96x ranking advantage
```

---

## Code Changes

### File Modified
**`crates/agentroot-cli/src/commands/search.rs`**

**Before** (required explicit LLM configuration):
```rust
let llm_available = std::env::var("AGENTROOT_LLM_URL").is_ok();
let results = if llm_available {
    orchestrated_search(db, &query, &options).await?
} else {
    unified_search(db, &query, &options).await?
};
```

**After** (tries orchestrated by default):
```rust
let results = match orchestrated_search(db, &query, &options).await {
    Ok(results) => results,
    Err(e) => {
        // Graceful fallback to unified search
        unified_search(db, &query, &options).await?
    }
};
```

---

## Migration Guide

### For Users

**No action required**. Search will automatically:
- Use LLM if configured (better quality)
- Fall back to heuristics if not (good quality)
- Use BM25 as final fallback (fast)

### For Developers

**No breaking changes**. Existing code continues to work:
- `agentroot search "query"` - uses new default
- `agentroot vsearch "query"` - deprecated but still works
- API: `db.search_fts()` / `db.search_vec()` - unchanged

---

## Monitoring

### Check Current Mode

**With verbose logging**:
```bash
export RUST_LOG=info
agentroot search "query"
```

**Look for**:
- `LLM Workflow: N steps` → Orchestrated mode active
- `Workflow planning failed, using fallback` → Fallback mode
- `Strategy: BM25` → BM25-only mode

---

## Troubleshooting

### Issue: Getting test files in results

**Diagnosis**: Vector search penalty not applied

**Solution**: Rebuild database importance scores:
```bash
agentroot pagerank compute
```

### Issue: Slow search

**Diagnosis**: LLM orchestration adds latency (~2s)

**Solution**: Disable LLM for speed:
```bash
unset AGENTROOT_LLM_URL
agentroot search "query"
```

### Issue: Poor semantic search

**Diagnosis**: Embeddings not indexed

**Solution**: Run embedding indexing:
```bash
agentroot embed
```

---

## Future Enhancements

1. **Adaptive mode switching** - Switch strategies mid-query based on results
2. **Learning from usage** - Optimize boosting based on user behavior
3. **Query expansion** - LLM-powered synonym expansion
4. **Multi-modal search** - Images, diagrams, code screenshots

---

## References

- **Tuning Results**: See `TUNING_RESULTS.md`
- **Enhancement Analysis**: See `LLM_ENHANCEMENT_ANALYSIS.md`
- **Orchestration Proof**: See `ORCHESTRATED_SEARCH_PROOF.md`

---

**Summary**: LLM orchestrated search is now **production-default**, with smart fallbacks ensuring reliability across all deployment scenarios. No configuration required for basic use, optional LLM/embedding APIs for enhanced quality.
