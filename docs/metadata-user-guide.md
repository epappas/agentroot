# Metadata System User Guide

This guide explains how to use Agentroot's LLM-powered metadata generation system to improve search quality and document discoverability.

## Table of Contents

1. [Overview](#overview)
2. [Getting Started](#getting-started)
3. [CLI Commands](#cli-commands)
4. [When to Regenerate Metadata](#when-to-regenerate-metadata)
5. [Customizing Metadata Generation](#customizing-metadata-generation)
6. [Troubleshooting](#troubleshooting)
7. [Best Practices](#best-practices)

## Overview

### What is Metadata?

Agentroot automatically generates rich semantic metadata for every indexed document, including:

- **Summary**: 100-200 word overview of the content
- **Semantic Title**: Improved title derived from content
- **Keywords**: 5-10 relevant search terms
- **Category**: Document type (tutorial, reference, code, config, etc.)
- **Intent**: Purpose and problem the document solves
- **Concepts**: Related technologies and entities
- **Difficulty**: Skill level (beginner, intermediate, advanced)
- **Suggested Queries**: Search terms to find this document

### Why Use Metadata?

Metadata improves your search experience by:

✅ **Better Ranking**: Documents with relevant metadata rank higher  
✅ **Precise Filtering**: Search by category, difficulty, or concept  
✅ **Improved Discovery**: Find documents you didn't know existed  
✅ **Contextual Results**: Understand what a document is about before opening it  

### How It Works

1. **Automatic Generation**: Metadata is created when you index a collection
2. **Smart Caching**: Metadata is cached by content hash (regenerated only when content changes)
3. **Fallback System**: Even without an LLM model, heuristic metadata is generated
4. **Search Integration**: Metadata is automatically searchable via BM25 full-text search

## Getting Started

### Initial Setup

No special configuration required! Metadata generation happens automatically when you index collections.

```bash
# Add a collection
agentroot collection add my-docs /path/to/documents --pattern "**/*.md"

# Index with metadata (automatic)
agentroot update

# Check status
agentroot status
```

The `agentroot status` command shows:
```
Index: 150 documents across 3 collections

Metadata:
  Generated:     150
  Pending:       0
```

### With LLM Model (Optional)

For higher quality metadata, download an LLM model:

```bash
# Download the default metadata model (llama-3.1-8b-instruct)
# Place in: ~/.local/share/agentroot/models/

# Or specify a custom model path in config:
# ~/.config/agentroot/config.toml
[metadata]
model_path = "/path/to/llama-3.1-8b-instruct.Q4_K_M.gguf"
```

**Note**: Without an LLM model, Agentroot uses intelligent fallback heuristics:
- Title from filename
- Summary from first paragraph
- Keywords from term frequency
- Category from file extension
- Concepts from capitalized terms

## CLI Commands

### `metadata refresh`

Regenerate metadata for a collection, document, or all collections.

#### Refresh Entire Collection

```bash
agentroot metadata refresh my-docs
```

#### Refresh All Collections

```bash
agentroot metadata refresh --all
```

#### Refresh Single Document

```bash
agentroot metadata refresh --doc path/to/document.md
# or
agentroot metadata refresh --doc #abc123
```

#### Force Regeneration

Ignore cache and regenerate metadata:

```bash
agentroot metadata refresh my-docs --force
```

#### Use Custom Model

```bash
agentroot metadata refresh my-docs --model /path/to/model.gguf
```

### `metadata show`

Display metadata for a specific document.

```bash
# By document ID
agentroot metadata show #abc123

# By path
agentroot metadata show my-docs/tutorial.md

# With JSON output
agentroot metadata show #abc123 --format json
```

**Example Output**:
```
Document: my-docs/rust-tutorial.md
Doc ID: #abc123

Summary:
  This tutorial introduces Rust programming for beginners...

Category: tutorial
Difficulty: beginner
Keywords: rust, programming, tutorial, systems, memory-safe

Concepts:
  - Rust
  - Systems Programming
  - Memory Safety

Suggested Queries:
  - rust tutorial beginners
  - learn rust programming
  - rust getting started
```

### `status`

View metadata generation statistics.

```bash
agentroot status
```

Shows:
- Total documents with metadata
- Documents pending metadata generation
- Collections with/without metadata
- Cache hit rate (if available)

## When to Regenerate Metadata

### Automatic Regeneration

Metadata is **automatically regenerated** when:

✅ Content changes (different content hash)  
✅ Running `agentroot update` or `agentroot collection update`  

### Manual Regeneration Needed

Regenerate metadata **manually** when:

1. **LLM Model Upgraded**
   ```bash
   # After upgrading to a better model
   agentroot metadata refresh --all --force
   ```

2. **Metadata Quality Issues**
   ```bash
   # If metadata seems inaccurate
   agentroot metadata refresh my-docs --force
   ```

3. **Schema/Prompt Changes**
   ```bash
   # After updating Agentroot
   agentroot metadata refresh --all
   ```

4. **Selective Re-generation**
   ```bash
   # Re-generate for specific category
   agentroot metadata refresh my-docs
   ```

### When NOT to Regenerate

❌ **Content unchanged**: Metadata is cached, no need to regenerate  
❌ **After every search**: Metadata doesn't change with searches  
❌ **Frequently**: Only regenerate when content or model changes  

## Customizing Metadata Generation

### Configuration File

Edit `~/.config/agentroot/config.toml`:

```toml
[metadata]
# Enable/disable metadata generation globally
enabled = true

# Path to LLM model file
model_path = "/path/to/llama-3.1-8b-instruct.gguf"

# Maximum tokens to send to LLM (higher = more context, slower)
max_content_tokens = 2048

# Enable caching (recommended)
cache_enabled = true
```

### Collection-Level Configuration

Set metadata options per collection:

```bash
agentroot collection add my-docs /path/to/docs \
  --config '{"metadata_enabled": true, "metadata_model": "llama-3.1-8b-instruct"}'
```

### Environment Variables

Override config with environment variables:

```bash
# Disable metadata generation temporarily
AGENTROOT_METADATA_ENABLED=false agentroot update

# Use custom model
AGENTROOT_METADATA_MODEL=/path/to/model.gguf agentroot update
```

### Custom Prompts (Advanced)

For developers, customize the metadata generation prompt by modifying:
```
crates/agentroot-core/src/llm/llama_metadata.rs
```

Look for the `build_prompt()` function.

## Troubleshooting

### Problem: No Metadata Generated

**Symptoms**:
- `agentroot status` shows "Metadata: Generated: 0"
- Search results don't include metadata fields

**Solutions**:

1. **Check if metadata is enabled**:
   ```bash
   # Verify config
   cat ~/.config/agentroot/config.toml | grep metadata
   ```

2. **Re-index with metadata**:
   ```bash
   agentroot metadata refresh my-docs
   ```

3. **Check for errors**:
   ```bash
   # Run with verbose logging
   RUST_LOG=debug agentroot update
   ```

### Problem: LLM Model Not Found

**Symptoms**:
- Error: "LLM model not found at..."
- Metadata uses fallback heuristics

**Solutions**:

1. **Download the model**:
   ```bash
   # Download llama-3.1-8b-instruct.Q4_K_M.gguf
   # Place in: ~/.local/share/agentroot/models/
   ```

2. **Specify model path**:
   ```toml
   [metadata]
   model_path = "/path/to/model.gguf"
   ```

3. **Use fallback** (no action needed):
   - Agentroot automatically uses heuristic metadata
   - Still provides value without LLM

### Problem: Metadata Quality is Poor

**Symptoms**:
- Inaccurate summaries
- Irrelevant keywords
- Wrong category

**Solutions**:

1. **Upgrade to LLM model** (if using fallback):
   ```bash
   # Download llama-3.1-8b-instruct model
   # Configure path in config.toml
   agentroot metadata refresh --all --force
   ```

2. **Increase context size**:
   ```toml
   [metadata]
   max_content_tokens = 4096  # Default: 2048
   ```

3. **Check document quality**:
   - Ensure documents have clear structure
   - Add headings and paragraphs
   - Include relevant keywords in content

### Problem: Metadata Regeneration Takes Too Long

**Symptoms**:
- `metadata refresh` is slow
- Indexing takes minutes for small collections

**Solutions**:

1. **Use cache** (default):
   ```toml
   [metadata]
   cache_enabled = true
   ```

2. **Reduce context size**:
   ```toml
   [metadata]
   max_content_tokens = 1024  # Faster, less accurate
   ```

3. **Use fallback mode**:
   ```bash
   # Temporarily disable LLM
   AGENTROOT_METADATA_MODEL= agentroot metadata refresh my-docs
   ```

4. **Refresh selectively**:
   ```bash
   # Only refresh changed documents
   agentroot update  # Instead of --force
   ```

### Problem: Database Growing Too Large

**Symptoms**:
- Database file > 1GB
- Slow queries

**Solutions**:

1. **Vacuum database**:
   ```bash
   agentroot vacuum
   ```

2. **Reduce metadata verbosity** (future feature):
   ```toml
   [metadata]
   summary_max_words = 100  # Default: 200
   ```

3. **Selective indexing**:
   ```bash
   # Index only important files
   agentroot collection add docs /path --pattern "**/*.md"
   # Instead of "**/*"
   ```

### Problem: Search Not Using Metadata

**Symptoms**:
- Search results don't improve with metadata
- Filtering by category doesn't work

**Solutions**:

1. **Verify metadata in results**:
   ```bash
   agentroot search "rust tutorial" --format json | jq '.results[0].metadata'
   ```

2. **Use metadata filters** (MCP tools):
   ```json
   {
     "query": "programming",
     "category": "tutorial",
     "difficulty": "beginner"
   }
   ```

3. **Check FTS5 index**:
   ```bash
   # Metadata should be in FTS index
   agentroot search "keyword_from_metadata"
   ```

## Best Practices

### 1. Initial Setup

```bash
# Start simple
agentroot collection add my-docs /path/to/docs

# Index with default settings
agentroot update

# Check results
agentroot status
agentroot metadata show #<docid>
```

### 2. Iterative Improvement

```bash
# If metadata quality is poor, upgrade to LLM
# Download model to ~/.local/share/agentroot/models/

# Regenerate with LLM
agentroot metadata refresh --all --force

# Compare results
agentroot metadata show #<docid>
```

### 3. Regular Maintenance

```bash
# Weekly: Update collections
agentroot update

# Monthly: Refresh metadata if model updated
agentroot metadata refresh --all

# Quarterly: Vacuum database
agentroot vacuum
```

### 4. Search Optimization

```bash
# Use metadata filters for precise results
agentroot search "topic" --category tutorial --difficulty beginner

# Leverage suggested queries
agentroot metadata show #<docid> | grep "Suggested Queries"
```

### 5. Performance Tuning

```bash
# For large collections (>10k docs):
# - Use selective patterns
# - Enable caching
# - Consider batch processing

# For real-time updates:
# - Use incremental indexing
# - Avoid --force flag
```

## Advanced Usage

### Programmatic Access (Rust API)

```rust
use agentroot_core::{Database, LlamaMetadataGenerator, MetadataGenerator};

// Create metadata generator
let generator = LlamaMetadataGenerator::from_default()?;

// Index with metadata
db.reindex_collection_with_metadata(
    "my-collection",
    Some(&generator as &dyn MetadataGenerator)
).await?;

// Search with metadata filters
let results = db.search_fts("query", &options)?;
for result in results {
    if let Some(summary) = &result.llm_summary {
        println!("Summary: {}", summary);
    }
}
```

### MCP Integration (AI Assistants)

When using Agentroot via MCP (Model Context Protocol):

```json
{
  "tool": "search",
  "arguments": {
    "query": "rust tutorial",
    "category": "tutorial",
    "difficulty": "beginner",
    "limit": 5
  }
}
```

AI assistants automatically see metadata in results:
```json
{
  "results": [
    {
      "docid": "#abc123",
      "title": "Rust Tutorial",
      "summary": "Learn Rust programming...",
      "category": "tutorial",
      "difficulty": "beginner",
      "keywords": ["rust", "programming", "tutorial"]
    }
  ]
}
```

## FAQ

**Q: Does metadata generation require an internet connection?**  
A: No. All metadata generation happens locally using your CPU.

**Q: How much disk space does metadata use?**  
A: Approximately 50-200 bytes per document (compressed). For 10,000 documents, expect ~0.5-2 MB overhead.

**Q: Can I delete metadata?**  
A: Not directly, but you can disable metadata generation and re-index without it.

**Q: Does metadata slow down searches?**  
A: No. Metadata is indexed alongside document content in FTS5, so searches remain fast.

**Q: Can I edit metadata manually?**  
A: Not currently supported. Metadata is regenerated from content.

**Q: What happens if I change the LLM model?**  
A: Run `agentroot metadata refresh --all --force` to regenerate metadata with the new model.

**Q: Is metadata multilingual?**  
A: Currently, metadata is generated in English. Multilingual support is planned.

## See Also

- [Metadata Performance Report](metadata-performance.md)
- [Getting Started Guide](getting-started.md)
- [MCP Server Documentation](mcp-server.md)
- [Troubleshooting Guide](troubleshooting.md)

## Support

For issues or questions:
- GitHub Issues: https://github.com/epappas/agentroot/issues
- Documentation: https://github.com/epappas/agentroot/tree/master/docs
