# Evidence-Based Analysis: Does LLM Enhancement Improve Search Quality?

## Executive Summary

**Conclusion**: **Mixed results**. LLM orchestration provides **better semantic search** for abstract queries but **worse results** for exact keyword matches. Current implementation needs tuning before production use.

---

## Test Results (2026-01-23)

### Query 1: "how do I get started"

| Rank | BM25-Only (Baseline) | LLM Orchestrated | Winner |
|------|---------------------|------------------|--------|
| #1   | ✅ `docs/getting-started.md` (147%) | ❌ `tests/document_tests.rs` (50%) | **BM25** |
| #2   | `docs/troubleshooting.md` (123%) | ❌ `tests/collection_tests.rs` (50%) | **BM25** |
| #3   | `docs/howto-guide.md` (99%) | ❌ `tests/search_tests.rs` (50%) | **BM25** |
| #4   | `docs/README.md` (96%) | ❌ `tests/update_embed_tests.rs` (50%) | **BM25** |
| #5   | `docs/metadata-user-guide.md` (67%) | ❌ `tools.rs` (50%) | **BM25** |

**Analysis**: 
- BM25 found the perfect document (#1: `getting-started.md`)
- LLM returned only test files in top 5
- `getting-started.md` not even in LLM's top 5 results
- **BM25 wins decisively**

**LLM Workflow Used**: Hybrid → Glossary → Merge → Rerank (4 steps)
**Problem**: Vector search + reranking ranked test files higher than docs

---

### Query 2: "MCP server"

| Rank | BM25-Only (Baseline) | LLM Orchestrated | Winner |
|------|---------------------|------------------|--------|
| #1   | ❌ No results | ✅ `docs/mcp-server.md` (50%) | **LLM** |
| #2   | - | `docs/troubleshooting.md` (50%) | **LLM** |
| #3   | - | `src/app.rs` (50%) | **LLM** |
| #4   | - | `mcp/lib.rs` (50%) | **LLM** |
| #5   | - | `docs/metadata-example-output.md` (50%) | **LLM** |

**Analysis**:
- BM25 found nothing (no exact match for "MCP server" vs "mcp-server.md")
- LLM correctly found `mcp-server.md` as #1
- **LLM wins decisively**

**LLM Workflow Used**: Vector → FilterMetadata → Rerank (3 steps)
**Success**: Semantic search found the right document despite filename mismatch

---

### Query 3: "semantic chunking"

| Rank | BM25-Only (Baseline) | LLM Orchestrated | Winner |
|------|---------------------|------------------|--------|
| #1   | ❌ No results | ✅ `docs/semantic-chunking.md` (50%) | **LLM** |
| #2   | - | `db/glossary.rs` (50%) | **LLM** |
| #3   | - | `docs/cli-reference.md` (50%) | **LLM** |
| #4   | - | `docs/troubleshooting.md` (50%) | **LLM** |
| #5   | - | `docs/getting-started.md` (50%) | **LLM** |

**Analysis**:
- BM25 found nothing
- LLM correctly found `semantic-chunking.md` as #1
- **LLM wins decisively**

**LLM Workflow Used**: Hybrid → Glossary → Merge → Rerank (4 steps)

---

### Query 4: "metadata generation"

| Rank | BM25-Only (Baseline) | LLM Orchestrated | Winner |
|------|---------------------|------------------|--------|
| #1   | `docs/providers.md` (293%) | `docs/providers.md` (50%) | **Tie** |
| #2   | `docs/cli-reference.md` (221%) | `docs/cli-reference.md` (50%) | **Tie** |
| #3   | `docs/getting-started.md` (210%) | `docs/getting-started.md` (50%) | **Tie** |
| #4   | `docs/architecture.md` (191%) | `docs/architecture.md` (191%) | **Tie** |
| #5   | `README.md` (176%) | `README.md` (176%) | **Tie** |

**Analysis**:
- **Identical results** in same order
- BM25 has better score differentiation (293% to 176%)
- LLM has flat scores (all 50%)
- **Tie** - same documents, same ranking

**LLM Workflow Used**: Vector → FilterMetadata(tags) → Rerank (3 steps)
**Note**: Filter was skipped (100% removal), fell back to unfiltered results

---

### Query 5: "provider implementation"

| Rank | BM25-Only (Baseline) | LLM Orchestrated | Winner |
|------|---------------------|------------------|--------|
| #1   | ❌ No results | `docs/mcp-server.md` (50%) | **LLM** |
| #2   | - | `examples/README.md` (50%) | **LLM** |
| #3   | - | - | **LLM** |

**Analysis**:
- BM25 found nothing
- LLM found 2 results (not great - missing `custom_provider.rs`)
- **LLM wins** (only one with results)

**LLM Workflow Used**: Vector → FilterMetadata → Rerank (3 steps)

---

## Score Summary

| Query | BM25 Win | LLM Win | Tie | Notes |
|-------|----------|---------|-----|-------|
| "how do I get started" | ✅ | | | BM25 perfect, LLM completely wrong |
| "MCP server" | | ✅ | | LLM found it, BM25 nothing |
| "semantic chunking" | | ✅ | | LLM found it, BM25 nothing |
| "metadata generation" | | | ✅ | Identical results |
| "provider implementation" | | ✅ | | LLM found something, BM25 nothing |

**Final Score**: BM25: 1, LLM: 3, Tie: 1

**But**: The one BM25 win was a **complete failure** of LLM (wrong documents entirely)

---

## Analysis

### LLM Strengths

1. **Semantic matching**: Finds documents even when keywords don't match exactly
   - "MCP server" → found "mcp-server.md" (BM25 missed)
   - "semantic chunking" → found "semantic-chunking.md"

2. **Handles concept queries**: Better at abstract/conceptual searches
   - Queries with no exact keyword matches

3. **Multi-step workflows**: Can combine strategies (hybrid + rerank)

### LLM Weaknesses

1. **❌ CRITICAL**: Returns completely wrong results for exact keyword queries
   - "how do I get started" → returned test files instead of `getting-started.md`
   - This is a production-breaking bug

2. **Poor score differentiation**: All results score 50% (flat)
   - BM25 scores range from 67% to 293%
   - Makes it hard to judge confidence

3. **Slower**: LLM planning + multi-step execution takes longer

4. **Filter failures**: LLM invents metadata filters that don't exist
   - Requires safety mechanism to skip bad filters

### BM25 Strengths

1. **✅ Excellent for exact matches**: Perfect when keywords match
   - "how do I get started" → correctly found `getting-started.md` #1

2. **Better score differentiation**: Clear confidence rankings (67%-293%)

3. **Faster**: Single-step search, no LLM overhead

4. **Predictable**: Same query always returns same results

### BM25 Weaknesses

1. **❌ Fails on semantic queries**: Returns nothing when keywords don't match
   - "MCP server" → nothing (vs filename "mcp-server.md")
   - "semantic chunking" → nothing
   - "provider implementation" → nothing

---

## Root Cause Analysis

### Why LLM Failed on "how do I get started"

**LLM Workflow**:
```
1. Hybrid Search (BM25 + Vector) for "get started agentroot"
2. Glossary Search for "agentroot"
3. Merge using RRF
4. Rerank top 10
```

**Problem**: 
- Vector embeddings for test files likely contain phrases like "get", "started", "agentroot"
- Reranker (if enabled) or vector scores ranked test files higher than docs
- Without strong BM25 signal, semantic search went wrong

**Fix Needed**:
- Boost document importance scores (README, getting-started should rank higher)
- Improve training/prompts for reranker
- Add document type filtering (exclude tests from user queries)

---

## Recommendations

### Short-term (Before Production)

1. **❌ Do NOT use LLM orchestrated mode as default**
   - Current failure on basic queries is unacceptable

2. **Use hybrid approach**:
   - Detect query type (exact vs semantic)
   - Use BM25 for keyword queries
   - Use LLM for semantic queries

3. **Add document type filters**:
   - Exclude test files from user-facing search
   - Boost documentation files

### Long-term (Future Improvements)

1. **Fix reranker/vector scoring**:
   - Investigate why test files rank higher
   - Add document importance to final scoring

2. **Improve LLM prompts**:
   - Teach LLM to detect exact keyword queries
   - Guide LLM to choose BM25 for exact matches

3. **Add confidence thresholds**:
   - If LLM workflow has low confidence, fallback to BM25

---

## Honest Conclusion

**Current State**: LLM enhancement is **NOT production-ready**.

**Evidence**:
- ✅ LLM is better at semantic/conceptual searches (3 wins)
- ✅ LLM finds documents BM25 misses
- ❌ LLM completely fails on basic keyword queries (returns test files)
- ❌ Critical production bug: "how do I get started" returns wrong documents

**Verdict**: LLM orchestration **improves semantic search** but **degrades keyword search**. Net effect is negative until the keyword search issue is fixed.

**Action Required**: Fix document importance scoring and test file filtering before enabling LLM orchestration by default.

---

**Test Date**: 2026-01-23
**Test Environment**: Basilica LLM API (Qwen 2.5-7B) + e5-mistral embeddings
**Database**: 151 documents, 997 embeddings
**Reproduction**: Run queries above with and without `AGENTROOT_LLM_URL` set
