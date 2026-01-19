# How-To Guide

Practical recipes for common Agentroot tasks.

## Quick Reference

```bash
# Basic workflow
agentroot collection add /path/to/code --name myproject
agentroot update
agentroot embed
agentroot query "what I'm looking for"

# Provider workflow
agentroot collection add https://github.com/owner/repo --provider github --name repo
agentroot update
agentroot search "keyword"
```

## Collections

### Add a Local Directory

```bash
# Basic - indexes all files
agentroot collection add ~/Documents/notes --name notes

# With pattern - only specific files
agentroot collection add ~/code/myapp --name myapp --mask '**/*.{rs,toml}'

# Multiple patterns
agentroot collection add ~/docs --name docs \
  --mask '**/*.md' \
  --mask '**/*.txt'
```

### Add a GitHub Repository

```bash
# Public repository
agentroot collection add https://github.com/rust-lang/rust \
  --name rust-docs \
  --mask '**/*.md' \
  --provider github

# Private repository with token
agentroot collection add https://github.com/myorg/private-repo \
  --name myrepo \
  --provider github \
  --config '{"github_token":"ghp_YOUR_TOKEN_HERE"}'

# Or use environment variable
export GITHUB_TOKEN=ghp_YOUR_TOKEN_HERE
agentroot collection add https://github.com/myorg/private-repo \
  --name myrepo \
  --provider github
```

### Add Web Pages (URL Provider)

```bash
# Index a single web page
agentroot collection add https://doc.rust-lang.org/book/ \
  --name rust-book \
  --provider url

# Index with custom timeout and user agent
agentroot collection add https://example.com/docs \
  --name docs \
  --provider url \
  --config '{"timeout":"60","user_agent":"agentroot/1.0"}'

# Index with redirect limit
agentroot collection add https://blog.example.com \
  --name blog \
  --provider url \
  --config '{"redirect_limit":"5"}'
```

### Add PDF Documents

```bash
# Index a single PDF
agentroot collection add /path/to/document.pdf \
  --name research \
  --provider pdf

# Index all PDFs in a directory
agentroot collection add /path/to/papers \
  --name papers \
  --mask '**/*.pdf' \
  --provider pdf

# Include hidden PDF files
agentroot collection add /path/to/pdfs \
  --name all-pdfs \
  --mask '**/*.pdf' \
  --provider pdf \
  --config '{"exclude_hidden":"false"}'
```

### Add Database Content (SQL Provider)

```bash
# Index a table (uses first 3 columns as id, title, content)
agentroot collection add /path/to/database.sqlite \
  --name blog-posts \
  --provider sql \
  --config '{"table":"posts"}'

# Index with custom query
agentroot collection add /path/to/database.sqlite \
  --name published-posts \
  --provider sql \
  --config '{"query":"SELECT id, title, content FROM posts WHERE published = 1"}'

# Index with custom column mapping
agentroot collection add /path/to/cms.sqlite \
  --name articles \
  --provider sql \
  --config '{"table":"articles","id_column":"article_id","title_column":"headline","content_column":"body"}'

# Index with JOIN query
agentroot collection add /path/to/app.sqlite \
  --name user-posts \
  --provider sql \
  --config '{"query":"SELECT p.id, u.username || ': ' || p.title, p.content FROM posts p JOIN users u ON p.user_id = u.id"}'
```

### Configure File Provider Options

```bash
# Include hidden files
agentroot collection add /path/to/code --name mycode \
  --mask '**/*.rs' \
  --config '{"exclude_hidden":"false"}'

# Don't follow symlinks
agentroot collection add /path/to/code --name mycode \
  --mask '**/*.py' \
  --config '{"follow_symlinks":"false"}'

# Multiple options
agentroot collection add /path/to/code --name mycode \
  --mask '**/*.js' \
  --config '{"exclude_hidden":"false","follow_symlinks":"true"}'
```

### Manage Collections

```bash
# List all collections
agentroot collection list

# Remove a collection
agentroot collection remove myproject

# Rename a collection
agentroot collection rename old-name new-name
```

## Indexing

### Index All Collections

