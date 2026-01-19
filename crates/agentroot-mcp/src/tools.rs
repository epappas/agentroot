//! MCP tool definitions and handlers

use agentroot_core::{Database, SearchOptions};
use serde_json::Value;
use anyhow::Result;
use crate::protocol::*;

pub fn search_tool_definition() -> ToolDefinition {
    ToolDefinition {
        name: "search".to_string(),
        description: "BM25 full-text search across your knowledge base".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Search query (keywords or phrases)"
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum results (default: 20)",
                    "default": 20
                },
                "minScore": {
                    "type": "number",
                    "description": "Minimum relevance score 0-1 (default: 0)",
                    "default": 0
                },
                "collection": {
                    "type": "string",
                    "description": "Filter by collection name"
                }
            },
            "required": ["query"]
        }),
    }
}

pub fn vsearch_tool_definition() -> ToolDefinition {
    ToolDefinition {
        name: "vsearch".to_string(),
        description: "Vector similarity search using embeddings".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Search query (natural language)"
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum results (default: 20)",
                    "default": 20
                },
                "minScore": {
                    "type": "number",
                    "description": "Minimum similarity score 0-1 (default: 0.3)",
                    "default": 0.3
                },
                "collection": {
                    "type": "string",
                    "description": "Filter by collection name"
                }
            },
            "required": ["query"]
        }),
    }
}

pub fn query_tool_definition() -> ToolDefinition {
    ToolDefinition {
        name: "query".to_string(),
        description: "Hybrid search with BM25, vectors, and reranking (best quality)".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Search query"
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum results (default: 20)",
                    "default": 20
                },
                "collection": {
                    "type": "string",
                    "description": "Filter by collection name"
                }
            },
            "required": ["query"]
        }),
    }
}

pub fn get_tool_definition() -> ToolDefinition {
    ToolDefinition {
        name: "get".to_string(),
        description: "Get a document by path, docid, or virtual path".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "file": {
                    "type": "string",
                    "description": "File path, docid (#abc123), or agentroot:// URI"
                },
                "fromLine": {
                    "type": "integer",
                    "description": "Start from line number"
                },
                "maxLines": {
                    "type": "integer",
                    "description": "Maximum lines to return"
                },
                "lineNumbers": {
                    "type": "boolean",
                    "description": "Include line numbers",
                    "default": false
                }
            },
            "required": ["file"]
        }),
    }
}

pub fn multi_get_tool_definition() -> ToolDefinition {
    ToolDefinition {
        name: "multi_get".to_string(),
        description: "Get multiple documents by glob pattern or comma-separated list".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Glob pattern or comma-separated list of paths/docids"
                },
                "maxLines": {
                    "type": "integer",
                    "description": "Maximum lines per file"
                },
                "maxBytes": {
                    "type": "integer",
                    "description": "Skip files larger than this (default: 10240)",
                    "default": 10240
                },
                "lineNumbers": {
                    "type": "boolean",
                    "description": "Include line numbers",
                    "default": false
                }
            },
            "required": ["pattern"]
        }),
    }
}

pub fn status_tool_definition() -> ToolDefinition {
    ToolDefinition {
        name: "status".to_string(),
        description: "Show index status and collection information".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {}
        }),
    }
}

pub async fn handle_search(db: &Database, args: Value) -> Result<ToolResult> {
    let query = args.get("query")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing query"))?;

    let options = SearchOptions {
        limit: args.get("limit").and_then(|v| v.as_u64()).unwrap_or(20) as usize,
        min_score: args.get("minScore").and_then(|v| v.as_f64()).unwrap_or(0.0),
        collection: args.get("collection").and_then(|v| v.as_str()).map(String::from),
        full_content: false,
    };

    let results = db.search_fts(query, &options)?;

    let summary = format!("Found {} results for \"{}\"", results.len(), query);
    let structured: Vec<Value> = results.iter().map(|r| {
        serde_json::json!({
            "docid": format!("#{}", r.docid),
            "file": r.display_path,
            "title": r.title,
            "score": (r.score * 100.0).round() / 100.0
        })
    }).collect();

    Ok(ToolResult {
        content: vec![Content::Text { text: summary }],
        structured_content: Some(serde_json::json!({ "results": structured })),
        is_error: None,
    })
}

pub async fn handle_vsearch(db: &Database, _args: Value) -> Result<ToolResult> {
    if !db.has_vector_index() {
        return Ok(ToolResult {
            content: vec![Content::Text {
                text: "Vector index not found. Run 'agentroot embed' first.".to_string()
            }],
            structured_content: None,
            is_error: Some(true),
        });
    }

    // TODO: Implement when embedder is available
    Ok(ToolResult {
        content: vec![Content::Text { text: "Vector search not yet implemented".to_string() }],
        structured_content: None,
        is_error: Some(true),
    })
}

pub async fn handle_query(db: &Database, args: Value) -> Result<ToolResult> {
    // Fallback to BM25 for now
    handle_search(db, args).await
}

pub async fn handle_get(db: &Database, args: Value) -> Result<ToolResult> {
    let file = args.get("file")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing file"))?;

    let doc = db.find_by_docid(file)?
        .ok_or_else(|| anyhow::anyhow!("Document not found: {}", file))?;

    let body = doc.body.unwrap_or_default();

    Ok(ToolResult {
        content: vec![Content::Resource {
            resource: ResourceContent {
                uri: doc.filepath,
                name: doc.display_path,
                title: Some(doc.title),
                mime_type: "text/markdown".to_string(),
                text: body,
            }
        }],
        structured_content: None,
        is_error: None,
    })
}

pub async fn handle_multi_get(db: &Database, args: Value) -> Result<ToolResult> {
    let pattern = args.get("pattern")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing pattern"))?;

    let docs = db.fuzzy_find_documents(pattern, 10)?;

    let contents: Vec<Content> = docs.into_iter().map(|doc| {
        Content::Resource {
            resource: ResourceContent {
                uri: doc.filepath,
                name: doc.display_path,
                title: Some(doc.title),
                mime_type: "text/markdown".to_string(),
                text: doc.body.unwrap_or_default(),
            }
        }
    }).collect();

    Ok(ToolResult {
        content: contents,
        structured_content: None,
        is_error: None,
    })
}

pub async fn handle_status(db: &Database) -> Result<ToolResult> {
    let collections = db.list_collections()?;
    let total_docs: usize = collections.iter().map(|c| c.document_count).sum();
    let needs_embedding = db.count_hashes_needing_embedding()?;
    let has_vector = db.has_vector_index();

    let summary = format!(
        "Index: {} documents across {} collections\n\
         Embeddings: {}\n\
         Vector index: {}",
        total_docs,
        collections.len(),
        if needs_embedding > 0 {
            format!("{} documents need embedding", needs_embedding)
        } else {
            "Up to date".to_string()
        },
        if has_vector { "Available" } else { "Not created" }
    );

    let structured = serde_json::json!({
        "totalDocuments": total_docs,
        "needsEmbedding": needs_embedding,
        "hasVectorIndex": has_vector,
        "collections": collections.iter().map(|c| serde_json::json!({
            "name": c.name,
            "path": c.path,
            "pattern": c.pattern,
            "documents": c.document_count
        })).collect::<Vec<_>>()
    });

    Ok(ToolResult {
        content: vec![Content::Text { text: summary }],
        structured_content: Some(structured),
        is_error: None,
    })
}
