# Troubleshooting

Common issues and solutions for Agentroot.

## Installation Issues

### Rust Toolchain Missing

**Problem**: `cargo: command not found`

**Solution**: Install Rust toolchain:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

Verify installation:

```bash
cargo --version
```

### Build Fails with Linking Errors

**Problem**: Compilation succeeds but linking fails

**Solution**: Install required system libraries:

**Ubuntu/Debian**:
```bash
sudo apt-get install build-essential pkg-config libssl-dev
```

**macOS**:
```bash
xcode-select --install
```

**Fedora/RHEL**:
```bash
sudo dnf install gcc openssl-devel
```

### Binary Not in PATH

**Problem**: `agentroot: command not found` after installation

**Solution**: Ensure Cargo bin directory is in PATH:

```bash
# Add to ~/.bashrc or ~/.zshrc
export PATH="$HOME/.cargo/bin:$PATH"

# Reload shell
source ~/.bashrc
```

Verify:

```bash
# Expand ~ manually if needed
agentroot collection add $HOME/projects/myapp --name myapp
```

## Provider-Specific Issues

### GitHub API Rate Limit

**Problem**: `GitHub API error: 403` or `API rate limit exceeded`

**Solution**: You've hit GitHub's API rate limits:

**Without authentication**:
- Limit: 60 requests/hour
- Solution: Set GITHUB_TOKEN

```bash
export GITHUB_TOKEN=ghp_your_token_here
agentroot update
```

**With authentication**:
- Limit: 5,000 requests/hour
- Solution: Wait for rate limit reset or use local caching

Check your rate limit:
```bash
curl -H "Authorization: token $GITHUB_TOKEN" \
  https://api.github.com/rate_limit
```

### GitHub Repository Not Found

**Problem**: `HTTP error: 404` when adding GitHub collection

**Solution**: Verify repository URL and access:

```bash
# Check URL format
# Correct: https://github.com/owner/repo
# Wrong: git@github.com:owner/repo.git

# Verify repository exists
curl -I https://github.com/owner/repo

# For private repos, ensure GITHUB_TOKEN has access
export GITHUB_TOKEN=ghp_your_token_with_repo_access
```

### GitHub Connection Timeout

**Problem**: `Network error: connection timeout`

**Solution**: Check internet connection and proxy settings:

```bash
# Test GitHub connectivity
curl -I https://github.com

# Set proxy if behind firewall
export HTTP_PROXY=http://proxy.example.com:8080
export HTTPS_PROXY=http://proxy.example.com:8080

# Try again
agentroot update
```

### GitHub Files Not Indexed

**Problem**: GitHub collection created but no files indexed

**Solution**: Check pattern and repository structure:

```bash
# List collections to verify provider
agentroot collection list
# Should show: [provider: github, ...]

# Try broader pattern first
agentroot collection add https://github.com/owner/repo \
  --name test \
  --mask '**/*' \
  --provider github

agentroot update
agentroot status  # Check document count
```

### Invalid Provider Type

**Problem**: `Unknown provider type: xyz`

**Solution**: Use supported provider types:

```bash
# Supported providers:
# - file (default, local filesystem)
# - github (GitHub repositories)
# - url (web pages via HTTP/HTTPS)
# - pdf (PDF documents)
# - sql (SQLite databases)

# Check available providers in code:
# See ProviderRegistry::with_defaults() in lib.rs

# Examples with explicit provider:
agentroot collection add /path --name local --provider file
agentroot collection add https://github.com/owner/repo --provider github
agentroot collection add https://example.com --provider url
agentroot collection add /path/to/docs --mask '**/*.pdf' --provider pdf
agentroot collection add /path/to/db.sqlite --provider sql
```

### Provider Config Parse Error

**Problem**: `Invalid provider config` or JSON parse error

**Solution**: Provider config must be valid JSON:

```bash
# Wrong: single quotes
agentroot collection add https://github.com/owner/repo \
  --config '{'github_token': 'ghp_...'}'

# Right: double quotes, escaped
agentroot collection add https://github.com/owner/repo \
  --config '{"github_token": "ghp_..."}'

# Or use environment variable instead
export GITHUB_TOKEN=ghp_...
agentroot collection add https://github.com/owner/repo \
  --provider github  # Uses GITHUB_TOKEN automatically
```

### File Provider with URL

**Problem**: `Path does not exist` when using URL with file provider

**Solution**: File provider only works with local paths. Use appropriate provider:

```bash
# Wrong: URL with file provider
agentroot collection add https://github.com/owner/repo \
  --provider file  # Error!

# Right: Use github provider
agentroot collection add https://github.com/owner/repo \
  --provider github

# Right: Local path with file provider
agentroot collection add /path/to/local \
  --provider file
```

