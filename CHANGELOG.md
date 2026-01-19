# Changelog

All notable changes to Agentroot will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0] - 2024-01-XX

### Added

#### Provider System (Multi-Source Indexing)
- Pluggable provider architecture for indexing from multiple sources
- `SourceProvider` trait for implementing content sources
- `ProviderRegistry` for managing available providers
- `ProviderConfig` for provider-specific configuration
- `SourceItem` data structure for unified content representation
- `FileProvider` - Local file system with glob patterns (default)
- `GitHubProvider` - GitHub repositories and files with API integration
- Support for provider-specific metadata and authentication
- Database schema v3 with source tracking columns
- `DocumentInsert` struct with builder pattern for cleaner API

#### CLI Enhancements
- `--provider` flag for `collection add` command (file, github, url, etc.)
- `--config` flag for provider-specific JSON configuration
- Enhanced `collection list` output showing provider type and document count
- Support for GitHub URLs in collection paths
- Automatic provider detection based on path format

#### Library API
- New public exports: `FileProvider`, `GitHubProvider`, `ProviderConfig`, `ProviderRegistry`, `SourceProvider`, `SourceItem`
- `add_collection()` now accepts 5 parameters (name, path, pattern, provider_type, provider_config)
- `insert_doc()` method using `DocumentInsert` struct (preferred over insert_document)
- `reindex_collection()` now uses provider system automatically
- Backward compatible: existing file-based collections work unchanged

#### Documentation
- Comprehensive [Provider System Guide](docs/providers.md)
- GitHub provider example (`examples/github_provider.rs`)
- Updated README with multi-source indexing section
- Updated getting started guide with provider usage
- Updated AGENTS.md with provider architecture details
- API reference updates for new signatures

### Changed
- Database schema upgraded from v2 to v3
- `documents` table: added `source_type` and `source_uri` columns
- `collections` table: added `provider_type` and `provider_config` columns
- `Document` struct: added `source_type` and `source_uri` fields
- `CollectionInfo` struct: added `provider_type` and `provider_config` fields
- `insert_document()` signature: now requires 8 parameters (legacy method)
- `add_collection()` signature: now requires 5 parameters (added provider support)
- Automatic migration from schema v2 to v3 on database open

### Dependencies
- Added `base64` for GitHub API response decoding
- Added `reqwest` blocking feature for synchronous HTTP
- Added `tempfile` for provider tests

### Fixed
- Clippy warning: Too many arguments in `insert_document()` (now uses struct pattern)

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
