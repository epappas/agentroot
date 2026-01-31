# MCP Server Integration

Agentroot includes a Model Context Protocol (MCP) server for integration with AI assistants like Claude Desktop, Continue.dev, and other MCP-compatible tools.

## What is MCP?

The Model Context Protocol is a standard for connecting AI assistants to external tools and data sources. Agentroot's MCP server exposes search capabilities through this protocol, allowing AI assistants to search your indexed codebase.

## Starting the MCP Server

The MCP server communicates via JSON-RPC over standard input/output:

```bash
agentroot mcp
```

This starts a server that:
- Reads JSON-RPC requests from stdin
- Writes JSON-RPC responses to stdout
- Runs until stdin is closed

## Available Tools

The MCP server exposes 16 tools for AI assistants:

### Search Tools

#### 1. search

BM25 full-text search across your knowledge base.

**Parameters**:
- `query` (string, required) - Search keywords or phrases
- `limit` (integer, optional) - Maximum results (default: 20)
- `minScore` (number, optional) - Minimum relevance score 0-1 (default: 0)
- `collection` (string, optional) - Filter by collection name
- `provider` (string, optional) - Filter by provider type
- `category` (string, optional) - Filter by LLM-generated category
- `difficulty` (string, optional) - Filter by difficulty level
- `concept` (string, optional) - Filter by concept/keyword

**Returns**: List of matching documents with scores, metadata, and summaries.

**Example tool call**:
```json
{
  "name": "search",
  "arguments": {
    "query": "error handling",
    "limit": 10,
    "category": "tutorial"
  }
}
```

#### 2. vsearch

Vector similarity search using embeddings.

**Parameters**:
- `query` (string, required) - Natural language search query
- `limit` (integer, optional) - Maximum results (default: 20)
- `minScore` (number, optional) - Minimum similarity score 0-1 (default: 0.3)
- `collection` (string, optional) - Filter by collection name
- `provider`, `category`, `difficulty`, `concept` (optional) - Metadata filters

**Returns**: Semantically similar documents.

**Note**: Requires embeddings to be generated first (`agentroot embed`).

#### 3. query

Hybrid search combining BM25 and vector similarity with RRF fusion.

**Parameters**:
- `query` (string, required) - Search query
- `limit` (integer, optional) - Maximum results (default: 20)
- `collection` (string, optional) - Filter by collection name
- `provider`, `category`, `difficulty`, `concept` (optional) - Metadata filters

**Returns**: Best results from combined search approaches.

#### 4. smart_search

Intelligent natural language search with automatic query understanding and fallback.

**Parameters**:
- `query` (string, required) - Natural language query
- `limit` (integer, optional) - Maximum results (default: 20)
- `minScore` (number, optional) - Minimum relevance score
- `collection` (string, optional) - Filter by collection name

**Returns**: Search results with automatic strategy selection.

### Document Retrieval Tools

#### 5. get

Retrieve a single document by path, docid, or virtual URI.