```bash
# Basic update
agentroot update

# With git pull first (for git repos)
agentroot update --pull

# Verbose mode
agentroot update -v
```

### Generate Embeddings

```bash
# Generate embeddings for all documents
agentroot embed

# Force re-embedding (ignore cache)
agentroot embed --force

# With custom model
agentroot embed --model /path/to/model.gguf
```

### Check Index Status

```bash
agentroot status
```

Output shows:
- Number of collections
- Total documents indexed
- Embeddings generated
- Documents pending embedding

## Searching

### BM25 Full-Text Search

```bash
# Basic keyword search
agentroot search "error handling"

# Search in specific collection
agentroot search "database query" -c myproject

# More results
agentroot search "async" -n 20

# Show full content
agentroot search "config" --full

# Minimum score filter
agentroot search "query" --min-score 0.7

# Output formats
agentroot search "query" --format json
agentroot search "query" --format csv
agentroot search "query" --format md
```

### Vector Similarity Search

```bash
# Natural language query
agentroot vsearch "how to handle database errors"

# Search specific collection
agentroot vsearch "authentication patterns" -c backend

# More results
agentroot vsearch "caching strategies" -n 15
```

**Note**: Requires running `agentroot embed` first.

### Hybrid Search (Recommended)

```bash
# Combines BM25 and vector search for best results
agentroot query "error handling patterns"

# With collection filter
agentroot query "async patterns" -c myproject

# More results
agentroot query "database design" -n 20
```

### Advanced FTS5 Queries

```bash
# Exact phrase
agentroot search '"error handling"'

# OR operator
agentroot search "error OR exception"

# NOT operator
agentroot search "error NOT warning"

# Prefix search
agentroot search "handle*"

# Column filter
agentroot search "title:README"

# NEAR operator (within N words)
agentroot search "NEAR(error handling, 5)"

# Hyphenated terms (use quotes)
agentroot search '"tree-sitter"'
```

## Document Retrieval

### Get Single Document

```bash
# By docid (from search results)
agentroot get "#a1b2c3"
agentroot get a1b2c3

# By path
agentroot get myproject/src/main.rs

# With line numbers
agentroot get myproject/src/main.rs --line-numbers
```

### Get Multiple Documents

```bash
# By glob pattern
agentroot multi-get "myproject/src/**/*.rs"

# Multiple docids
agentroot multi-get "#a1b2c3, #d4e5f6, #789abc"

# Limit output size
agentroot multi-get "**/*.md" -l 100 --max-bytes 5000

# With line numbers
agentroot multi-get "src/**/*.rs" --line-numbers
```

### List Files

```bash
# List all collections
agentroot ls

# List files in collection
agentroot ls myproject

# List with path prefix
agentroot ls myproject/src/handlers
```

## Maintenance

### Clean Up Database

```bash
# Remove orphaned data
agentroot cleanup

# Compact database (reclaim space)
agentroot cleanup --vacuum
```

### Reset Everything

```bash
# Backup first (optional)
cp -r ~/.cache/agentroot ~/.cache/agentroot.backup

# Remove database
rm -rf ~/.cache/agentroot/index.sqlite*

# Start fresh
agentroot collection add /path --name test
agentroot update
agentroot embed
```

## Context Management

Context helps provide additional information about paths for search.

### Add Context

```bash
# Add context to current directory
agentroot context add "Main application source code"

# Add context to specific path
agentroot context add /src/handlers "HTTP request handlers"

# Add global context
agentroot context add / "This is a web application backend"
```

### Manage Context

```bash
# List all contexts
agentroot context list

# Check for missing contexts
agentroot context check

# Remove context
agentroot context rm /src/handlers
```

## Output Formats

All commands support `--format` option:

### JSON

```bash
agentroot search "query" --format json > results.json
```

Output structure:
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

### CSV

```bash
agentroot search "query" --format csv > results.csv
```

### Markdown

```bash
agentroot search "query" --format md > results.md
```

### Files Only

```bash
# Pipe to other commands
agentroot search "TODO" --format files | xargs grep -n "TODO"

# Edit all matching files
agentroot search "deprecated" --format files | xargs $EDITOR
```

