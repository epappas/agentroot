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
                },
                "category": {
                    "type": "string",
                    "description": "Filter by document category (tutorial, reference, code, config, etc.)"
                },
                "difficulty": {
                    "type": "string",
                    "description": "Filter by difficulty level (beginner, intermediate, advanced)"
                },
                "concept": {
                    "type": "string",
                    "description": "Filter by concept/topic"
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
                },
                "category": {
                    "type": "string",
                    "description": "Filter by document category (tutorial, reference, code, config, etc.)"
                },
                "difficulty": {
                    "type": "string",
                    "description": "Filter by difficulty level (beginner, intermediate, advanced)"
                },
                "concept": {
                    "type": "string",
                    "description": "Filter by concept/topic"
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
                },
                "category": {
                    "type": "string",
                    "description": "Filter by document category (tutorial, reference, code, config, etc.)"
                },
                "difficulty": {
                    "type": "string",
                    "description": "Filter by difficulty level (beginner, intermediate, advanced)"
                },
                "concept": {
                    "type": "string",
                    "description": "Filter by concept/topic"
                }
            },
            "required": ["query"]
        }),
    }
}

pub fn smart_search_tool_definition() -> ToolDefinition {
    ToolDefinition {
        name: "smart_search".to_string(),
        description: "Intelligent natural language search with automatic query understanding and filtering. Understands temporal filters like 'last hour', metadata filters like 'by Alice', and automatically falls back to BM25 if models are unavailable.".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Natural language search query (e.g., 'files edited last hour', 'rust tutorials by Alice')"
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

    let mut results = db.search_fts(query, &options)?;

    // Apply metadata filters
    let category_filter = args.get("category").and_then(|v| v.as_str());
    let difficulty_filter = args.get("difficulty").and_then(|v| v.as_str());
    let concept_filter = args.get("concept").and_then(|v| v.as_str());

    if category_filter.is_some() || difficulty_filter.is_some() || concept_filter.is_some() {
        results.retain(|r| {
            let matches_category = category_filter.map_or(true, |cat| {
                r.llm_category
                    .as_ref()
                    .map_or(false, |c| c.to_lowercase().contains(&cat.to_lowercase()))
            });
            let matches_difficulty = difficulty_filter.map_or(true, |diff| {
                r.llm_difficulty
                    .as_ref()
                    .map_or(false, |d| d.to_lowercase() == diff.to_lowercase())
            });
            let matches_concept = concept_filter.map_or(true, |concept| {
                r.llm_keywords.as_ref().map_or(false, |kws| {
                    kws.iter()
                        .any(|kw| kw.to_lowercase().contains(&concept.to_lowercase()))
                })
            });
            matches_category && matches_difficulty && matches_concept
        });
    }

    let summary = format!("Found {} results for \"{}\"", results.len(), query);
    let structured: Vec<Value> = results
        .iter()
        .map(|r| {
            let mut result_json = serde_json::json!({
                "docid": format!("#{}", r.docid),
                "file": r.display_path,
                "title": r.title,
                "score": (r.score * 100.0).round() / 100.0
            });

            // Include LLM metadata if available
            if let Some(summary) = &r.llm_summary {
                result_json["summary"] = Value::String(summary.clone());
            }
            if let Some(category) = &r.llm_category {
                result_json["category"] = Value::String(category.clone());
            }
            if let Some(difficulty) = &r.llm_difficulty {
                result_json["difficulty"] = Value::String(difficulty.clone());
            }
            if let Some(keywords) = &r.llm_keywords {
                result_json["keywords"] = serde_json::to_value(keywords).unwrap();
            }

            // Include user metadata if available
            if let Some(user_meta) = &r.user_metadata {
                if let Ok(json_str) = user_meta.to_json() {
                    if let Ok(parsed) = serde_json::from_str::<Value>(&json_str) {
                        result_json["userMetadata"] = parsed;
                    }
                }
            }

            result_json
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

    // Try HTTP embedder first, fallback to local
    let embedder: Box<dyn agentroot_core::Embedder> = match agentroot_core::HttpEmbedder::from_env()
    {
        Ok(http) => Box::new(http),
        Err(_) => {
            return Ok(ToolResult {
                    content: vec![Content::Text {
                        text: "No embedding service configured. Set AGENTROOT_EMBEDDING_URL, \
                              AGENTROOT_EMBEDDING_MODEL, and AGENTROOT_EMBEDDING_DIMS environment variables. \
                              See VLLM_SETUP.md for details."
                            .to_string(),
                    }],
                    structured_content: None,
                    is_error: Some(true),
                });
        }
    };

    let mut results = db.search_vec(query, embedder.as_ref(), &options).await?;

    // Apply metadata filters
    let category_filter = args.get("category").and_then(|v| v.as_str());
    let difficulty_filter = args.get("difficulty").and_then(|v| v.as_str());
    let concept_filter = args.get("concept").and_then(|v| v.as_str());

    if category_filter.is_some() || difficulty_filter.is_some() || concept_filter.is_some() {
        results.retain(|r| {
            let matches_category = category_filter.map_or(true, |cat| {
                r.llm_category
                    .as_ref()
                    .map_or(false, |c| c.to_lowercase().contains(&cat.to_lowercase()))
            });
            let matches_difficulty = difficulty_filter.map_or(true, |diff| {
                r.llm_difficulty
                    .as_ref()
                    .map_or(false, |d| d.to_lowercase() == diff.to_lowercase())
            });
            let matches_concept = concept_filter.map_or(true, |concept| {
                r.llm_keywords.as_ref().map_or(false, |kws| {
                    kws.iter()
                        .any(|kw| kw.to_lowercase().contains(&concept.to_lowercase()))
                })
            });
            matches_category && matches_difficulty && matches_concept
        });
    }

    let summary = format!("Found {} results for \"{}\"", results.len(), query);
    let structured: Vec<Value> = results
        .iter()
        .map(|r| {
            let mut result_json = serde_json::json!({
                "docid": format!("#{}", r.docid),
                "file": r.display_path,
                "title": r.title,
                "score": (r.score * 100.0).round() / 100.0
            });

            // Include LLM metadata if available
            if let Some(summary) = &r.llm_summary {
                result_json["summary"] = Value::String(summary.clone());
            }
            if let Some(category) = &r.llm_category {
                result_json["category"] = Value::String(category.clone());
            }
            if let Some(difficulty) = &r.llm_difficulty {
                result_json["difficulty"] = Value::String(difficulty.clone());
            }
            if let Some(keywords) = &r.llm_keywords {
                result_json["keywords"] = serde_json::to_value(keywords).unwrap();
            }

            // Include user metadata if available
            if let Some(user_meta) = &r.user_metadata {
                if let Ok(json_str) = user_meta.to_json() {
                    if let Ok(parsed) = serde_json::from_str::<Value>(&json_str) {
                        result_json["userMetadata"] = parsed;
                    }
                }
            }

            result_json
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

    // Try HTTP embedder, fallback to BM25-only if not configured
    let embedder: Box<dyn agentroot_core::Embedder> = match agentroot_core::HttpEmbedder::from_env()
    {
        Ok(http) => Box::new(http),
        Err(_) => {
            // No HTTP embedder configured, fall back to BM25-only search
            return handle_search(db, args).await;
        }
    };

    let bm25_results = db.search_fts(query, &options)?;
    let vec_results = db.search_vec(query, embedder.as_ref(), &options).await?;

    let fused_results = agentroot_core::search::rrf_fusion(&bm25_results, &vec_results);

    let mut final_results: Vec<_> = fused_results
        .into_iter()
        .filter(|r| r.score >= options.min_score)
        .take(options.limit)
        .collect();

    // Apply metadata filters
    let category_filter = args.get("category").and_then(|v| v.as_str());
    let difficulty_filter = args.get("difficulty").and_then(|v| v.as_str());
    let concept_filter = args.get("concept").and_then(|v| v.as_str());

    if category_filter.is_some() || difficulty_filter.is_some() || concept_filter.is_some() {
        final_results.retain(|r| {
            let matches_category = category_filter.map_or(true, |cat| {
                r.llm_category
                    .as_ref()
                    .map_or(false, |c| c.to_lowercase().contains(&cat.to_lowercase()))
            });
            let matches_difficulty = difficulty_filter.map_or(true, |diff| {
                r.llm_difficulty
                    .as_ref()
                    .map_or(false, |d| d.to_lowercase() == diff.to_lowercase())
            });
            let matches_concept = concept_filter.map_or(true, |concept| {
                r.llm_keywords.as_ref().map_or(false, |kws| {
                    kws.iter()
                        .any(|kw| kw.to_lowercase().contains(&concept.to_lowercase()))
                })
            });
            matches_category && matches_difficulty && matches_concept
        });
    }

    let summary = format!(
        "Found {} results for \"{}\" (hybrid search)",
        final_results.len(),
        query
    );
    let structured: Vec<Value> = final_results
        .iter()
        .map(|r| {
            let mut result_json = serde_json::json!({
                "docid": format!("#{}", r.docid),
                "file": r.display_path,
                "title": r.title,
                "score": (r.score * 100.0).round() / 100.0
            });

            // Include LLM metadata if available
            if let Some(summary) = &r.llm_summary {
                result_json["summary"] = Value::String(summary.clone());
            }
            if let Some(category) = &r.llm_category {
                result_json["category"] = Value::String(category.clone());
            }
            if let Some(difficulty) = &r.llm_difficulty {
                result_json["difficulty"] = Value::String(difficulty.clone());
            }
            if let Some(keywords) = &r.llm_keywords {
                result_json["keywords"] = serde_json::to_value(keywords).unwrap();
            }

            // Include user metadata if available
            if let Some(user_meta) = &r.user_metadata {
                if let Ok(json_str) = user_meta.to_json() {
                    if let Ok(parsed) = serde_json::from_str::<Value>(&json_str) {
                        result_json["userMetadata"] = parsed;
                    }
                }
            }

            result_json
        })
        .collect();

    Ok(ToolResult {
        content: vec![Content::Text { text: summary }],
        structured_content: Some(serde_json::json!({ "results": structured })),
        is_error: None,
    })
}

pub async fn handle_smart_search(db: &Database, args: Value) -> Result<ToolResult> {
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
        provider: None,
        full_content: false,
    };

    // Use smart_search which handles parsing and fallbacks
    let results = agentroot_core::smart_search(db, query, &options).await?;

    let summary = format!(
        "Found {} results for \"{}\" (smart search)",
        results.len(),
        query
    );
    let structured: Vec<Value> = results
        .iter()
        .map(|r| {
            let mut result_json = serde_json::json!({
                "docid": format!("#{}", r.docid),
                "file": r.display_path,
                "title": r.title,
                "score": (r.score * 100.0).round() / 100.0
            });

            // Include LLM metadata if available
            if let Some(summary) = &r.llm_summary {
                result_json["summary"] = Value::String(summary.clone());
            }
            if let Some(category) = &r.llm_category {
                result_json["category"] = Value::String(category.clone());
            }
            if let Some(difficulty) = &r.llm_difficulty {
                result_json["difficulty"] = Value::String(difficulty.clone());
            }
            if let Some(keywords) = &r.llm_keywords {
                result_json["keywords"] = serde_json::to_value(keywords).unwrap();
            }

            // Include user metadata if available
            if let Some(user_meta) = &r.user_metadata {
                if let Ok(json_str) = user_meta.to_json() {
                    if let Ok(parsed) = serde_json::from_str::<Value>(&json_str) {
                        result_json["userMetadata"] = parsed;
                    }
                }
            }

            result_json
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

pub fn metadata_add_tool_definition() -> ToolDefinition {
    ToolDefinition {
        name: "metadata_add".to_string(),
        description: "Add custom user metadata to a document".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "docid": {
                    "type": "string",
                    "description": "Document ID (#abc123) or path"
                },
                "metadata": {
                    "type": "object",
                    "description": "Metadata fields as key-value pairs. Values can be strings, numbers, booleans, or arrays",
                    "additionalProperties": true
                }
            },
            "required": ["docid", "metadata"]
        }),
    }
}

pub fn metadata_get_tool_definition() -> ToolDefinition {
    ToolDefinition {
        name: "metadata_get".to_string(),
        description: "Get custom user metadata from a document".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "docid": {
                    "type": "string",
                    "description": "Document ID (#abc123) or path"
                }
            },
            "required": ["docid"]
        }),
    }
}

pub fn metadata_query_tool_definition() -> ToolDefinition {
    ToolDefinition {
        name: "metadata_query".to_string(),
        description: "Query documents by custom user metadata".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "field": {
                    "type": "string",
                    "description": "Metadata field name to query"
                },
                "operator": {
                    "type": "string",
                    "enum": ["eq", "contains", "gt", "lt", "has", "exists"],
                    "description": "Comparison operator"
                },
                "value": {
                    "type": "string",
                    "description": "Value to compare against (not needed for 'exists' operator)"
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum results (default: 20)",
                    "default": 20
                }
            },
            "required": ["field", "operator"]
        }),
    }
}

