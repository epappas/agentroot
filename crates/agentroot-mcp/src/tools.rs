//! MCP tool definitions and handlers

use crate::protocol::*;
use agentroot_core::{Database, SearchOptions};
use anyhow::Result;
use serde_json::Value;

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
                },
                "provider": {
                    "type": "string",
                    "description": "Filter by provider type (file, github, url, etc.)"
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
                },
                "provider": {
                    "type": "string",
                    "description": "Filter by provider type (file, github, url, etc.)"
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
                },
                "provider": {
                    "type": "string",
                    "description": "Filter by provider type (file, github, url, etc.)"
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

pub fn collection_add_tool_definition() -> ToolDefinition {
    ToolDefinition {
        name: "collection_add".to_string(),
        description: "Add a new collection to index".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "Collection name"
                },
                "path": {
                    "type": "string",
                    "description": "Path to local directory or URL"
                },
                "pattern": {
                    "type": "string",
                    "description": "Glob pattern for files (default: **/*.md)",
                    "default": "**/*.md"
                },
                "provider": {
                    "type": "string",
                    "description": "Provider type: file, github, url (default: file)",
                    "default": "file"
                },
                "config": {
                    "type": "string",
                    "description": "Provider-specific JSON configuration"
                }
            },
            "required": ["name", "path"]
        }),
    }
}

pub fn collection_remove_tool_definition() -> ToolDefinition {
    ToolDefinition {
        name: "collection_remove".to_string(),
        description: "Remove a collection and its documents".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "Collection name to remove"
                }
            },
            "required": ["name"]
        }),
    }
}

pub fn collection_update_tool_definition() -> ToolDefinition {
    ToolDefinition {
        name: "collection_update".to_string(),
        description: "Reindex a collection (scan for new/changed documents)".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "Collection name to update"
                }
            },
            "required": ["name"]
        }),
    }
}

pub async fn handle_search(db: &Database, args: Value) -> Result<ToolResult> {
    let query = args
        .get("query")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing query"))?;

    let options = SearchOptions {
        limit: args.get("limit").and_then(|v| v.as_u64()).unwrap_or(20) as usize,
        min_score: args.get("minScore").and_then(|v| v.as_f64()).unwrap_or(0.0),
        collection: args
            .get("collection")
            .and_then(|v| v.as_str())
            .map(String::from),
        provider: args
            .get("provider")
            .and_then(|v| v.as_str())
            .map(String::from),
        full_content: false,
    };

    let results = db.search_fts(query, &options)?;

    let summary = format!("Found {} results for \"{}\"", results.len(), query);
    let structured: Vec<Value> = results
        .iter()
        .map(|r| {
            serde_json::json!({
                "docid": format!("#{}", r.docid),
                "file": r.display_path,
                "title": r.title,
                "score": (r.score * 100.0).round() / 100.0
            })
        })
        .collect();

    Ok(ToolResult {
        content: vec![Content::Text { text: summary }],
        structured_content: Some(serde_json::json!({ "results": structured })),
        is_error: None,
    })
}

pub async fn handle_vsearch(db: &Database, args: Value) -> Result<ToolResult> {
    if !db.has_vector_index() {
        return Ok(ToolResult {
            content: vec![Content::Text {
                text: "Vector index not found. Run 'agentroot embed' first.".to_string(),
            }],
            structured_content: None,
            is_error: Some(true),
        });
    }

    let query = args
        .get("query")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing query"))?;

    let options = SearchOptions {
        limit: args.get("limit").and_then(|v| v.as_u64()).unwrap_or(20) as usize,
        min_score: args.get("minScore").and_then(|v| v.as_f64()).unwrap_or(0.3),
        collection: args
            .get("collection")
            .and_then(|v| v.as_str())
            .map(String::from),
        provider: args
            .get("provider")
            .and_then(|v| v.as_str())
            .map(String::from),
        full_content: false,
    };

    let embedder = match agentroot_core::LlamaEmbedder::from_default() {
        Ok(e) => e,
        Err(e) => {
            return Ok(ToolResult {
                content: vec![Content::Text {
                    text: format!(
                        "Could not load embedding model: {}. \
                         Download an embedding model to use vector search. \
                         See: https://github.com/epappas/agentroot#embedding-models",
                        e
                    ),
                }],
                structured_content: None,
                is_error: Some(true),
            });
        }
    };

    let results = db.search_vec(query, &embedder, &options).await?;

    let summary = format!("Found {} results for \"{}\"", results.len(), query);
    let structured: Vec<Value> = results
        .iter()
        .map(|r| {
            serde_json::json!({
                "docid": format!("#{}", r.docid),
                "file": r.display_path,
                "title": r.title,
                "score": (r.score * 100.0).round() / 100.0
            })
        })
        .collect();

    Ok(ToolResult {
        content: vec![Content::Text { text: summary }],
        structured_content: Some(serde_json::json!({ "results": structured })),
        is_error: None,
    })
}