**Parameters**:
- `file` (string, required) - File path, docid (#abc123), or agentroot:// URI
- `fromLine` (integer, optional) - Start from line number
- `maxLines` (integer, optional) - Maximum lines to return
- `lineNumbers` (boolean, optional) - Include line numbers (default: false)

**Returns**: Document content as a resource.

```json
{
  "name": "get",
  "arguments": { "file": "#a1b2c3" }
}
```

#### 6. multi_get

Retrieve multiple documents by glob pattern or comma-separated list.

**Parameters**:
- `pattern` (string, required) - Glob pattern or comma-separated paths/docids
- `maxLines` (integer, optional) - Maximum lines per file
- `maxBytes` (integer, optional) - Skip files larger than this (default: 10240)
- `lineNumbers` (boolean, optional) - Include line numbers (default: false)

**Returns**: Array of document resources.

#### 7. status

Show index status and collection information.

**Parameters**: None

**Returns**: Statistics about indexed documents, collections, and embedding status.

### Collection Management Tools

#### 8. collection_add

Add a new collection to index.

**Parameters**:
- `name` (string, required) - Collection name
- `path` (string, required) - Directory path or URL
- `pattern` (string, optional) - Glob pattern (default: `**/*.md`)
- `provider` (string, optional) - Provider type: file, github, url, pdf, sql
- `config` (string, optional) - JSON provider config

#### 9. collection_remove

Remove a collection and its documents.

**Parameters**:
- `name` (string, required) - Collection name to remove

#### 10. collection_update

Reindex a collection (scan for new/changed documents).

**Parameters**:
- `name` (string, required) - Collection name to reindex

### Metadata Tools

#### 11. metadata_add

Add custom user metadata to a document.

**Parameters**:
- `docid` (string, required) - Document ID (e.g., `#a1b2c3`)
- `metadata` (object, required) - Key-value metadata pairs

**Example**:
```json
{
  "name": "metadata_add",
  "arguments": {
    "docid": "#e192a2",
    "metadata": { "author": "Alice", "difficulty": 3 }
  }
}
```

#### 12. metadata_get

Get custom user metadata from a document.

**Parameters**:
- `docid` (string, required) - Document ID

#### 13. metadata_query

Query documents by custom user metadata.

**Parameters**:
- `field` (string, required) - Metadata field name
- `operator` (string, required) - One of: `eq`, `contains`, `gt`, `lt`, `has`, `exists`
- `value` (string, optional) - Value to compare against
- `limit` (integer, optional) - Maximum results (default: 20)

**Example**:
```json
{
  "name": "metadata_query",
  "arguments": { "field": "author", "operator": "eq", "value": "Alice" }
}
```

### Chunk Navigation Tools

#### 14. search_chunks

Search for specific code chunks (functions, methods, classes).

**Parameters**:
- `query` (string, required) - Search query
- `limit` (integer, optional) - Maximum results (default: 10)
- `minScore` (number, optional) - Minimum relevance score
- `collection` (string, optional) - Filter by collection
- `label` (string, optional) - Filter by label (format: `key:value`)

**Returns**: Matching chunks with type, breadcrumb, line ranges, and labels.

#### 15. get_chunk

Retrieve a specific code chunk by its hash, including all metadata.

**Parameters**:
- `chunk_hash` (string, required) - Chunk hash
- `include_context` (boolean, optional) - Include surrounding chunks (default: false)

#### 16. navigate_chunks

Navigate to previous or next chunk within the same document.

**Parameters**:
- `chunk_hash` (string, required) - Starting chunk hash
- `direction` (string, required) - `prev` or `next`

## Integration with Claude Desktop

To integrate Agentroot with Claude Desktop, add this configuration:

### macOS

Edit `~/Library/Application Support/Claude/claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "agentroot": {
      "command": "agentroot",
      "args": ["mcp"]
    }
  }
}
```

### Linux

Edit `~/.config/Claude/claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "agentroot": {
      "command": "agentroot",
      "args": ["mcp"]
    }
  }
}
```

### Windows

Edit `%APPDATA%\Claude\claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "agentroot": {
      "command": "agentroot",
      "args": ["mcp"]
    }
  }
}
```

After editing the config, restart Claude Desktop. The Agentroot tools will appear in Claude's tool palette.

## Integration with Claude Code

Add to your `.claude/settings.json` or project-level `.mcp.json`:

```json
{
  "mcpServers": {
    "agentroot": {
      "command": "agentroot",
      "args": ["mcp"]
    }
  }
}
```

To use a custom database path:

```json
{
  "mcpServers": {
    "agentroot": {
      "command": "agentroot",
      "args": ["mcp"],
      "env": {
        "AGENTROOT_DB": "/path/to/index.sqlite"
      }
    }
  }
}
```

## Integration with Continue.dev

Add this to your Continue configuration (`.continue/config.json`):

```json
{
  "experimental": {
    "modelContextProtocolServers": [
      {
        "name": "agentroot",
        "command": "agentroot",
        "args": ["mcp"]
      }
    ]
  }
}
```

## Protocol Details

The MCP server implements the Model Context Protocol version 2024-11-05 via JSON-RPC 2.0.

### Initialization

On startup, AI assistants send an `initialize` request:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "initialize",
  "params": {}
}
```

The server responds with capabilities:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "protocolVersion": "2024-11-05",
    "capabilities": {
      "tools": {},
      "resources": { "subscribe": false },
      "prompts": {}
    },
    "serverInfo": {
      "name": "agentroot",
      "version": "0.1.0"
    }
  }
}
```

