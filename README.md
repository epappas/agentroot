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
- **AI-Powered Search**: Natural language queries with LLM-based query understanding and metadata generation (optional vLLM integration)
- **Response Caching**: 7,000-10,000x speedup for repeated queries with intelligent cache management
- **AST-Aware Chunking**: Intelligently chunks code by semantic units (functions, classes, methods) using tree-sitter
- **Smart Cache Invalidation**: Content-addressable chunk hashing achieves 80-90% cache hit rates on re-indexing
- **Multi-Language Support**: Rust, Python, JavaScript/TypeScript, Go (with fallback for other languages)
- **Local-First or Cloud**: Run entirely offline with local models, or connect to [Basilica](https://basilica.ai) for GPU-accelerated inference
- **MCP Server**: Model Context Protocol support for AI assistant integration

### Powered by Basilica

AgentRoot integrates seamlessly with **[Basilica](https://basilica.ai)** ([GitHub](https://github.com/one-covenant/basilica)) - a trustless GPU compute marketplace built on Bittensor's decentralized infrastructure. Basilica provides production-grade AI inference with verified hardware, automatic failover, and 99.9% uptime. When connected to Basilica, AgentRoot achieves 10x faster embeddings and GPU-accelerated search while maintaining privacy through decentralized compute verification.

**Why Basilica works so well with AgentRoot:**
- ⚡ OpenAI-compatible API - zero custom integration needed
- 🔒 Trustless verification - binary validation of GPU compute
- 🚀 10x faster - GPU acceleration with intelligent load balancing
- 💾 Smart caching - AgentRoot + Basilica layers for 7,000x speedup
- 🌐 Decentralized - 100+ verified GPU nodes on Bittensor Subnet 39

See [VLLM_SETUP.md](VLLM_SETUP.md) for Basilica integration details.

## Installation

### From crates.io (Recommended)

```bash
cargo install agentroot
```

Verify installation:

```bash
agentroot --version
```

### From Source

```bash
git clone https://github.com/epappas/agentroot
cd agentroot
cargo build --release

# Install to PATH
cargo install --path crates/agentroot-cli
```

### Dependencies

Agentroot requires an embedding model for vector search. On first run, it will download nomic-embed-text-v1.5 (~100MB) to `~/.local/share/agentroot/models/`.

## Quick Start

### Option 1: Local-Only (Privacy-First)

```bash
# 1. Add a collection (index files from a directory)
agentroot collection add /path/to/your/code --name myproject --mask '**/*.rs'

# 2. Index the files
agentroot update

# 3. Generate embeddings (downloads model on first run)
agentroot embed

# 4. Search
agentroot search "error handling"      # BM25 full-text search
agentroot vsearch "error handling"     # Vector similarity search
agentroot query "error handling"       # Hybrid search (best quality)
```

### Option 2: AI-Powered with Basilica (Recommended)

```bash
# 1. Get Basilica endpoints at https://basilica.ai (instant access)
# 2. Configure endpoints (see VLLM_SETUP.md for details)
export AGENTROOT_LLM_URL="https://your-id.deployments.basilica.ai"
export AGENTROOT_LLM_MODEL="Qwen/Qwen2.5-7B-Instruct"
export AGENTROOT_EMBEDDING_URL="https://your-id.deployments.basilica.ai"
export AGENTROOT_EMBEDDING_MODEL="intfloat/e5-mistral-7b-instruct"
export AGENTROOT_EMBEDDING_DIMS="4096"

# 2. Add and index collection
agentroot collection add /path/to/your/code --name myproject
agentroot update

# 3. Generate embeddings (uses vLLM, 10x faster with GPU)
agentroot embed

# 4. Generate AI metadata (optional but recommended)
agentroot metadata refresh myproject

# 5. Smart natural language search
agentroot smart "show me files dealing with error handling"
```

**Benefits of Basilica Integration:**
- 🚀 10x faster with GPU acceleration (decentralized Bittensor network)
- 🧠 Smarter queries with LLM understanding
- 📊 Rich metadata generation
- ⚡ 7,000x speedup for cached queries
- 🔒 Trustless compute with hardware verification
- 🌐 99.9% uptime with automatic failover

See [Complete Workflow Guide](WORKFLOW.md) for step-by-step tutorials and [VLLM_SETUP.md](VLLM_SETUP.md) for Basilica setup.

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
| **URLProvider** | ✅ Available | Web pages and HTTP(S) documents |
| **PDFProvider** | ✅ Available | PDF document text extraction |
| **SQLProvider** | ✅ Available | SQLite database content indexing |
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

# Provider examples
cargo run -p agentroot-core --example github_provider  # GitHub repositories
cargo run -p agentroot-core --example url_provider     # Web pages/HTTP
cargo run -p agentroot-core --example pdf_provider     # PDF documents
cargo run -p agentroot-core --example sql_provider     # SQLite databases
cargo run -p agentroot-core --example custom_provider  # Custom provider template
```

All examples are production-ready, compile cleanly, and demonstrate real functionality. See [examples/README.md](examples/README.md) for details.

## Commands

| Command | Description | Speed | Quality |
|---------|-------------|-------|---------|
| `collection add <path>` | Add a new collection | - | - |
| `collection list` | List all collections | - | - |
| `collection remove <name>` | Remove a collection | - | - |
| `update` | Re-index all collections | Fast | - |
| `embed` | Generate vector embeddings | Medium | - |
| `metadata refresh` | Generate AI metadata (vLLM) | Medium | - |
| `search <query>` | BM25 full-text search | ⚡ <10ms | ⭐⭐⭐ |
| `vsearch <query>` | Vector similarity search | ~100ms | ⭐⭐⭐⭐ |
| `query <query>` | Hybrid search with RRF | ~150ms | ⭐⭐⭐⭐⭐ |
| `smart <query>` | AI natural language search (vLLM) | ~150ms* | ⭐⭐⭐⭐⭐ |
| `get <docid>` | Get document by path or docid | <1ms | - |
| `multi-get <pattern>` | Get multiple documents | <10ms | - |
| `ls [collection]` | List files in a collection | <1ms | - |
| `status` | Show index status | <1ms | - |
| `mcp` | Start MCP server for AI integration | - | - |

*First query ~1.5s, cached queries ~150ms (10x faster)

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
# Keyword search (fast, <10ms)
agentroot search "Result<T>"

# Semantic search (understands meaning, ~100ms)
agentroot vsearch "how to handle database errors"

# Hybrid search (best quality, ~150ms)
agentroot query "error handling patterns in async code"

# AI natural language search (with vLLM, understands complex queries)
agentroot smart "show me all files that deal with async error handling"
```

Example output:
```
🤖 Parsed query: async error handling
📊 Search type: Hybrid
🔍 Expanded terms: error handling, async, Result, tokio

  94% src/async/error.rs #a1b2c3
  Async error handling utilities with retry and backoff

  91% src/api/handlers.rs #d4e5f6
  HTTP handlers with async error propagation

  87% src/database/pool.rs #g7h8i9
  Connection pool error recovery strategies
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
- Embedding (local): ~50-100 chunks/second (CPU-dependent)
- Embedding (vLLM): ~200-500 chunks/second (GPU-accelerated)

### Search Speed

| Operation | First Query | Cached Query | Speedup |
|-----------|-------------|--------------|---------|
| BM25 search | <10ms | <10ms | 1x |
| Vector search | ~100ms | ~100ms | 1x |
| Hybrid search | ~150ms | ~150ms | 1x |
| Smart search (vLLM) | ~1500ms | ~150ms | **10x** |
| Embedding (vLLM) | 600ms | 80µs | **7,500x** |

### Response Caching (vLLM)

AgentRoot intelligently caches LLM responses and embeddings:

```
Cache Performance:
  Embedding cache:   7,000-10,000x speedup (600ms → 80µs)
  Query cache:       10x speedup (1.5s → 0.15s)
  TTL:               1 hour (auto-expiration)
  Thread-safe:       Concurrent access supported
```

### Chunk Cache Efficiency

```
Initial indexing:  0% cache hits (all chunks computed)
Minor edits:       90-95% cache hits
Feature additions: 80-90% cache hits
Major refactor:    60-80% cache hits
```

**Real-World Example:**
```bash
# Test caching yourself
cargo run --release --example test_cache

# Output:
# First embed:  632ms  (cache miss)
# Second embed: 80µs   (cache hit - 7,900x faster!)
```

See [Performance Documentation](docs/performance.md) for detailed benchmarks.

## Configuration

### Database Location

```
~/.cache/agentroot/index.sqlite
```

### Model Location (Local Mode)

```
~/.local/share/agentroot/models/
```

### Environment Variables

#### Basic Configuration

```bash
# Override database path
export AGENTROOT_DB=/custom/path/index.sqlite

# Override models directory (local mode)
export AGENTROOT_MODELS=/custom/path/models

# Set log level
export RUST_LOG=debug
```

#### Basilica Integration (Optional - Recommended)

For AI-powered features with Basilica's decentralized GPU network:

```bash
# Get endpoints at https://basilica.ai (instant access)

# LLM Service (for query parsing, metadata generation)
export AGENTROOT_LLM_URL="https://your-id.deployments.basilica.ai"
export AGENTROOT_LLM_MODEL="Qwen/Qwen2.5-7B-Instruct"

# Embedding Service (for vector search)
export AGENTROOT_EMBEDDING_URL="https://your-id.deployments.basilica.ai"
export AGENTROOT_EMBEDDING_MODEL="intfloat/e5-mistral-7b-instruct"
export AGENTROOT_EMBEDDING_DIMS="4096"

# Optional: Timeouts
export AGENTROOT_LLM_TIMEOUT="120"
```

**When to use Basilica:**
- ✅ Want GPU-accelerated search (10x faster)
- ✅ Need AI metadata generation
- ✅ Natural language queries
- ✅ Trust decentralized compute verification
- ✅ Team with shared infrastructure
- ✅ Production reliability (99.9% uptime)

**When to use Local:**
- ✅ Privacy-critical code (air-gapped)
- ✅ Offline development
- ✅ No external dependencies

See [VLLM_SETUP.md](VLLM_SETUP.md) for complete Basilica integration guide.

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
- ⚡ [**Quick Reference**](QUICKSTART.md) - Fast 30-second start (NEW!)
- 🚀 [**End-to-End Workflow**](WORKFLOW.md) - Complete real-world tutorial (NEW!)
- [Getting Started](docs/getting-started.md) - Step-by-step tutorial for new users
- [vLLM Setup Guide](VLLM_SETUP.md) - Configure AI-powered features (NEW!)
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

**Development:**
- [TODO](TODO.md) - Known issues and planned improvements
- [AGENTS.md](AGENTS.md) - Guidelines for AI coding agents

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