pub async fn handle_query(db: &Database, args: Value) -> Result<ToolResult> {
    if !db.has_vector_index() {
        return handle_search(db, args).await;
    }

    let query = args
        .get("query")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing query"))?;

    let options = SearchOptions {
        limit: args.get("limit").and_then(|v| v.as_u64()).unwrap_or(20) as usize,
        min_score: 0.0,
        collection: args
            .get("collection")
            .and_then(|v| v.as_str())
            .map(String::from),
        provider: args
            .get("provider")
            .and_then(|v| v.as_str())
            .map(String::from),
        full_content: false,
    };

    let embedder = match agentroot_core::LlamaEmbedder::from_default() {
        Ok(e) => e,
        Err(_) => {
            return handle_search(db, args).await;
        }
    };

    let bm25_results = db.search_fts(query, &options)?;
    let vec_results = db.search_vec(query, &embedder, &options).await?;

    let fused_results = agentroot_core::search::rrf_fusion(&bm25_results, &vec_results);

    let final_results: Vec<_> = fused_results.into_iter().take(options.limit).collect();

    let summary = format!(
        "Found {} results for \"{}\" (hybrid search)",
        final_results.len(),
        query
    );
    let structured: Vec<Value> = final_results
        .iter()
        .map(|r| {
            serde_json::json!({
                "docid": format!("#{}", r.docid),
                "file": r.display_path,
                "title": r.title,
                "score": (r.score * 100.0).round() / 100.0
            })
        })
        .collect();

    Ok(ToolResult {
        content: vec![Content::Text { text: summary }],
        structured_content: Some(serde_json::json!({ "results": structured })),
        is_error: None,
    })
}

pub async fn handle_get(db: &Database, args: Value) -> Result<ToolResult> {
    let file = args
        .get("file")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing file"))?;

    let doc = db
        .find_by_docid(file)?
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
            },
        }],
        structured_content: None,
        is_error: None,
    })
}

pub async fn handle_multi_get(db: &Database, args: Value) -> Result<ToolResult> {
    let pattern = args
        .get("pattern")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing pattern"))?;

    let docs = db.fuzzy_find_documents(pattern, 10)?;

    let contents: Vec<Content> = docs
        .into_iter()
        .map(|doc| Content::Resource {
            resource: ResourceContent {
                uri: doc.filepath,
                name: doc.display_path,
                title: Some(doc.title),
                mime_type: "text/markdown".to_string(),
                text: doc.body.unwrap_or_default(),
            },
        })
        .collect();

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

    let mut provider_stats: std::collections::HashMap<String, (usize, usize)> =
        std::collections::HashMap::new();
    for coll in &collections {
        let entry = provider_stats
            .entry(coll.provider_type.clone())
            .or_insert((0, 0));
        entry.0 += 1;
        entry.1 += coll.document_count;
    }

    let mut provider_summary = String::new();
    for (provider, (coll_count, doc_count)) in &provider_stats {
        provider_summary.push_str(&format!(
            "\n  - {}: {} collections, {} documents",
            provider, coll_count, doc_count
        ));
    }

    let summary = format!(
        "Index: {} documents across {} collections\n\
         Embeddings: {}\n\
         Vector index: {}\n\
         \n\
         Providers:{}",
        total_docs,
        collections.len(),
        if needs_embedding > 0 {
            format!("{} documents need embedding", needs_embedding)
        } else {
            "Up to date".to_string()
        },
        if has_vector {
            "Available"
        } else {
            "Not created"
        },
        provider_summary
    );

    let provider_stats_json: Vec<_> = provider_stats
        .iter()
        .map(|(provider, (coll_count, doc_count))| {
            serde_json::json!({
                "provider": provider,
                "collections": coll_count,
                "documents": doc_count
            })
        })
        .collect();

    let structured = serde_json::json!({
        "totalDocuments": total_docs,
        "needsEmbedding": needs_embedding,
        "hasVectorIndex": has_vector,
        "providers": provider_stats_json,
        "collections": collections.iter().map(|c| serde_json::json!({
            "name": c.name,
            "path": c.path,
            "pattern": c.pattern,
            "provider": c.provider_type,
            "documents": c.document_count
        })).collect::<Vec<_>>()
    });

    Ok(ToolResult {
        content: vec![Content::Text { text: summary }],
        structured_content: Some(structured),
        is_error: None,
    })
}

pub async fn handle_collection_add(db: &Database, args: Value) -> Result<ToolResult> {
    let name = args
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing collection name"))?;

    let path = args
        .get("path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing path"))?;

    let pattern = args
        .get("pattern")
        .and_then(|v| v.as_str())
        .unwrap_or("**/*.md");

    let provider = args
        .get("provider")
        .and_then(|v| v.as_str())
        .unwrap_or("file");

    let config = args.get("config").and_then(|v| v.as_str());

    db.add_collection(name, path, pattern, provider, config)?;

    let summary = format!(
        "Added collection '{}' (provider: {}, path: {})",
        name, provider, path
    );

    Ok(ToolResult {
        content: vec![Content::Text { text: summary }],
        structured_content: Some(serde_json::json!({
            "name": name,
            "path": path,
            "pattern": pattern,
            "provider": provider
        })),
        is_error: None,
    })
}

pub async fn handle_collection_remove(db: &Database, args: Value) -> Result<ToolResult> {
    let name = args
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing collection name"))?;

    let removed = db.remove_collection(name)?;

    if removed {
        Ok(ToolResult {
            content: vec![Content::Text {
                text: format!("Removed collection '{}'", name),
            }],
            structured_content: Some(serde_json::json!({
                "name": name,
                "removed": true
            })),
            is_error: None,
        })
    } else {
        Ok(ToolResult {
            content: vec![Content::Text {
                text: format!("Collection '{}' not found", name),
            }],
            structured_content: Some(serde_json::json!({
                "name": name,
                "removed": false
            })),
            is_error: Some(true),
        })
    }
}

pub async fn handle_collection_update(db: &Database, args: Value) -> Result<ToolResult> {
    let name = args
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing collection name"))?;

    let updated = db.reindex_collection(name).await?;

    let summary = format!("Updated collection '{}': {} files changed", name, updated);

    Ok(ToolResult {
        content: vec![Content::Text { text: summary }],
        structured_content: Some(serde_json::json!({
            "name": name,
            "filesUpdated": updated
        })),
        is_error: None,
    })
}