### GitHub Large Repository Timeout

**Problem**: Indexing GitHub repo times out or takes hours

**Solution**: Large repositories may hit API limits or take long time:

1. **Use specific file patterns**:
```bash
# Instead of **/*
agentroot collection add https://github.com/owner/repo \
  --mask '**/*.md' \  # Only markdown
  --provider github
```

2. **Index incrementally**:
```bash
# First run: indexes everything (may be slow)
agentroot update

# Subsequent runs: only changed files (fast)
agentroot update
```

3. **Monitor progress**:
```bash
# Use verbose mode
agentroot update -v

# Check status in another terminal
watch -n 5 agentroot status
```

4. **Consider local clone**:
```bash
# For very large repos, clone locally first
git clone https://github.com/owner/repo /tmp/repo
agentroot collection add /tmp/repo --name local-copy
```

### CLI Update Command Panics with GitHub Collections

**Problem**: `agentroot update` crashes with "Cannot drop a runtime in a context where blocking is not allowed"

**Root Cause**: This is a known issue with the GitHubProvider using `reqwest::blocking::Client` in an async tokio context. The core provider system works correctly (verified by unit tests), but CLI runtime handling has limitations.

**Workaround**: Use the library API directly for GitHub collections:

```rust
use agentroot_core::Database;

let db = Database::open(Database::default_path())?;
db.initialize()?;

// Add GitHub collection
db.add_collection("rust-docs", 
    "https://github.com/rust-lang/rust", 
    "**/*.md", 
    "github", 
    None)?;

// Reindex (works correctly)
let updated = db.reindex_collection("rust-docs")?;
println!("Updated {} files", updated);
```

See `examples/github_provider.rs` for a complete working example.

**Status**: This will be fixed in a future release by converting GitHubProvider to use async reqwest client.

**For now**: File-based collections work perfectly via CLI. GitHub collections can be managed via the library API.

### URL Provider Connection Timeout

**Problem**: `HTTP error: connection timeout` or request hangs when indexing URLs

**Solution**: Adjust timeout settings or check network:

```bash
# Increase timeout (default is 30 seconds)
agentroot collection add https://example.com/docs \
  --name docs \
  --provider url \
  --config '{"timeout":"120"}'

# Check URL accessibility
curl -I https://example.com/docs

# Test with different user agent (some sites block default agents)
agentroot collection add https://example.com \
  --provider url \
  --config '{"user_agent":"Mozilla/5.0"}'
```

### URL Provider HTTP Errors

**Problem**: `HTTP error: 404`, `403`, or `401` when fetching URLs

**Solution**: Verify URL and access permissions:

```bash
# 404 - Page not found
curl -I https://example.com/missing-page  # Should return 404

# 403 - Forbidden (may require authentication or user-agent)
agentroot collection add https://example.com \
  --provider url \
  --config '{"user_agent":"Mozilla/5.0 (compatible; agentroot/1.0)"}'

# 401 - Authentication required
# URLProvider does not currently support authenticated requests
# Workaround: Download content locally first
wget -r -np -k https://example.com/docs
agentroot collection add ./docs --name docs --provider file
```

### URL Provider SSL/TLS Errors

**Problem**: `SSL error` or certificate validation fails

**Solution**: Check certificate validity:

```bash
# Test SSL certificate
curl -vI https://example.com

# If certificate is self-signed or expired, download content locally
wget --no-check-certificate -r https://example.com
agentroot collection add ./downloaded --provider file
```

### URL Provider Too Many Redirects

**Problem**: `Redirect limit exceeded` when fetching URL

**Solution**: Increase redirect limit or check for redirect loops:

```bash
# Increase redirect limit (default is 10)
agentroot collection add https://example.com \
  --provider url \
  --config '{"redirect_limit":"20"}'

# Check redirect chain
curl -L -I https://example.com  # Shows all redirects
```

### PDF Provider No Text Extracted

**Problem**: PDF indexed but contains no content or shows empty results

**Solution**: PDF may be image-based (scanned document):

```bash
# Check if PDF has extractable text
pdftotext document.pdf - | head -20

# If output is empty, PDF is image-based
# You'll need OCR to extract text (not currently supported)
# Workaround: Use OCR tool first
# Example with tesseract:
# pdftoppm document.pdf output -png
# tesseract output.png output
# agentroot collection add output.txt --provider file
```

Alternative: Some PDFs have copy protection that prevents text extraction. Try opening in PDF viewer and checking if text is selectable.

### PDF Provider File Not Found

