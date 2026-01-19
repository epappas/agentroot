# Agentroot

Fast local semantic search for your codebase and knowledge base. Agentroot provides hybrid search combining BM25 full-text search with vector similarity search, powered by AST-aware semantic chunking for code files.

## Why Agentroot?

Traditional code search tools fall short in several ways:

### The Problem

**Keyword search** (grep, ripgrep, GitHub search):
- Finds exact matches only
- Misses semantically similar code
- Splits functions at arbitrary boundaries
- No understanding of code structure

**Naive semantic search**:
- Chunks text at character boundaries
- Breaks functions mid-implementation
- Loses context (docstrings, comments)
- Poor embedding quality

### The Solution

Agentroot solves these problems with:

#### 1. AST-Aware Semantic Chunking

Code files are parsed with tree-sitter and chunked by semantic units (functions, classes, methods):

```
Traditional chunking:              Agentroot (AST-aware):
─────────────────────              ───────────────────────

fn process_data() {                /// Process input data
    let x = parse();               fn process_data() {
    let y = validate();                let x = parse();
} ← Split here!                        let y = validate();
                                       transform(x, y)
fn next_function() {               }  ← Kept intact
```

Benefits:
- Functions stay intact
- Context preserved (docstrings, comments)
- Better embedding quality
- More accurate search results

#### 2. Smart Content-Addressable Caching

Each chunk gets a blake3 hash based on its content and context. On re-indexing:

```
Edit 1 function out of 100:
❌ Without cache: Re-embed all 100 functions (30s)
✅ With cache: Re-embed 1 function (0.3s)

Typical cache hit rates:
- Minor edits: 90-95%
- Feature additions: 80-90%
- Major refactoring: 60-80%
```

Result: **5-10x faster re-indexing** for typical development workflows.

#### 3. Hybrid Search with RRF

Combines the best of both worlds:

- **BM25**: Fast exact keyword matching (<10ms)
- **Vector search**: Semantic understanding (~100ms)
- **RRF fusion**: Intelligently combines rankings

```bash
Query: "error handling patterns"

BM25 finds:
- Exact matches: "error", "handling"
- Technical terms: "Result<T>", "anyhow"

Vector search finds:
- Semantic matches: exception handling code
- Similar patterns without exact keywords
- Related concepts

Hybrid combines both for best results
```

#### 4. Privacy-First Local Operation

- All data stays on your machine
- No API keys required
- No cloud services
- Works completely offline (after model download)

### Comparison to Alternatives

| Feature | Agentroot | ripgrep | GitHub Search | Semantic Code Search |
|---------|-----------|---------|---------------|---------------------|
| Keyword search | ✅ BM25 | ✅ Fast | ✅ Advanced | ⚠️ Limited |
| Semantic search | ✅ Hybrid | ❌ No | ❌ No | ✅ Yes |
| AST-aware chunking | ✅ Yes | ❌ No | ❌ No | ⚠️ Varies |
| Local-first | ✅ Yes | ✅ Yes | ❌ Cloud | ⚠️ Varies |
| Smart caching | ✅ 80-90% hit | N/A | N/A | ❌ No |
| Speed (keyword) | ✅ <10ms | ✅ <10ms | ⚠️ 100ms+ | ❌ Slow |
| Speed (semantic) | ✅ ~100ms | ❌ N/A | ❌ N/A | ⚠️ 500ms+ |
| Setup complexity | ✅ One command | ✅ None | ⚠️ OAuth | ⚠️ Complex |

**When to use Agentroot**:
- You want semantic understanding of code
- You need privacy (local-first)
- You frequently re-index (cache helps)
- You want best-of-both-worlds hybrid search

**When to use alternatives**:
- ripgrep: Pure keyword search, need maximum speed
- GitHub Search: Already on GitHub, want web interface
- Other tools: Specific enterprise requirements

## Features

- **Multi-Source Indexing**: Pluggable provider system for indexing from local files, GitHub repositories, URLs, databases, and more
- **Hybrid Search**: Combines BM25 full-text search with vector similarity search using Reciprocal Rank Fusion
- **AST-Aware Chunking**: Intelligently chunks code by semantic units (functions, classes, methods) using tree-sitter
- **Smart Cache Invalidation**: Content-addressable chunk hashing achieves 80-90% cache hit rates on re-indexing
- **Multi-Language Support**: Rust, Python, JavaScript/TypeScript, Go (with fallback for other languages)
- **Local-First**: All data stays on your machine, runs entirely offline
- **MCP Server**: Model Context Protocol support for AI assistant integration

## Installation

### From Source

```bash
git clone https://github.com/spacejar/agentroot
cd agentroot
cargo build --release

# Install to PATH
cargo install --path crates/agentroot-cli
```

