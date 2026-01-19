# Agentroot

Fast local semantic search for your codebase and knowledge base. Agentroot provides hybrid search combining BM25 full-text search with vector similarity search, powered by AST-aware semantic chunking for code files.

## Features

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

### Dependencies

Agentroot requires an embedding model for vector search. On first run, it will download `nomic-embed-text-v1.5` (~100MB) to `~/.local/share/agentroot/models/`.

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

See [CLI Reference](docs/cli-reference.md) for detailed usage.

## Architecture

```
agentroot/
├── agentroot-core/     # Core library
│   ├── db/             # SQLite database layer
│   ├── index/          # Indexing and chunking
│   │   └── ast_chunker/  # AST-aware semantic chunking
│   ├── search/         # Search algorithms
│   └── llm/            # Embedding model integration
├── agentroot-cli/      # Command-line interface
├── agentroot-mcp/      # MCP server for AI assistants
└── agentroot-tui/      # Terminal UI (experimental)
```

### Key Components

- **AST Chunker**: Uses tree-sitter to parse code and extract semantic units
- **Embedding Cache**: blake3-hashed chunks enable smart cache invalidation
- **Hybrid Search**: RRF-based fusion of BM25 and vector search results
- **SQLite Storage**: FTS5 for full-text search, BLOB storage for embeddings

See [Architecture](docs/architecture.md) for detailed documentation.

## Supported Languages

| Language | File Extensions | Semantic Units |
|----------|----------------|----------------|
| Rust | `.rs` | functions, impl blocks, structs, enums, traits, modules |
| Python | `.py` | functions, classes, decorated definitions |
| JavaScript | `.js`, `.jsx` | functions, classes, methods, arrow functions |
| TypeScript | `.ts`, `.tsx` | functions, classes, interfaces, type aliases |
| Go | `.go` | functions, methods, types, interfaces |
| Other | `*` | Character-based chunking (fallback) |

## Configuration

### Database Location

```
~/.cache/agentroot/index.sqlite
```

### Model Location

```
~/.local/share/agentroot/models/
```

### Collection Masks

Filter files using glob patterns:

```bash
# Only Rust files
agentroot collection add ./src --name rust-code --mask '**/*.rs'

# Multiple patterns
agentroot collection add ./docs --name docs --mask '**/*.md' --mask '**/*.txt'

# Exclude patterns
agentroot collection add ./src --name code --exclude '**/test/**'
```

## MCP Server

Agentroot includes an MCP (Model Context Protocol) server for integration with AI assistants:

```bash
# Start MCP server
agentroot mcp

# Or with specific socket
agentroot mcp --socket /tmp/agentroot.sock
```

See [MCP Server](docs/mcp-server.md) for integration details.

## Performance

### Embedding Cache

The smart cache system achieves high hit rates through content-addressable hashing:

```
Initial indexing:  0% cache hits (all chunks computed)
Minor edits:       85-95% cache hits
Major refactor:    60-80% cache hits
```

### Search Performance

- BM25 search: <10ms for typical queries
- Vector search: <100ms for 10k chunks
- Hybrid search: <150ms combined

## Development

```bash
# Run tests
cargo test

# Run with debug logging
RUST_LOG=debug cargo run --bin agentroot -- status

# Run clippy
cargo clippy

# Format code
cargo fmt
```

## License

MIT License - see [LICENSE](LICENSE) for details.

## Related Documentation

- [Architecture](docs/architecture.md) - System design and components
- [CLI Reference](docs/cli-reference.md) - Complete command reference
- [Semantic Chunking](docs/semantic-chunking.md) - How AST-aware chunking works
- [Embedding Cache](docs/embedding-cache.md) - Smart cache invalidation system
