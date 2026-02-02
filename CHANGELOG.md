# Changelog

All notable changes to Agentroot will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

#### Long-Term Memory System
- **Memory Storage** - Persistent memories with FTS5 search and content deduplication
  - Categories: `preference`, `entity`, `pattern`, `fact`
  - blake3 content hashing for automatic deduplication (ON CONFLICT upsert)
  - Access tracking (count, last_accessed_at) for memory reinforcement
  - 6 unit tests covering CRUD, dedup, FTS search, stats aggregation
- **LLM Memory Extraction** - Extract memories from search sessions via LLM analysis
  - Structured prompt for JSON array extraction with category/content/confidence
  - Graceful degradation on LLM failure (returns empty vec)
  - 4 unit tests for JSON parsing and validation
- **5 MCP Memory Tools** - `memory_store`, `memory_search`, `memory_list`, `memory_extract`, `memory_delete`

#### ANN Index (HNSW)
- **Approximate Nearest Neighbor** - HNSW index via `instant-distance` crate
  - Automatic build from database embeddings (threshold: 1000+ embeddings)
  - Cosine distance metric matching existing vector search
  - Falls back to brute-force for small collections (zero behavior change)
  - Thread-safe via RwLock for concurrent read access
  - 3 unit tests (below threshold, build+search, empty index)

#### Embedding Model Versioning
- **Model column on embeddings table** - Track which model generated each embedding
  - `get_embedding_stats()` returns per-model embedding counts
  - Filter embeddings by model name
  - Schema migration v11 with backward-compatible ALTER TABLE

#### Search Observability
- **Atomic search statistics** - Lock-free counters for query metrics
  - Track BM25, vector, hybrid query counts separately
  - Latency accumulation for average calculation
  - Cache hit/miss rates and ANN vs brute-force search counts
  - `SearchStatsSnapshot` for serializable reads
  - 2 unit tests (record + snapshot, concurrent access)

#### Agentic Extensions
- **Session Management** - Multi-turn search sessions with UUID-based tracking
  - Session context (key-value store), query history, seen-document demotion
  - Configurable TTL (default: 1 hour)
  - 4 MCP tools: `session_start`, `session_get`, `session_set`, `session_end`
- **Directory Browsing** - Navigate indexed collection structure
  - Directory index with document counts, concepts, and metadata
  - 2 MCP tools: `browse_directory`, `search_directories`
- **Batch & Explore Tools** - Multi-query and exploration support
  - `batch_search`: Execute multiple queries in a single call
  - `explore`: Search with related directories, concepts, and follow-up suggestions
- **Tiered Detail Levels** - L0 (minimal), L1 (standard), L2 (full content) response detail

#### Schema Migration v11
- `memories` table with FTS5 full-text index and triggers
- `model` column on `embeddings` table
- Guards for on-demand table creation (embeddings may not exist at migration time)

### Changed
- MCP server now exposes 29 tools (up from 16)
- `SearchContext` struct combines `AnnIndex` + `SearchStats` for shared search state
- `search_vec_with_ann()` accepts optional ANN index for accelerated vector search
- Updated integration guide and MCP server docs to document all 29 tools
- Moved development analysis notes from root to `docs/internal/`

#### Intelligent Glossary System
- **Semantic Concept Discovery** - Automatically extract and index key concepts from documents
  - LLM-powered concept extraction during metadata generation (5-10 concepts per document)
  - Concept normalization for deduplication (e.g., "Machine Learning" → "machine_learning")
  - Snippet-based linking showing concept usage in context (~100 chars)
  - FTS5-powered concept search for fast semantic queries
  - Global glossary spanning all collections
  - Database schema v7 with 3 new tables: `concepts`, `concept_chunks`, `concepts_fts`
  - Comprehensive test coverage: 6 unit tests + 5 integration tests

- **GlossarySearch Workflow Step** - New search mode for abstract/exploratory queries
  - Integrated into orchestrated workflows via `WorkflowStep::GlossarySearch`
  - Supplementary search aid (not primary mechanism)
  - Returns documents via semantic concept relationships
  - Example: Query "orchestrator" finds docs about "kubernetes", "container management"
  - LLM-guided usage (only triggers for appropriate queries)
  - Configurable confidence threshold (default 0.3)

- **Enhanced Metadata Generation** - Extended metadata with concept extraction
  - New `ExtractedConcept` struct with `term` and `snippet` fields
  - Few-shot prompt examples teaching concept extraction
  - Automatic normalization of concept variations
  - Concepts extracted per document (not per chunk)
  - Integration with indexing pipeline via `extract_and_link_concepts()`

#### New Providers
- **URLProvider** - Index content from web pages and HTTP(S) documents
  - Automatic title extraction from HTML `<title>` tags or markdown headers
  - Proper error handling for 404, 403, 401, 429, and server errors
  - Configurable timeout (30 seconds default)
  - Redirect following (up to 10 redirects)
  - User-agent header customization
  - 6 unit tests covering title extraction and error cases