**Problem**: `File not found` when adding PDF collection

**Solution**: Verify path and file extension:

```bash
# Check file exists
ls -la /path/to/document.pdf

# Verify .pdf extension
file /path/to/document.pdf
# Should show: PDF document, version X.X

# For directories, ensure pattern matches
agentroot collection add /path/to/pdfs \
  --mask '**/*.pdf' \  # Case-sensitive!
  --provider pdf

# Check for uppercase extension
ls /path/to/pdfs/*.PDF
# If found, add pattern:
agentroot collection add /path/to/pdfs \
  --mask '**/*.{pdf,PDF}' \
  --provider pdf
```

### PDF Provider Permission Denied

**Problem**: `Permission denied` when reading PDF files

**Solution**: Ensure read permissions:

```bash
# Check permissions
ls -la /path/to/document.pdf

# Fix permissions
chmod u+r /path/to/document.pdf

# For directory
chmod -R u+r /path/to/pdfs/
```

### SQL Provider Database Not Found

**Problem**: `Database not found` or `unable to open database file`

**Solution**: Verify database path and format:

```bash
# Check file exists
ls -la /path/to/database.sqlite

# Verify it's a valid SQLite database
file /path/to/database.sqlite
# Should show: SQLite 3.x database

# Test with sqlite3
sqlite3 /path/to/database.sqlite "SELECT COUNT(*) FROM sqlite_master;"
# Should return a number (not an error)
```

### SQL Provider Invalid Query

**Problem**: `SQL error: syntax error` or query fails

**Solution**: Test query in sqlite3 first:

```bash
# Test query
sqlite3 /path/to/database.sqlite
> SELECT id, title, content FROM posts WHERE published = 1;

# If query fails, fix syntax
# Common issues:
# - Missing quotes around string literals
# - Invalid column names
# - Wrong table name

# Escape JSON quotes properly in shell
agentroot collection add /path/to/db.sqlite \
  --provider sql \
  --config '{"query":"SELECT id, title, body FROM posts WHERE status = \"published\""}'
```

### SQL Provider Column Mapping Error

**Problem**: Results have wrong titles or content

**Solution**: Specify column mapping explicitly:

```bash
# Check table structure
sqlite3 /path/to/database.sqlite
> .schema posts

# Map columns correctly
agentroot collection add /path/to/db.sqlite \
  --provider sql \
  --config '{
    "table":"posts",
    "id_column":"post_id",
    "title_column":"headline",
    "content_column":"body_text"
  }'
```

### SQL Provider Empty Results

**Problem**: SQL collection created but no documents indexed

**Solution**: Verify query returns rows:

```bash
# Test query
sqlite3 /path/to/database.sqlite
> SELECT id, title, content FROM posts;
# Should show rows

# Check if WHERE clause is too restrictive
agentroot collection add /path/to/db.sqlite \
  --provider sql \
  --config '{"table":"posts"}'  # Index all rows first

# Then refine with query
agentroot collection remove posts
agentroot collection add /path/to/db.sqlite \
  --provider sql \
  --config '{"query":"SELECT id, title, content FROM posts WHERE published = 1"}'
```

## Indexing Issues

### Database Locked

**Problem**: `Database error: database is locked`

**Solution**: Another Agentroot process is accessing the database:

```bash
# Find the process
ps aux | grep agentroot

# Kill if needed
killall agentroot

# Or wait for operation to complete
```

If problem persists, remove lock file:

```bash
rm ~/.cache/agentroot/index.sqlite-wal
rm ~/.cache/agentroot/index.sqlite-shm
```

### Permission Denied

**Problem**: `Permission denied` when updating collection

**Solution**: Ensure read access to collection directory:

```bash
ls -la /path/to/collection

# Fix permissions if needed
chmod -R u+r /path/to/collection
```

For database permissions:

```bash
ls -la ~/.cache/agentroot/
chmod u+rw ~/.cache/agentroot/index.sqlite
```

### Out of Disk Space

**Problem**: `No space left on device`

**Solution**: Database and models require disk space:

- Database: Grows with indexed content (typically 10-50MB per 1000 files)
- Models: ~100MB for nomic-embed-text-v1.5
- Cache: Grows with embeddings (typically 3KB per chunk)

Check space:

```bash
df -h ~/.cache/agentroot
df -h ~/.local/share/agentroot
```

Clean up old data:

```bash
# Remove unused chunk embeddings
agentroot cleanup

# Vacuum database
agentroot cleanup --vacuum

# Remove entire database (lose all data)
rm -rf ~/.cache/agentroot/index.sqlite
```

## Embedding Issues

### Model Download Fails

