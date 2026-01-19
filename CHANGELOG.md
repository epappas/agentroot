# Changelog

All notable changes to Agentroot will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Comprehensive documentation suite
- Getting started tutorial
- MCP server integration guide
- Troubleshooting guide
- Performance documentation
- Contributing guidelines

## [0.1.0] - 2024-01-XX

### Added
- AST-aware semantic chunking for code files
- Support for Rust, Python, JavaScript, TypeScript, and Go
- Content-addressable embedding cache with blake3 hashing
- Hybrid search combining BM25 and vector similarity
- Reciprocal Rank Fusion for result ranking
- MCP (Model Context Protocol) server for AI assistant integration
- CLI with collection management, indexing, and search commands
- SQLite database with FTS5 full-text search
- Embedding generation using nomic-embed-text-v1.5
- Multiple output formats (CLI, JSON, CSV, Markdown, XML, files)
- Virtual path system (agentroot://)
- Document retrieval by path, docid, or glob pattern
- Collection-level file filtering with glob patterns
- Git integration for automatic pulling before index updates
- Smart cache with 80-90% hit rates on re-indexing

### Features

#### Core Library (agentroot-core)
- Database layer with SQLite and FTS5
- Index pipeline with file scanning, parsing, and chunking
- AST-aware semantic chunker using tree-sitter
- Oversized chunk handling with smart boundary detection
- Character-based fallback chunking for unsupported languages
- BM25 full-text search implementation
- Vector similarity search with cosine similarity
- Hybrid search with RRF fusion and query expansion support
- Embedding model integration via llama.cpp
- Content-addressable storage with SHA-256 and blake3 hashing

#### CLI (agentroot-cli)
- Collection management (add, list, remove, rename)
- Index operations (update, embed, cleanup)
- Search commands (search, vsearch, query)
- Document retrieval (get, multi-get, ls)
- Status and statistics reporting
- Progress indicators for long operations
- Multiple output format support

#### MCP Server (agentroot-mcp)
- JSON-RPC 2.0 protocol implementation
- Tools: search, vsearch, query, get, multi_get, status
- Resource support for document retrieval
- Integration with Claude Desktop and Continue.dev
- Prompt definitions for AI assistants

#### Terminal UI (agentroot-tui)
- Experimental interactive interface

[Unreleased]: https://github.com/spacejar/agentroot/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/spacejar/agentroot/releases/tag/v0.1.0