pub async fn handle_metadata_add(db: &Database, args: Value) -> Result<ToolResult> {
    use agentroot_core::MetadataBuilder;

    let docid = args
        .get("docid")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing docid"))?;

    let metadata_obj = args
        .get("metadata")
        .and_then(|v| v.as_object())
        .ok_or_else(|| anyhow::anyhow!("Missing or invalid metadata"))?;

    let mut builder = MetadataBuilder::new();

    for (key, value) in metadata_obj {
        match value {
            Value::String(s) => {
                builder = builder.text(key, s.clone());
            }
            Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    builder = builder.integer(key, i);
                } else if let Some(f) = n.as_f64() {
                    builder = builder.float(key, f);
                }
            }
            Value::Bool(b) => {
                builder = builder.boolean(key, *b);
            }
            Value::Array(arr) => {
                let tags: Vec<String> = arr
                    .iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect();
                builder = builder.tags(key, tags);
            }
            _ => {}
        }
    }

    let metadata = builder.build();
    db.add_metadata(docid, &metadata)?;

    let summary = format!("Added metadata to document: {}", docid);

    Ok(ToolResult {
        content: vec![Content::Text { text: summary }],
        structured_content: Some(serde_json::json!({
            "docid": docid,
            "added": true
        })),
        is_error: None,
    })
}

