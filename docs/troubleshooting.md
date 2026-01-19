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
which agentroot
# Should show: /home/user/.cargo/bin/agentroot
```

## Collection Management Issues

### Collection Already Exists

**Problem**: `Collection 'myproject' already exists`

**Solution**: Remove the old collection first or use a different name:

```bash
# Remove old collection
agentroot collection remove myproject

# Or use different name
agentroot collection add /path --name myproject-v2
```

### Collection Not Found

**Problem**: `Collection not found: myproject`

**Solution**: List existing collections to verify name:

```bash
agentroot collection list
```

Collection names are case-sensitive. Use exact name shown in list.

### No Files Matched

**Problem**: After adding collection, `agentroot update` shows 0 files

**Solution**: Check your file mask pattern:

```bash
# Verify files exist
ls /path/to/collection/**/*.rs

# Try without mask first
agentroot collection add /path --name test

# Then add specific mask
agentroot collection remove test
agentroot collection add /path --name test --mask '**/*.rs'
```

Common mask patterns:
- `**/*.rs` - All Rust files recursively
- `*.md` - Markdown files in root only
- `**/*.{rs,py}` - Multiple extensions (shell dependent)

### Path Does Not Exist

**Problem**: `IO error: No such file or directory`

**Solution**: Ensure path is absolute or relative to current directory:

```bash
# Use absolute path
agentroot collection add /home/user/projects/myapp --name myapp

# Or relative to current directory
cd ~/projects
agentroot collection add ./myapp --name myapp

# Expand ~ manually if needed
agentroot collection add $HOME/projects/myapp --name myapp
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