- **PDFProvider** - Extract and index text from PDF files
  - Text extraction using pdf-extract library
  - Smart title extraction from content or filename
  - Directory scanning with glob pattern support
  - Handles image-based PDFs gracefully with error messages
  - Automatic exclusion of common directories (node_modules, .git, etc.)
  - 5 unit tests covering extraction and filename handling

- **SQLProvider** - Index content from SQLite databases
  - Flexible query configuration (table-based or custom SQL)
  - Configurable column mapping (id, title, content)
  - Proper type handling for INTEGER/TEXT/REAL id columns
  - Support for complex queries with JOINs and filters
  - Virtual URI format: `sql://path/to/db.sqlite/row_id`
  - 4 unit tests covering queries, configuration, and error handling

#### CLI Integration Tests
- 16 new integration tests across 4 test suites
- Collection tests (7 tests): add, list, remove, rename, duplicate detection
- Search tests (9 tests): BM25, output formats, filters, limits
- Document tests (4 tests): get, ls, multi-get with patterns
- Update/embed tests (6 tests): indexing, status, incremental updates
- Support for `AGENTROOT_DB` environment variable for isolated testing

#### TUI Enhancements
- **Collections View** mode for browsing and filtering by collection
- **Help Screen** with comprehensive keyboard shortcuts
- Collection filter toggle (activate/deactivate filter)
- Provider filter support in search
- Enhanced navigation with vim-style j/k keys
- Better status messages and mode indicators
- New keybindings: `c` (collections), `?` (help)

#### CI/CD and Performance
- **GitHub Actions benchmark workflow** (`.github/workflows/benchmark.yml`)
  - Runs on push, PR, weekly schedule, and manual trigger
  - Automatic performance regression detection (>5% slowdown fails CI)
  - PR comments on performance regressions
  - Artifact storage with 30-day retention
  - GitHub Pages deployment for HTML reports
  - Baseline comparison with criterion caching

- **Benchmark comparison script** (`scripts/bench-compare.sh`)
  - Interactive baseline comparison
  - Visual regression detection
  - Baseline save/restore functionality
  - Git commit-based baseline naming

- **Comprehensive performance documentation** (`docs/performance.md`)
  - 400+ lines covering indexing, search, and memory optimization
  - Large codebase best practices (10K-100K+ files)
  - Database tuning guide (SQLite pragma settings)
  - Troubleshooting common performance issues
  - Profiling and monitoring strategies
  - Expected throughput metrics and benchmarks

#### Examples
- `examples/pdf_provider.rs` - PDF indexing and search (176 lines)
- `examples/sql_provider.rs` - SQLite database indexing (236 lines)
- Enhanced `examples/url_provider.rs` with comprehensive error handling
- All examples include step-by-step demonstrations and cleanup instructions

#### LLM-Generated Metadata System
- **Automatic metadata generation** during document indexing using local LLMs
  - 8 metadata fields: summary, semantic title, keywords, category, intent, concepts, difficulty, suggested queries
  - Smart content truncation strategies (markdown, code, generic)
  - Fallback heuristics when LLM unavailable
  - Content-hash based caching for efficiency
  - Default model: `llama-3.1-8b-instruct.Q4_K_M.gguf` (8B parameters, ~4.5GB)

- **Database schema v4** with 10 new metadata columns
  - All metadata fields indexed in FTS5 for full-text search
  - Automatic trigger-based sync to search index
  - Migration from v3 handles schema upgrade transparently

- **CLI metadata commands**
  - `agentroot metadata refresh <collection>` - Regenerate metadata for collection
  - `agentroot metadata refresh --all` - Regenerate for all collections
  - `agentroot metadata refresh --doc <path>` - Regenerate single document
  - `agentroot metadata show <docid>` - Display document metadata
  - Support for custom model paths via `--model` flag

- **Enhanced search results** with metadata fields
  - BM25 search includes metadata in queries automatically
  - Vector search fetches metadata alongside results
  - Search results include summary, title, keywords, category, difficulty

- **Updated status command** with metadata statistics
  - Shows count of documents with generated metadata
  - Shows count of pending metadata generation


### Changed
- ProviderRegistry now includes URLProvider, PDFProvider, and SQLProvider by default
- Test coverage increased from 92 to 107 tests (+15 tests)
- CLI now supports `AGENTROOT_DB` environment variable for database path override
- TUI app state includes collection/provider filtering
- Cargo workspace includes pdf-extract dependency

### Fixed
- **Critical**: Database migration not running automatically on `initialize()` - schema v2 databases now properly upgrade to v3
- **Critical**: Foreign key constraint violation in `reindex_collection()` when updating documents - content now inserted before document reference update
- CLI exit handling causing tokio runtime panic (partial fix - simplified exit code handling)

### Dependencies
- Added `pdf-extract = "0.7"` for PDF text extraction

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

[Unreleased]: https://github.com/epappas/agentroot/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/epappas/agentroot/releases/tag/v0.1.0