Verify installation:

```bash
agentroot --version
```

### Dependencies

Agentroot requires an embedding model for vector search. On first run, it will download nomic-embed-text-v1.5 (~100MB) to `~/.local/share/agentroot/models/`.

## Quick Start

```bash
# 1. Add a collection (index files from a directory)
agentroot collection add /path/to/your/code --name myproject --mask '**/*.rs'

# 2. Index the files
agentroot update

# 3. Generate embeddings
agentroot embed

# 4. Search
agentroot search "error handling"      # BM25 full-text search
agentroot vsearch "error handling"     # Vector similarity search
agentroot query "error handling"       # Hybrid search (best quality)
```

See [Getting Started Guide](docs/getting-started.md) for detailed walkthrough.

## Multi-Source Indexing

Agentroot can index content from multiple sources beyond local files using its pluggable provider system:

### Local Files (Default)

```bash
# Add local directory
agentroot collection add /path/to/code --name myproject --mask '**/*.rs'
```

### GitHub Repositories

```bash
# Add GitHub repository
agentroot collection add https://github.com/rust-lang/rust \
  --name rust-lang \
  --mask '**/*.md' \
  --provider github

# Optionally provide GitHub token for higher rate limits
export GITHUB_TOKEN=ghp_your_token_here
```

### Provider Architecture

The provider system is extensible and designed to support:

| Provider | Status | Description |
|----------|--------|-------------|
| **FileProvider** | ✅ Available | Local file system with glob patterns |
| **GitHubProvider** | ✅ Available | GitHub repositories and files |
| **URLProvider** | 🔄 Planned | Web pages and documents |
| **PDFProvider** | 🔄 Planned | PDF document extraction |
| **SQLProvider** | 🔄 Planned | Database content indexing |
| **CalendarProvider** | 🔄 Planned | Calendar events and notes |

Adding a new provider is simple - implement the `SourceProvider` trait and register it. See [Provider Documentation](docs/providers.md) for details.

### Using Providers in Code

```rust
use agentroot_core::{Database, GitHubProvider, ProviderConfig};

let db = Database::open("index.db")?;
db.initialize()?;

// Add GitHub collection
db.add_collection(
    "rust-docs",
    "https://github.com/rust-lang/rust",
    "**/*.md",
    "github",
    None,
)?;

// Index using provider
db.reindex_collection("rust-docs")?;
```

See [examples/github_provider.rs](examples/github_provider.rs) for a complete working example.

## Code Examples

Working code examples demonstrating library usage are available in [`examples/`](examples/):

```bash
# Basic search example (database setup, indexing, BM25 search)
cargo run -p agentroot-core --example basic_search

# Semantic chunking example (AST-aware code parsing)
cargo run -p agentroot-core --example semantic_chunking

# Custom indexing pipeline example
cargo run -p agentroot-core --example custom_index

# GitHub provider example (indexing from GitHub repositories)
cargo run -p agentroot-core --example github_provider
```

All examples are production-ready, compile cleanly, and demonstrate real functionality. See [examples/README.md](examples/README.md) for details.

## Commands

| Command | Description |
|---------|-------------|
| `collection add <path>` | Add a new collection |
| `collection list` | List all collections |
| `collection remove <name>` | Remove a collection |
| `update` | Re-index all collections |
| `embed` | Generate vector embeddings |
| `search <query>` | BM25 full-text search |
| `vsearch <query>` | Vector similarity search |
| `query <query>` | Hybrid search with reranking |
| `get <docid>` | Get document by path or docid |
| `ls [collection]` | List files in a collection |
| `status` | Show index status |
| `mcp` | Start MCP server for AI integration |

See [CLI Reference](docs/cli-reference.md) for complete documentation.

## Example Usage

### Index a Rust Project

```bash
agentroot collection add ~/projects/myapp --name myapp \
  --mask '**/*.rs' \
  --exclude '**/target/**'

agentroot update
agentroot embed
```

### Search for Error Handling Patterns

```bash
# Keyword search (fast)
agentroot search "Result<T>"

# Semantic search (understands meaning)
agentroot vsearch "how to handle database errors"

# Hybrid search (best quality)
agentroot query "error handling patterns in async code"
```

### Retrieve Specific Files

```bash
# By path
agentroot get myapp/src/error.rs

# By docid (from search results)
agentroot get "#a1b2c3"

# Multiple files
agentroot multi-get "myapp/src/*.rs"
```

### Integration with AI Assistants

Start MCP server for Claude Desktop or Continue.dev:

```bash
agentroot mcp
```

See [MCP Server Documentation](docs/mcp-server.md) for integration details.

## Architecture

