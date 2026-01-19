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
| [Provider System](providers.md) | Multi-source indexing (files, GitHub, URLs, etc.) |
| [Semantic Chunking](semantic-chunking.md) | AST-aware code parsing and chunking |
| [Embedding Cache](embedding-cache.md) | Content-addressable caching system |
| [MCP Server](mcp-server.md) | Model Context Protocol integration for AI assistants |

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
| Update index | `agentroot update` |
| Generate embeddings | `agentroot embed` |
| Search (BM25) | `agentroot search "keyword"` |
| Search (vector) | `agentroot vsearch "natural language query"` |
| Search (hybrid) | `agentroot query "best quality search"` |
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
# Basic search (database setup, indexing, BM25)
cargo run --example basic_search

# Semantic chunking (AST-aware parsing)
cargo run --example semantic_chunking

# Custom indexing pipeline
cargo run --example custom_index

# GitHub provider
cargo run --example github_provider
```

## Contributing

See [CONTRIBUTING.md](../CONTRIBUTING.md) for development setup and contribution guidelines.

## License

MIT License - see [LICENSE](../LICENSE) for details.