pub async fn handle_metadata_get(db: &Database, args: Value) -> Result<ToolResult> {
    let docid = args
        .get("docid")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing docid"))?;

    match db.get_metadata(docid)? {
        Some(metadata) => {
            let json = metadata.to_json()?;
            let parsed: serde_json::Value = serde_json::from_str(&json)?;

            Ok(ToolResult {
                content: vec![Content::Text {
                    text: format!("User metadata for {}: {}", docid, json),
                }],
                structured_content: Some(serde_json::json!({
                    "docid": docid,
                    "metadata": parsed
                })),
                is_error: None,
            })
        }
        None => Ok(ToolResult {
            content: vec![Content::Text {
                text: format!("No user metadata found for document: {}", docid),
            }],
            structured_content: Some(serde_json::json!({
                "docid": docid,
                "metadata": null
            })),
            is_error: None,
        }),
    }
}

pub async fn handle_metadata_query(db: &Database, args: Value) -> Result<ToolResult> {
    use agentroot_core::MetadataFilter;

    let field = args
        .get("field")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing field"))?
        .to_string();

    let operator = args
        .get("operator")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing operator"))?;

    let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(20) as usize;

    let filter = match operator {
        "exists" => MetadataFilter::Exists(field),
        _ => {
            let value = args
                .get("value")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Missing value for operator"))?;

            match operator {
                "eq" => MetadataFilter::TextEq(field, value.to_string()),
                "contains" => MetadataFilter::TextContains(field, value.to_string()),
                "gt" => {
                    if let Ok(num) = value.parse::<i64>() {
                        MetadataFilter::IntegerGt(field, num)
                    } else if let Ok(num) = value.parse::<f64>() {
                        MetadataFilter::FloatGt(field, num)
                    } else {
                        return Err(anyhow::anyhow!("Invalid numeric value for gt"));
                    }
                }
                "lt" => {
                    if let Ok(num) = value.parse::<i64>() {
                        MetadataFilter::IntegerLt(field, num)
                    } else if let Ok(num) = value.parse::<f64>() {
                        MetadataFilter::FloatLt(field, num)
                    } else {
                        return Err(anyhow::anyhow!("Invalid numeric value for lt"));
                    }
                }
                "has" => MetadataFilter::TagsContain(field, value.to_string()),
                _ => return Err(anyhow::anyhow!("Invalid operator")),
            }
        }
    };

    let docids = db.find_by_metadata(&filter, limit)?;

    let summary = if docids.is_empty() {
        "No documents found matching filter".to_string()
    } else {
        format!("Found {} document(s) matching filter", docids.len())
    };

    Ok(ToolResult {
        content: vec![Content::Text {
            text: format!("{}\n{}", summary, docids.join("\n")),
        }],
        structured_content: Some(serde_json::json!({
            "count": docids.len(),
            "documents": docids
        })),
        is_error: None,
    })
}