**Problem**: `HTTP error: connection timeout` or `Network error`

**Solution**: Model download requires internet connection. Check connectivity:

```bash
# Test connection
curl -I https://huggingface.co

# Try again
agentroot embed
```

If behind proxy, set environment variables:

```bash
export HTTP_PROXY=http://proxy.example.com:8080
export HTTPS_PROXY=http://proxy.example.com:8080
```

### Model File Corrupted

**Problem**: `LLM error: Failed to load model`

**Solution**: Remove and re-download model:

```bash
rm -rf ~/.local/share/agentroot/models/
agentroot embed
```

### Out of Memory

**Problem**: Process killed or `Cannot allocate memory`

**Solution**: Embedding models require RAM:

- nomic-embed-text-v1.5: ~1-2GB RAM
- Batch processing: Additional ~500MB

On low-memory systems, close other applications or use swap:

```bash
# Check memory usage
free -h

# Monitor during embedding
watch -n 1 free -h
```

### Embedding Takes Too Long

**Problem**: `agentroot embed` runs for hours

**Solution**: Large codebases take time. Monitor progress:

```bash
# Use verbose mode
agentroot embed -v

# Check status in another terminal
agentroot status
```

Typical speeds:
- First run: ~50-100 chunks/second (CPU dependent)
- With cache: 5-10x faster

For very large repos (>10K files), consider:
- Excluding test directories: `--exclude '**/test/**'`
- Indexing only relevant file types
- Using release build (if running from source)

## Search Issues

### No Results Found

**Problem**: Search returns empty results

**Solution**: Verify index exists and has content:

```bash
# Check status
agentroot status

# Verify documents are indexed
agentroot ls <collection>

# Try broader search
agentroot search "common-word"

# Check collection filter
agentroot search "query" -c correct-collection-name
```

### Vector Search Not Available

**Problem**: `Vector index not found. Run 'agentroot embed' first.`

**Solution**: Generate embeddings:

```bash
agentroot embed
```

This is required before using `vsearch` or `query` commands.

### Search Results Irrelevant

**Problem**: Search returns unrelated documents

**Solution**: Different search methods have different strengths:

**For exact matches** (function names, error codes):
```bash
agentroot search "exact_function_name"
```

**For concepts** (natural language):
```bash
agentroot vsearch "how to handle database errors"
```

**Best overall quality**:
```bash
agentroot query "database error handling"
```

Adjust minimum score to filter low-quality results:

```bash
agentroot search "query" --min-score 0.5
```

### FTS5 Query Syntax Errors

**Problem**: `FTS5 syntax error` or unexpected results

**Solution**: Special characters need escaping or quotes:

```bash
# Wrong: hyphen interpreted as NOT
agentroot search tree-sitter

# Right: use quotes
agentroot search '"tree-sitter"'

# Wrong: unbalanced quotes
agentroot search 'can't

# Right: escape or use different quotes
agentroot search "can't"
```

Common FTS5 operators:
- Phrase: `"exact phrase"`
- OR: `term1 OR term2`
- NOT: `term1 NOT term2`
- Prefix: `prefix*`

## Performance Issues

### Slow Searches

**Problem**: Searches take several seconds

**Solution**: 

**BM25 search** should be < 10ms. If slow:

```bash
# Rebuild FTS5 index
agentroot cleanup --vacuum
```

**Vector search** depends on corpus size:
- <1K docs: <100ms
- 1-10K docs: 100-500ms
- >10K docs: May need filtering by collection

Use collection filter:

```bash
agentroot vsearch "query" -c specific-collection
```

### High Memory Usage

**Problem**: Agentroot uses several GB of RAM

**Solution**: Vector search loads embeddings into memory. For large corpora:

1. Filter by collection when searching
2. Reduce indexed documents
3. Use BM25 search when semantic understanding not needed

Check memory:

```bash
ps aux | grep agentroot
```

### Large Database File

**Problem**: `index.sqlite` grows to several GB

**Solution**: Database size depends on indexed content:

- Documents: ~10KB per file (average)
- Embeddings: ~3KB per chunk (768-dim f32)
- FTS5 index: ~30% of document size

Optimize database:

```bash
# Remove orphaned data
agentroot cleanup

# Compact database
agentroot cleanup --vacuum
```

Check size:

```bash
du -sh ~/.cache/agentroot/
```

## MCP Integration Issues

### Tools Not Appearing in Claude

**Problem**: Claude doesn't show Agentroot tools

**Solution**: 

1. Verify config file location:
   - macOS: `~/Library/Application Support/Claude/claude_desktop_config.json`
   - Linux: `~/.config/Claude/claude_desktop_config.json`