```
agentroot/
├── agentroot-core/     # Core library
│   ├── db/             # SQLite database layer
│   ├── index/          # Indexing and chunking
│   │   └── ast_chunker/  # AST-aware semantic chunking
│   ├── providers/      # Pluggable content sources
│   ├── search/         # Search algorithms
│   └── llm/            # Embedding model integration
├── agentroot-cli/      # Command-line interface
├── agentroot-mcp/      # MCP server for AI assistants
└── agentroot-tui/      # Terminal UI (experimental)
```

### Key Components

**AST Chunker**: Uses tree-sitter to parse code and extract semantic units. Supports Rust, Python, JavaScript, TypeScript, and Go.

**Embedding Cache**: blake3-hashed chunks enable smart cache invalidation. Only changed chunks are re-embedded, achieving 80-90% cache hit rates.

**Hybrid Search**: Reciprocal Rank Fusion combines BM25 (keyword) and vector (semantic) results for optimal quality.

**SQLite Storage**: FTS5 for full-text search, BLOB storage for embeddings, content-addressable deduplication.

See [Architecture Documentation](docs/architecture.md) for detailed design.

## Supported Languages

| Language | File Extensions | Semantic Units |
|----------|----------------|----------------|
| Rust | `.rs` | functions, impl blocks, structs, enums, traits, modules |
| Python | `.py` | functions, classes, decorated definitions |
| JavaScript | `.js`, `.jsx` | functions, classes, methods, arrow functions |
| TypeScript | `.ts`, `.tsx` | functions, classes, interfaces, type aliases |
| Go | `.go` | functions, methods, types, interfaces |
| Other | `*` | Character-based chunking (fallback) |

See [Semantic Chunking Documentation](docs/semantic-chunking.md) for technical details.

## Performance

### Indexing Speed

- Scanning: ~1000 files/second
- AST parsing: ~1-5ms per file
- Embedding: ~50-100 chunks/second (CPU-dependent)

### Search Speed

- BM25 search: <10ms for typical queries
- Vector search: <100ms for 10K chunks
- Hybrid search: <150ms combined

### Cache Efficiency

```
Initial indexing:  0% cache hits (all chunks computed)
Minor edits:       90-95% cache hits
Feature additions: 80-90% cache hits
Major refactor:    60-80% cache hits
```

See [Performance Documentation](docs/performance.md) for benchmarks.

## Configuration

### Database Location

```
~/.cache/agentroot/index.sqlite
```

### Model Location

```
~/.local/share/agentroot/models/
```

### Environment Variables

```bash
# Override database path
export AGENTROOT_DB=/custom/path/index.sqlite

# Override models directory
export AGENTROOT_MODELS=/custom/path/models

# Set log level
export RUST_LOG=debug
```

## Development

```bash
# Build all workspace members
cargo build

# Run tests
cargo test

# Run with debug logging
RUST_LOG=debug cargo run --bin agentroot -- status

# Run clippy
cargo clippy --all-targets --all-features

# Format code
cargo fmt
```

See [AGENTS.md](AGENTS.md) for developer guidelines.

## Documentation

**Start Here:**
- [Getting Started](docs/getting-started.md) - Step-by-step tutorial for new users
- [How-To Guide](docs/howto-guide.md) - Practical recipes for common tasks

**Reference:**
- [CLI Reference](docs/cli-reference.md) - Complete command reference
- [Provider System](docs/providers.md) - Multi-source indexing guide (files, GitHub, etc.)
- [Troubleshooting](docs/troubleshooting.md) - Common issues and solutions

**Technical Details:**
- [Architecture](docs/architecture.md) - System design and components
- [Semantic Chunking](docs/semantic-chunking.md) - AST-aware chunking details
- [Embedding Cache](docs/embedding-cache.md) - Smart cache invalidation
- [Performance](docs/performance.md) - Benchmarks and optimization

**Integration:**
- [MCP Server](docs/mcp-server.md) - AI assistant integration (Claude, Continue.dev)

**Index:**
- [Documentation Index](docs/README.md) - Complete documentation overview

## Contributing

Contributions are welcome! See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

MIT License - see [LICENSE](LICENSE) for details.

## Acknowledgments

Built with:
- [tree-sitter](https://tree-sitter.github.io/) - AST parsing
- [llama.cpp](https://github.com/ggerganov/llama.cpp) - Embedding model inference
- [SQLite](https://www.sqlite.org/) with FTS5 - Database and full-text search
- [blake3](https://github.com/BLAKE3-team/BLAKE3) - Content hashing

Embedding model: [nomic-embed-text-v1.5](https://huggingface.co/nomic-ai/nomic-embed-text-v1.5) by Nomic AI.