## Environment Variables

### Database Location

```bash
# Use custom database location
export AGENTROOT_DB=/custom/path/index.sqlite
agentroot update
```

### Models Directory

```bash
# Use custom models directory
export AGENTROOT_MODELS=/custom/path/models
agentroot embed
```

### Logging

```bash
# Enable debug logging
RUST_LOG=debug agentroot update

# Module-specific logging
RUST_LOG=agentroot_core::db=trace agentroot search "query"
```

### GitHub Token

```bash
# Set token for GitHub API
export GITHUB_TOKEN=ghp_YOUR_TOKEN_HERE
agentroot collection add https://github.com/owner/repo --provider github
```

## Workflows

### Daily Development Workflow

```bash
# Morning: update index
agentroot update

# Work on code...

# Search as needed
agentroot query "how did we implement X"

# End of day: update embeddings if needed
agentroot embed  # Fast with cache
```

### Adding New Project

```bash
# 1. Add collection
agentroot collection add ~/code/newproject --name newproject --mask '**/*.{rs,toml}'

# 2. Index files
agentroot update

# 3. Generate embeddings
agentroot embed

# 4. Verify
agentroot status
agentroot ls newproject
```

### Research GitHub Repository

```bash
# 1. Add GitHub repo
agentroot collection add https://github.com/rust-lang/rust \
  --name rust-docs \
  --mask '**/*.md' \
  --provider github

# 2. Index (may take a few minutes for large repos)
agentroot update

# 3. Search documentation
agentroot search "async traits" -c rust-docs
agentroot query "how to implement custom allocator" -c rust-docs
```

### Multi-Project Search

```bash
# Add multiple projects
agentroot collection add ~/code/frontend --name frontend --mask '**/*.{js,jsx,ts,tsx}'
agentroot collection add ~/code/backend --name backend --mask '**/*.rs'
agentroot collection add ~/code/docs --name docs --mask '**/*.md'

# Index all
agentroot update
agentroot embed

# Search across all
agentroot query "authentication"

# Search specific project
agentroot query "authentication" -c backend
```

### Code Review Assistance

```bash
# Find similar implementations
agentroot vsearch "functions that validate user input" -c myproject

# Find all error handling
agentroot search "Result<" -c myproject --format files

# Find TODOs
agentroot search "TODO OR FIXME" --format files
```

### Documentation Search

```bash
# Add documentation
agentroot collection add ~/Documents/notes --name notes --mask '**/*.md'
agentroot collection add ~/Documents/wiki --name wiki --mask '**/*.md'

# Natural language search
agentroot query "how to set up postgres" -c notes
agentroot query "deployment checklist" -c wiki
```

### Index Research Papers

```bash
# 1. Add PDF collection
agentroot collection add ~/Documents/research/papers \
  --name research-papers \
  --mask '**/*.pdf' \
  --provider pdf

# 2. Index PDFs
agentroot update

# 3. Generate embeddings
agentroot embed

# 4. Search by topic
agentroot query "machine learning optimization techniques" -c research-papers
agentroot vsearch "neural network architectures" -c research-papers
```

### Index Documentation Website

```bash
# 1. Add URL collection
agentroot collection add https://doc.rust-lang.org/book/ \
  --name rust-book \
  --provider url

# 2. Index content
agentroot update

# 3. Search documentation
agentroot query "ownership and borrowing" -c rust-book
agentroot search "lifetime" -c rust-book
```

### Index CMS or Blog Database

```bash
# 1. Add SQL collection (published posts only)
agentroot collection add ~/blog/database.sqlite \
  --name blog-posts \
  --provider sql \
  --config '{"query":"SELECT id, title, content FROM posts WHERE status = 'published' ORDER BY created_at DESC"}'

# 2. Index database
agentroot update

# 3. Generate embeddings
agentroot embed

# 4. Search posts
agentroot query "deployment strategies" -c blog-posts
agentroot search "docker OR kubernetes" -c blog-posts
```

## Integration with Other Tools

### Pipe to Editor

```bash
# Edit all files containing pattern
agentroot search "deprecated" --format files | xargs $EDITOR
```

