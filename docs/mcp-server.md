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

The MCP server exposes six tools for AI assistants:

### 1. search

BM25 full-text search across your knowledge base.

**Parameters**:
- `query` (string, required) - Search keywords or phrases
- `limit` (integer, optional) - Maximum results (default: 20)
- `minScore` (number, optional) - Minimum relevance score 0-1 (default: 0)
- `collection` (string, optional) - Filter by collection name

**Returns**: List of matching documents with scores.

**Example tool call**:
```json
{
  "name": "search",
  "arguments": {
    "query": "error handling",
    "limit": 10,
    "minScore": 0.5
  }
}
```

### 2. vsearch

Vector similarity search using embeddings.

**Parameters**:
- `query` (string, required) - Natural language search query
- `limit` (integer, optional) - Maximum results (default: 20)
- `minScore` (number, optional) - Minimum similarity score 0-1 (default: 0.3)
- `collection` (string, optional) - Filter by collection name

**Returns**: Semantically similar documents.

**Note**: Requires embeddings to be generated first (`agentroot embed`). Currently returns an error if vector index is not available.

### 3. query

Hybrid search combining BM25 and vector similarity.

**Parameters**:
- `query` (string, required) - Search query
- `limit` (integer, optional) - Maximum results (default: 20)
- `collection` (string, optional) - Filter by collection name

**Returns**: Best results from combined search approaches.

**Note**: Currently falls back to BM25 search. Full hybrid implementation pending.

### 4. get

Retrieve a single document by path, docid, or virtual URI.

**Parameters**:
- `file` (string, required) - File path, docid (#abc123), or agentroot:// URI
- `fromLine` (integer, optional) - Start from line number
- `maxLines` (integer, optional) - Maximum lines to return
- `lineNumbers` (boolean, optional) - Include line numbers (default: false)

**Returns**: Document content as a resource.

**Example tool call**:
```json
{
  "name": "get",
  "arguments": {
    "file": "#a1b2c3"
  }
}
```

### 5. multi_get

Retrieve multiple documents by glob pattern or comma-separated list.

**Parameters**:
- `pattern` (string, required) - Glob pattern or comma-separated paths/docids
- `maxLines` (integer, optional) - Maximum lines per file
- `maxBytes` (integer, optional) - Skip files larger than this (default: 10240)
- `lineNumbers` (boolean, optional) - Include line numbers (default: false)

**Returns**: Array of document resources.

**Example tool call**:
```json
{
  "name": "multi_get",
  "arguments": {
    "pattern": "myproject/src/*.rs"
  }
}
```

### 6. status

Show index status and collection information.

**Parameters**: None

**Returns**: Statistics about indexed documents, collections, and embedding status.

**Example tool call**:
```json
{
  "name": "status",
  "arguments": {}
}
```

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

Response includes all six tools with their schemas.

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

1. **Vector search not fully implemented** - `vsearch` returns an error if embeddings aren't available
2. **Hybrid search falls back to BM25** - `query` currently uses only BM25
3. **No subscription support** - Resources don't support real-time updates
4. **No prompt templates** - Only basic prompt definitions
5. **No batch operations** - Tools must be called individually

These limitations may be addressed in future versions.

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

- [MCP Specification](https://spec.modelcontextprotocol.io/)
- [Claude Desktop MCP Integration](https://docs.anthropic.com/claude/docs/mcp)
- [Getting Started Guide](getting-started.md) for indexing basics
- [CLI Reference](cli-reference.md) for command details
