# Getting Started with Agentroot

This guide walks you through installing and using Agentroot for the first time.

## What is Agentroot?

Agentroot is a local semantic search engine for codebases and knowledge bases. It combines:

- **BM25 full-text search** for exact keyword matching
- **Vector similarity search** for semantic understanding
- **AST-aware chunking** to keep code functions intact
- **Smart caching** for 5-10x faster re-indexing

Everything runs locally on your machine. No API keys, no cloud services.

## Installation

### Build from Source

```bash
git clone https://github.com/epappas/agentroot
cd agentroot
cargo build --release

# Install to PATH
cargo install --path crates/agentroot-cli
```

Verify installation:

```bash
agentroot --version
# Output: agentroot 0.1.0
```

### First Run: Model Download

On first run, Agentroot downloads an embedding model (nomic-embed-text-v1.5, ~100MB) to:

```
~/.local/share/agentroot/models/
```

This happens automatically when you run `agentroot embed` for the first time.

## Basic Workflow

The typical workflow has 4 steps:

1. **Add collection** - Tell Agentroot which directory to index
2. **Update index** - Scan files and extract content
3. **Generate embeddings** - Create vector embeddings for semantic search
4. **Search** - Find what you need

## Step-by-Step Example

Let's index a Rust project and search it.

### Step 1: Add a Collection

A collection is a set of files you want to search. Add your first collection:

```bash
agentroot collection add /path/to/your/rust/project --name myproject --mask '**/*.rs'
```

Options explained:
- `--name myproject` - Name your collection (defaults to directory name)
- `--mask '**/*.rs'` - Only index Rust files (glob pattern)

You can add multiple collections:

```bash
agentroot collection add ~/Documents/notes --name notes --mask '**/*.md'
agentroot collection add ~/code/python-app --name pyapp --mask '**/*.py'
```

List your collections:

```bash
agentroot collection list
# Output:
# myproject: /path/to/your/rust/project (**/*.rs) [provider: file, 0 documents]
# notes: /home/user/Documents/notes (**/*.md) [provider: file, 0 documents]
```

#### Using Different Providers

Agentroot can index content from multiple sources:

**Local Files (default)**:
```bash
agentroot collection add /path/to/code --name mycode --mask '**/*.rs'
```

**GitHub Repositories**:
```bash
agentroot collection add https://github.com/rust-lang/rust \
  --name rust-docs \
  --mask '**/*.md' \
  --provider github
```

**With GitHub Authentication** (for higher rate limits and private repos):
```bash
# Option 1: Use environment variable
export GITHUB_TOKEN=ghp_your_token_here
agentroot collection add https://github.com/rust-lang/rust \
  --name rust-docs \
  --mask '**/*.md' \
  --provider github

# Option 2: Pass token in config (more explicit)
agentroot collection add https://github.com/rust-lang/rust \
  --name rust-docs \
  --mask '**/*.md' \
  --provider github \
  --config '{"github_token":"ghp_your_token_here"}'
```

**Web Pages**:
```bash
agentroot collection add https://doc.rust-lang.org/book/ \
  --name rust-book \
  --provider url
```

**PDF Documents**:
```bash
agentroot collection add ~/Documents/papers \
  --name research \
  --mask '**/*.pdf' \
  --provider pdf
```

**SQLite Databases**:
```bash
# Index a table
agentroot collection add ~/blog.sqlite \
  --name blog-posts \
  --provider sql \
  --config '{"table":"posts"}'

# Or use a custom query
agentroot collection add ~/blog.sqlite \
  --name published-posts \
  --provider sql \
  --config '{"query":"SELECT id, title, content FROM posts WHERE published = 1"}'
```

See [Provider Documentation](providers.md) for complete details on available providers and how to use them.

### Step 2: Index Files

Scan the filesystem and extract content:

```bash
agentroot update
```

What happens:
- Scans directories in all collections
- Applies file masks (glob patterns)
- Extracts file content and metadata
- Computes SHA-256 hashes for deduplication
- Stores in SQLite database at `~/.cache/agentroot/index.sqlite`

Output example:

```
Updating myproject...
myproject: 42 files indexed
Updating notes...
notes: 15 files indexed
Done (2/2)
```

Check index status:

```bash
agentroot status
# Output:
# Collections: 2
# Documents:   57
# Embedded:    0
# Pending:     57
```

### Step 3: Generate Embeddings

Create vector embeddings for semantic search:

```bash
agentroot embed
```

What happens:
- Loads embedding model (downloads on first run)
- Chunks documents using AST-aware semantic chunking
- Computes embeddings for each chunk
- Caches embeddings by content hash (blake3)
- Stores in database

Output example:

```
Loading embedding model: nomic-embed-text-v1.5.Q4_K_M (768 dimensions)
Embedding: 57/57 docs, 523 chunks
  Cached: 0 (0.0%)
  Computed: 523 (100.0%)
Done in 42s
```

On subsequent runs, the cache speeds things up dramatically:

```bash
agentroot embed
# After minor edits:
# Cached: 490 (93.7%)
# Computed: 33 (6.3%)
# Done in 3s
```

### Step 4: Search

Now you can search your indexed content.

#### BM25 Full-Text Search

Fast keyword search using SQLite FTS5:

```bash
agentroot search "error handling"
```

Output:

```
 87% myproject/src/error.rs #a1b2c3
 72% myproject/src/handler.rs #d4e5f6
 68% myproject/src/main.rs #789abc
```

Search in specific collection:

