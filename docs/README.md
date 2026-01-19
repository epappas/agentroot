# Agentroot Documentation

Complete documentation for Agentroot - local semantic search for codebases and knowledge bases.

## Quick Links

- **[Getting Started Guide](getting-started.md)** - New to Agentroot? Start here
- **[How-To Guide](howto-guide.md)** - Practical recipes for common tasks
- **[CLI Reference](cli-reference.md)** - Complete command reference

## Documentation Index

### User Guides

| Guide | Description |
|-------|-------------|
| [Getting Started](getting-started.md) | Installation, first steps, basic workflow |
| [How-To Guide](howto-guide.md) | Practical examples and common workflows |
| [CLI Reference](cli-reference.md) | Complete command-line interface documentation |

### Features

| Document | Description |
|----------|-------------|
| [Provider System](providers.md) | Multi-source indexing (files, GitHub, URLs, PDFs, databases) |
| [Semantic Chunking](semantic-chunking.md) | AST-aware code parsing and chunking |
| [Embedding Cache](embedding-cache.md) | Content-addressable caching system |
| [MCP Server](mcp-server.md) | Model Context Protocol integration for AI assistants |
| [Performance](performance.md) | Performance tuning for large codebases (10K-100K+ files) |

### Reference

| Document | Description |
|----------|-------------|
| [Architecture](architecture.md) | System design and technical overview |
| [Performance](performance.md) | Performance characteristics and optimization |
| [Troubleshooting](troubleshooting.md) | Solutions to common problems |

## Documentation Structure

```
docs/
├── README.md                  # This file
├── getting-started.md         # New user guide
├── howto-guide.md            # Practical examples
├── cli-reference.md          # Command reference
├── providers.md              # Multi-source indexing
├── semantic-chunking.md      # AST-aware chunking
├── embedding-cache.md        # Caching system
├── mcp-server.md            # AI assistant integration
├── architecture.md          # Technical design
├── performance.md           # Performance guide
└── troubleshooting.md       # Problem solving
```

## Quick Start

```bash
# 1. Install
cargo install --path crates/agentroot-cli

# 2. Add a collection
agentroot collection add /path/to/code --name myproject

# 3. Index files
agentroot update

# 4. Generate embeddings
agentroot embed

# 5. Search
agentroot query "what you're looking for"
```

See [Getting Started Guide](getting-started.md) for detailed walkthrough.

## Key Features

- **Multi-Source Indexing**: Index from local files, GitHub, URLs, and more
- **Hybrid Search**: Combines BM25 full-text with vector similarity search
- **AST-Aware Chunking**: Keeps code functions intact for better embeddings
- **Smart Caching**: 80-90% cache hit rates on re-indexing
- **Local-First**: All data stays on your machine
- **MCP Integration**: Works with Claude and other AI assistants

## Common Tasks

| Task | Command |
|------|---------|
| Add local directory | `agentroot collection add /path --name myproject` |
| Add GitHub repo | `agentroot collection add https://github.com/owner/repo --provider github --name repo` |
| Add PDF directory | `agentroot collection add /path/to/pdfs --mask '**/*.pdf' --provider pdf --name pdfs` |
| Add SQLite database | `agentroot collection add database.db --provider sql --config '{"table":"articles"}' --name articles` |
| Update index | `agentroot update` |
| Generate embeddings | `agentroot embed` |
| Search (BM25) | `agentroot search "keyword"` |
| Search (vector) | `agentroot vsearch "natural language query"` |
| Search (hybrid) | `agentroot query "best quality search"` |
| Filter by provider | `agentroot search "keyword" --provider pdf` |
| List collections | `agentroot collection list` |
| Check status | `agentroot status` |

## Need Help?

1. **First time?** → [Getting Started Guide](getting-started.md)
2. **How do I...?** → [How-To Guide](howto-guide.md)
3. **Command details?** → [CLI Reference](cli-reference.md)
4. **Something broken?** → [Troubleshooting](troubleshooting.md)
5. **Still stuck?** → [Open an issue](https://github.com/spacejar/agentroot/issues)

## Examples

Working code examples are in [`../examples/`](../examples/):

```bash
# Core examples
cargo run --example basic_search           # Database setup and BM25 search
cargo run --example semantic_chunking      # AST-aware code parsing
cargo run --example custom_index           # Custom indexing pipeline

# Provider examples
cargo run --example github_provider        # Index GitHub repositories
cargo run --example url_provider          # Index web pages
cargo run --example pdf_provider          # Index PDF documents
cargo run --example sql_provider          # Index SQLite databases
cargo run --example custom_provider       # Custom provider template
```

See [`../examples/README.md`](../examples/README.md) for detailed descriptions.

## Contributing

See [CONTRIBUTING.md](../CONTRIBUTING.md) for development setup and contribution guidelines.

## License

MIT License - see [LICENSE](../LICENSE) for details.