2. Check JSON syntax:
   ```bash
   cat ~/Library/Application\ Support/Claude/claude_desktop_config.json | jq
   ```

3. Verify agentroot in PATH:
   ```bash
   which agentroot
   ```

4. Restart Claude Desktop completely

5. Check Claude logs for errors (location varies by OS)

### MCP Server Crashes

**Problem**: Tools fail with connection errors

**Solution**: Test MCP server manually:

```bash
# Test initialization
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' | agentroot mcp

# Should output JSON response
```

Enable debug logging:

```bash
RUST_LOG=debug agentroot mcp
```

### Search Results Not Showing

**Problem**: MCP tools run but no results in Claude

**Solution**: Verify index has data:

```bash
agentroot status
```

Test search directly:

```bash
agentroot search "test query"
```

If direct search works but MCP doesn't, check Claude's prompt and tool call details.

## Debugging

### Enable Verbose Logging

Set log level:

```bash
# Info level
RUST_LOG=info agentroot update

# Debug level (very verbose)
RUST_LOG=debug agentroot embed

# Trace level (extremely verbose)
RUST_LOG=trace agentroot search "query"
```

Filter by module:

```bash
# Only database logs
RUST_LOG=agentroot_core::db=debug agentroot update

# Only indexing logs
RUST_LOG=agentroot_core::index=trace agentroot update
```

### Inspect Database

Use sqlite3 to inspect database:

```bash
sqlite3 ~/.cache/agentroot/index.sqlite

# List collections
.mode column
.headers on
SELECT * FROM collections;

# Count documents
SELECT COUNT(*) FROM documents;

# Check embeddings
SELECT model, COUNT(*) FROM chunk_embeddings GROUP BY model;

# Exit
.quit
```

### Check File Locations

Verify Agentroot directories:

```bash
# Database location
ls -lh ~/.cache/agentroot/

# Models location
ls -lh ~/.local/share/agentroot/models/

# Config (if exists)
ls -lh ~/.config/agentroot/
```

Override locations:

```bash
export AGENTROOT_DB=/custom/path/index.sqlite
export AGENTROOT_MODELS=/custom/path/models
```

### Test with Fresh Database

Start with clean slate:

```bash
# Backup existing database
mv ~/.cache/agentroot ~/.cache/agentroot.backup

# Run with fresh database
agentroot collection add /path --name test
agentroot update
agentroot embed
agentroot search "test"

# If it works, issue was with old database
# Restore backup if needed:
# rm -rf ~/.cache/agentroot
# mv ~/.cache/agentroot.backup ~/.cache/agentroot
```

## Reporting Issues

If none of these solutions work, gather information for bug report:

```bash
# Version
agentroot --version

# System info
uname -a

# Rust version
rustc --version

# Status
agentroot status

# Log output
RUST_LOG=debug agentroot <command> 2>&1 | tee agentroot-debug.log
```

Include:
1. Agentroot version
2. Operating system
3. Command that failed
4. Full error message
5. Debug logs

Report issues at: https://github.com/spacejar/agentroot/issues

## FAQ

### Why are my changes not appearing in search?

Run `agentroot update` to re-scan files. The index is not automatically updated.

### Do I need to re-embed after every change?

For search: No, BM25 search works immediately after `update`.

For vector search: Yes, run `agentroot embed` to update embeddings. The cache makes this fast (only changed chunks are re-embedded).

### Can I exclude certain directories?

Yes, use `--exclude` when adding collections:

```bash
agentroot collection add ./src --name code \
  --mask '**/*.rs' \
  --exclude '**/target/**' \
  --exclude '**/test/**'
```

### How much disk space do I need?

Rough estimates:
- Database: 10-50MB per 1000 files
- Models: 100MB (one-time download)
- Cache: 3KB per chunk (typically 5-10 chunks per file)

For 10,000 files: ~500MB-1GB total.

### Can I use custom embedding models?

Not currently. Agentroot uses nomic-embed-text-v1.5 (768 dimensions). Custom model support may be added in future versions.

### Is my data sent to any servers?

No. Agentroot runs entirely locally. The only network request is downloading the embedding model on first run (from Hugging Face).

### Can I use Agentroot for non-code files?

Yes. Agentroot works with any text files. Markdown, documentation, notes, logs, configuration files, etc. AST-aware chunking only applies to supported code languages; other files use character-based chunking.

### How do I uninstall?

```bash
# Remove binary
cargo uninstall agentroot

# Remove data
rm -rf ~/.cache/agentroot
rm -rf ~/.local/share/agentroot
rm -rf ~/.config/agentroot
```