```bash
agentroot search "function" -c myproject
```

Get more results:

```bash
agentroot search "database" -n 20
```

Show full document content:

```bash
agentroot search "config" --full
```

#### Vector Similarity Search

Semantic search using embeddings:

```bash
agentroot vsearch "how to handle errors"
```

This finds semantically similar content even if exact keywords don't match.

#### Hybrid Search (Best Quality)

Combines BM25 and vector search with Reciprocal Rank Fusion:

```bash
agentroot query "error handling"
```

Hybrid search provides the best results by leveraging both:
- BM25 for exact keyword matches
- Vector search for semantic understanding
- RRF fusion to combine rankings intelligently

### Step 5: Retrieve Documents

Get a specific document by its ID (from search results):

```bash
agentroot get "#a1b2c3"
```

Or by path:

```bash
agentroot get myproject/src/error.rs
```

Get multiple files with glob pattern:

```bash
agentroot multi-get "myproject/src/*.rs"
```

List files in a collection:

```bash
agentroot ls myproject
```

## Common Patterns

### Excluding Files

Exclude directories or file patterns:

```bash
agentroot collection add ./src --name code \
  --mask '**/*.rs' \
  --exclude '**/target/**' \
  --exclude '**/test/**'
```

### Multiple File Types

Index multiple file types in one collection:

```bash
agentroot collection add ./docs --name documentation \
  --mask '**/*.md' \
  --mask '**/*.txt' \
  --mask '**/*.rst'
```

### Re-indexing After Changes

After editing files, update the index:

```bash
agentroot update
agentroot embed
```

The cache will reuse embeddings for unchanged chunks, making this fast.

### Git Integration

Pull latest changes before indexing:

```bash
agentroot update --pull
```

This runs `git pull` in each collection that's a git repository.

## Output Formats

All commands support multiple output formats:

### JSON

For programmatic consumption:

```bash
agentroot search "query" --format json
```

### CSV

For spreadsheet import:

```bash
agentroot search "query" --format csv
```

### Markdown

For documentation:

```bash
agentroot search "query" --format md
```

### Files Only

For piping to other commands:

```bash
agentroot search "query" --format files | xargs cat
```

## Understanding Search Quality

### When to Use Each Search Type

**BM25 (`search`)**:
- Exact keyword matching
- Technical terms, function names, identifiers
- Fast (< 10ms)
- Example: `search "fn process_data"`

**Vector (`vsearch`)**:
- Semantic understanding
- Natural language queries
- Concept-based search
- Example: `vsearch "how to parse JSON"`

**Hybrid (`query`)**:
- Best overall quality
- Combines both approaches
- Use this as your default
- Example: `query "error handling patterns"`

### FTS5 Query Syntax

The `search` command supports SQLite FTS5 operators:

```bash
# Phrase search (exact match)
agentroot search '"error handling"'

# OR operator
agentroot search "error OR exception"

# NOT operator
agentroot search "error NOT warning"

# Prefix search
agentroot search "hand*"

# NEAR operator (terms within N words)
agentroot search "NEAR(error handling, 5)"

# Column filter
agentroot search "title:README"
```

Note: Hyphens can cause issues (interpreted as NOT). Use quotes for hyphenated terms:

```bash
agentroot search '"tree-sitter"'
```

## File Locations

Understanding where Agentroot stores data:

### Database

```
~/.cache/agentroot/index.sqlite
```

This contains:
- Document metadata
- Content (deduplicated by SHA-256 hash)
- Vector embeddings
- FTS5 index
- Chunk embedding cache

### Models

```
~/.local/share/agentroot/models/
```

Downloaded embedding models are stored here.

### Override Paths

Use environment variables to override defaults:

```bash
export AGENTROOT_DB=/custom/path/index.sqlite
export AGENTROOT_MODELS=/custom/path/models
```

## Next Steps

Now that you understand the basics:

1. **Read [CLI Reference](cli-reference.md)** for complete command documentation
2. **Learn about [AST-Aware Chunking](semantic-chunking.md)** to understand why search quality is better
3. **Explore [Architecture](architecture.md)** to understand how it works
4. **Check [Troubleshooting](troubleshooting.md)** if you encounter issues
5. **Try [MCP Server](mcp-server.md)** to integrate with AI assistants

## Quick Reference

### Essential Commands

```bash
# Add collection
agentroot collection add <path> --name <name> --mask <pattern>

# Update index
agentroot update

# Generate embeddings
agentroot embed

# Search (hybrid, best quality)
agentroot query <query>

# Search (BM25 full-text)
agentroot search <query>

# Search (vector similarity)
agentroot vsearch <query>

# Get document
agentroot get <path-or-docid>

# List collections
agentroot collection list

# Show status
agentroot status
```

### Typical Workflow

```bash
# Initial setup
agentroot collection add ~/myproject --name myproject --mask '**/*.rs'
agentroot update
agentroot embed

# Daily usage
agentroot query "what I'm looking for"

# After making changes
agentroot update
agentroot embed
```

## Tips

1. **Use hybrid search by default** - `query` provides the best results
2. **Cache makes re-indexing fast** - Don't worry about running `update` and `embed` frequently
3. **Specific collections** - Use `-c <collection>` to narrow search scope
4. **File masks are powerful** - Use glob patterns to index exactly what you need
5. **Exclude tests** - Typically you want `--exclude '**/test/**'` to avoid test files
6. **Check status** - Run `agentroot status` to see what's indexed and what's pending