### Pipe to Grep

```bash
# Get files then search within them
agentroot search "error" --format files | xargs grep -n "Error::"
```

### Pipe to ripgrep

```bash
# Fast follow-up search in results
agentroot search "handler" --format files | xargs rg "async fn"
```

### Pipe to jq (JSON processing)

```bash
# Filter JSON results
agentroot search "query" --format json | jq '.[] | select(.score > 0.8)'

# Extract just paths
agentroot search "query" --format json | jq -r '.[].path'
```

### Shell Scripts

```bash
#!/bin/bash
# search_and_edit.sh - Search and open results in editor

QUERY="$1"
FILES=$(agentroot search "$QUERY" --format files | head -5)

if [ -z "$FILES" ]; then
    echo "No results found"
    exit 1
fi

$EDITOR $FILES
```

### Git Hooks

```bash
# .git/hooks/post-commit
#!/bin/bash
# Update index after commit

agentroot update
echo "Index updated"
```

## Tips and Best Practices

### Pattern Design

```bash
# ✅ Good: Specific patterns
agentroot collection add ~/code --mask '**/*.{rs,toml,md}'

# ❌ Bad: Too broad
agentroot collection add ~/code --mask '**/*'  # Includes binaries, etc.

# ✅ Good: Exclude generated files
agentroot collection add ~/code --mask '**/*.js' \
  --config '{"exclude_dirs":"node_modules,dist,build"}'
```

### Collection Organization

```bash
# ✅ Good: Separate collections by project/type
agentroot collection add ~/code/projectA --name projectA
agentroot collection add ~/code/projectB --name projectB
agentroot collection add ~/docs --name docs

# ❌ Bad: One giant collection
agentroot collection add ~ --name everything  # Too broad
```

### Search Strategy

```bash
# For exact matches (function names, error codes):
agentroot search "exact_function_name"

# For concepts (natural language):
agentroot vsearch "how to handle database errors"

# For best overall results:
agentroot query "database error handling"
```

### Performance

```bash
# Filter by collection for faster vector search
agentroot vsearch "query" -c specific-collection

# Use BM25 when semantic search not needed
agentroot search "keyword" -c collection

# Limit results to avoid overhead
agentroot query "query" -n 10
```

### Caching

```bash
# Cache works automatically - just re-run embed after changes
agentroot update  # Updates document content
agentroot embed   # Only re-embeds changed chunks (5-10x faster)

# Force re-embed only when needed (model change, etc.)
agentroot embed --force
```

## Common Patterns

### Find Similar Code

```bash
# Vector search excels at this
agentroot vsearch "functions that parse JSON into structs" -c backend
```

### Find All Implementations

```bash
# BM25 search for specific trait/interface
agentroot search "impl Display for" -c myproject
```

### Find Related Documentation

```bash
# Hybrid search for documentation
agentroot query "how to use the authentication module" -c docs
```

### Find TODOs and FIXMEs

```bash
# BM25 search for keywords
agentroot search "TODO OR FIXME" --format files
```

### Code Archaeology

```bash
# Find where a pattern is used
agentroot search "DatabasePool" --full

# Find similar error handling
agentroot vsearch "error handling with context" -c backend
```

## Troubleshooting

See [Troubleshooting Guide](troubleshooting.md) for detailed solutions to common issues.

### Quick Checks

```bash
# Verify installation
agentroot --version

# Check index status
agentroot status

# List collections
agentroot collection list

# Test search
agentroot search "test"

# Check database
sqlite3 ~/.cache/agentroot/index.sqlite "SELECT COUNT(*) FROM documents;"
```

### Debug Mode

```bash
# Enable verbose logging
RUST_LOG=debug agentroot update

# Check specific module
RUST_LOG=agentroot_core::providers=trace agentroot update
```

## See Also

- [Getting Started Guide](getting-started.md) - Initial setup and basic usage
- [CLI Reference](cli-reference.md) - Complete command reference
- [Provider Documentation](providers.md) - Multi-source indexing details
- [Troubleshooting](troubleshooting.md) - Solutions to common problems
- [MCP Server Guide](mcp-server.md) - AI assistant integration
