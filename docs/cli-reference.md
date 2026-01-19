# CLI Reference

Complete reference for all Agentroot commands.

## Global Options

```
--format <FORMAT>  Output format [cli, json, csv, md, xml, files]
-v, --verbose      Enable verbose output
-h, --help         Print help information
-V, --version      Print version information
```

## Collection Management

### collection add

Add a new collection to the index from various sources (local files, GitHub, etc.).

```bash
agentroot collection add <PATH> [OPTIONS]
```

**Arguments:**
- `<PATH>` - Source path or URL (filesystem path, GitHub repository URL, etc.)

**Options:**
- `--name <NAME>` - Collection name (defaults to directory/repository name)
- `--mask <PATTERN>` - Glob pattern for files to include (default: `**/*.md`)
- `--provider <TYPE>` - Provider type: `file` (default), `github`, etc.
- `--config <JSON>` - Provider-specific configuration (JSON format)

**Provider Types:**

- **`file`** - Index local filesystem directories
- **`github`** - Index GitHub repositories (supports authentication)

**Examples:**

```bash
# Index local directory (file provider - default)
agentroot collection add ~/Documents/notes --name mynotes

# Index Rust files only
agentroot collection add ./src --name rust-code --mask '**/*.rs'

# Index markdown and text files
agentroot collection add ./docs --name docs --mask '**/*.md'

# Index with custom file provider options
agentroot collection add ./project --name myproject --mask '**/*.{rs,toml}' \
  --config '{"exclude_hidden":"false","follow_symlinks":"true"}'

# Index GitHub repository
agentroot collection add https://github.com/rust-lang/rust --name rust-lang \
  --provider github --mask '**/*.md'

# Index GitHub repository with authentication
agentroot collection add https://github.com/myorg/private-repo --name myrepo \
  --provider github --config '{"github_token":"ghp_your_token_here"}'
```

**Provider Configuration:**

File provider options (`--config` JSON keys):
- `exclude_hidden` - Skip hidden files/directories (default: `true`)
- `follow_symlinks` - Follow symbolic links (default: `true`)

GitHub provider options (`--config` JSON keys):
- `github_token` - GitHub personal access token for authentication

See [Providers Documentation](providers.md) for detailed provider information.

### collection list

List all collections.

```bash
agentroot collection list
```

**Output:**
```
myproject: /home/user/projects/myproject (**/*.rs)
notes: /home/user/Documents/notes (*.md)
```

### collection remove

Remove a collection and all its indexed documents.

```bash
agentroot collection remove <NAME>
```

**Arguments:**
- `<NAME>` - Name of collection to remove

### collection rename

Rename an existing collection.

```bash
agentroot collection rename <OLD_NAME> <NEW_NAME>
```

## Indexing

### update

Re-index all collections, scanning for new, modified, and deleted files.

```bash
agentroot update [OPTIONS]
```

**Options:**
- `--pull` - Run `git pull` in git repositories before indexing

**Output:**
```
Updating myproject                            myproject: 42 files updated
Updating notes                                notes: 15 files updated
Done (2/2)
```

### embed

Generate vector embeddings for all indexed documents.

```bash
agentroot embed [OPTIONS]
```

**Options:**
- `-f, --force` - Force re-embedding of all documents (ignore cache)
- `-m, --model <PATH>` - Path to embedding model (GGUF file)

**Output:**
```
Loading embedding model: nomic-embed-text-v1.5.Q4_K_M (768 dimensions)
Embedding: 42/42 docs, 380 chunks
  Cached: 320 (84.2%)
  Computed: 60 (15.8%)
Done
```

## Search

### search

BM25 full-text search using SQLite FTS5.

```bash
agentroot search <QUERY> [OPTIONS]
```

**Arguments:**
- `<QUERY>` - Search query (supports FTS5 syntax)

**Options:**
- `-c, --collection <NAME>` - Restrict search to a collection
- `-n <NUM>` - Number of results (default: 10)
- `--all` - Return all matches
- `--min-score <NUM>` - Minimum score threshold
- `--full` - Show full document content
- `--line-numbers` - Add line numbers to output

**Examples:**

```bash
# Basic search
agentroot search "error handling"

# Search in specific collection
agentroot search "async function" -c myproject

# Get more results
agentroot search "database" -n 20

# Show full content
agentroot search "config" --full
```

**Output:**
```
 85% myproject/src/error.rs #a1b2c3
 72% myproject/src/handler.rs #d4e5f6
 68% myproject/src/main.rs #789abc
```

### vsearch

Vector similarity search using embeddings.

```bash
agentroot vsearch <QUERY> [OPTIONS]
```

**Arguments:**
- `<QUERY>` - Natural language query

**Options:**
- Same as `search`

**Note:** Requires embeddings to be generated first with `agentroot embed`.

### query

Hybrid search combining BM25 and vector search with Reciprocal Rank Fusion.

```bash
agentroot query <QUERY> [OPTIONS]
```

**Arguments:**
- `<QUERY>` - Search query

**Options:**
- Same as `search`