### Tool Discovery

AI assistants discover available tools via `tools/list`:

```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "method": "tools/list",
  "params": {}
}
```

Response includes all 16 tools with their schemas.

### Tool Invocation

AI assistants call tools via `tools/call`:

```json
{
  "jsonrpc": "2.0",
  "id": 3,
  "method": "tools/call",
  "params": {
    "name": "search",
    "arguments": {
      "query": "error handling",
      "limit": 10
    }
  }
}
```

The server returns results:

```json
{
  "jsonrpc": "2.0",
  "id": 3,
  "result": {
    "content": [
      {
        "type": "text",
        "text": "Found 3 results for \"error handling\""
      }
    ],
    "structured_content": {
      "results": [
        {
          "docid": "#a1b2c3",
          "file": "myproject/src/error.rs",
          "title": "Error Handling Module",
          "score": 0.87
        }
      ]
    }
  }
}
```

## Usage Examples

### Searching from Claude

Once integrated, you can ask Claude:

```
Search my codebase for error handling patterns
```

Claude will use the `search` tool to find relevant documents and provide insights based on your code.

### Getting Files

```
Show me the contents of myproject/src/main.rs
```

Claude will use the `get` tool to retrieve the file and discuss it.

### Status Checks

```
What's the status of my Agentroot index?
```

Claude will use the `status` tool to show statistics.

## Debugging

Enable debug logging to see MCP protocol messages:

```bash
RUST_LOG=debug agentroot mcp
```

This will log all JSON-RPC requests and responses to stderr.

## Security Considerations

The MCP server:
- Only reads from your indexed database
- Cannot modify your filesystem
- Cannot execute arbitrary commands
- Only exposes search and retrieval operations

However:
- It has access to all indexed content
- AI assistants can read any document in your collections
- Ensure you trust the AI assistant before integrating

Do not index sensitive files (passwords, API keys, credentials) if using MCP integration with external AI services.

## Limitations

Current limitations of the MCP server:

1. **Vector/hybrid search requires embeddings** - `vsearch` and `query` need `agentroot embed` to have been run first; they fall back to BM25 otherwise
2. **No subscription support** - Resources don't support real-time updates
3. **No batch operations** - Tools must be called individually

## Troubleshooting

### Tool not appearing in Claude

1. Verify config file location and JSON syntax
2. Restart Claude Desktop completely
3. Check that `agentroot` is in your PATH:
   ```bash
   which agentroot
   ```
4. Test MCP server manually:
   ```bash
   echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' | agentroot mcp
   ```

### Search returns no results

1. Verify collections are indexed:
   ```bash
   agentroot status
   ```
2. Run update if needed:
   ```bash
   agentroot update
   ```
3. Check collection names match your queries

### Vector search fails

1. Generate embeddings first:
   ```bash
   agentroot embed
   ```
2. This downloads the model (~100MB) on first run
3. Verify embeddings exist:
   ```bash
   agentroot status
   ```

## Further Reading

- [Integration Guide](integration-guide.md) for SDK, CLI, and MCP integration
- [MCP Specification](https://spec.modelcontextprotocol.io/)
- [Getting Started Guide](getting-started.md) for indexing basics
- [CLI Reference](cli-reference.md) for command details