**Note:** This provides the best search quality by combining lexical and semantic matching.

## Document Retrieval

### get

Retrieve a single document by path or document ID.

```bash
agentroot get <IDENTIFIER> [OPTIONS]
```

**Arguments:**
- `<IDENTIFIER>` - File path or docid (e.g., `#a1b2c3` or `a1b2c3`)

**Options:**
- `--line-numbers` - Add line numbers to output

**Examples:**

```bash
# Get by docid (from search results)
agentroot get "#a1b2c3"
agentroot get a1b2c3

# Get by path
agentroot get myproject/src/main.rs
```

### multi-get

Retrieve multiple documents by glob pattern or comma-separated list.

```bash
agentroot multi-get <PATTERN> [OPTIONS]
```

**Arguments:**
- `<PATTERN>` - Glob pattern or comma-separated docids

**Options:**
- `-l <NUM>` - Maximum lines per file
- `--max-bytes <NUM>` - Skip files larger than this (default: 10KB)
- `--line-numbers` - Add line numbers to output

**Examples:**

```bash
# Get by glob pattern
agentroot multi-get "myproject/src/*.rs"

# Get multiple docids
agentroot multi-get "#a1b2c3, #d4e5f6, #789abc"

# Limit output size
agentroot multi-get "**/*.md" -l 100 --max-bytes 5000
```

### ls

List collections or files within a collection.

```bash
agentroot ls [COLLECTION[/PATH]]
```

**Arguments:**
- `[COLLECTION]` - Collection name (optional)
- `[PATH]` - Path prefix within collection (optional)

**Examples:**

```bash
# List all collections
agentroot ls

# List files in a collection
agentroot ls myproject

# List files with path prefix
agentroot ls myproject/src/handlers
```

**Output:**
```
myproject/src/main.rs #a1b2c3
myproject/src/error.rs #d4e5f6
myproject/src/config.rs #789abc
```

## Status and Maintenance

### status

Show index status and statistics.

```bash
agentroot status
```

**Output:**
```
Collections: 3
Documents:   156
Embedded:    156
Pending:     0
```

### cleanup

Clean up the database (remove orphaned data, optimize).

```bash
agentroot cleanup [OPTIONS]
```

**Options:**
- `--vacuum` - Run SQLite VACUUM to reclaim space

## Context Management

### context add

Add context metadata for a path.

```bash
agentroot context add [PATH] <TEXT>
```

**Arguments:**
- `[PATH]` - Path to add context for (defaults to current directory)
- `<TEXT>` - Context description

**Examples:**

```bash
# Add context to current directory
agentroot context add "Main application source code"

# Add context to specific path
agentroot context add /src/handlers "HTTP request handlers"

# Add global context
agentroot context add / "Always include this in search context"
```

### context list

List all contexts.

```bash
agentroot context list
```

### context check

Check for collections or paths missing context.

```bash
agentroot context check
```

### context rm

Remove context for a path.

```bash
agentroot context rm <PATH>
```

## MCP Server

### mcp

Start the Model Context Protocol server.

```bash
agentroot mcp [OPTIONS]
```

**Options:**
- `--socket <PATH>` - Unix socket path (default: stdio)

See [MCP Server](mcp-server.md) for integration details.

## Output Formats

All commands support multiple output formats via `--format`:

### cli (default)

Human-readable terminal output with colors and formatting.

### json

JSON output for programmatic consumption:

```bash
agentroot search "query" --format json
```

```json
[
  {
    "docid": "#a1b2c3",
    "score": 0.85,
    "path": "src/main.rs",
    "collection": "myproject",
    "title": "Main Application"
  }
]
```

### csv

CSV output for spreadsheet import:

```bash
agentroot search "query" --format csv
```

```csv
docid,score,path,collection,title
#a1b2c3,0.85,src/main.rs,myproject,Main Application
```

### md

Markdown output:

```bash
agentroot search "query" --format md
```

```markdown
| Score | Path | Collection |
|-------|------|------------|
| 85% | src/main.rs | myproject |
```

### xml

XML output:

```bash
agentroot search "query" --format xml
```

### files

File paths only (useful for piping):

```bash
agentroot search "query" --format files | xargs cat
```

## FTS5 Query Syntax

The `search` command uses SQLite FTS5 syntax:

```bash
# Simple term search
agentroot search "error"

# Phrase search (exact match)
agentroot search '"error handling"'

# AND (implicit)
agentroot search "error handling"

# OR
agentroot search "error OR exception"

# NOT
agentroot search "error NOT warning"

# Prefix search
agentroot search "hand*"

# Column filter (searches title or content)
agentroot search "title:README"

# NEAR (terms within N words)
agentroot search "NEAR(error handling, 5)"
```

**Note:** Hyphens in terms can cause issues (interpreted as NOT). Use quotes for hyphenated terms: `"tree-sitter"`.

## Environment Variables

- `RUST_LOG` - Set log level (e.g., `RUST_LOG=debug`)
- `AGENTROOT_DB` - Override database path
- `AGENTROOT_MODELS` - Override models directory
