//! MCP tool definitions and handlers

use crate::protocol::*;
use agentroot_core::{Database, DetailLevel, SearchOptions};
use anyhow::Result;
use serde_json::Value;
use tracing::warn;

// Common detail and session parameters for search tool schemas
fn detail_param() -> Value {
    serde_json::json!({
        "type": "string",
        "enum": ["L0", "L1", "L2"],
        "default": "L1",
        "description": "Context detail level. L0=abstract (~100 tokens), L1=overview (~2K tokens), L2=full content."
    })
}

fn session_id_param() -> Value {
    serde_json::json!({
        "type": "string",
        "description": "Optional session ID for multi-turn context tracking (from session_start)"
    })
}

fn parse_detail(args: &Value) -> DetailLevel {
    DetailLevel::from_str_opt(args.get("detail").and_then(|v| v.as_str()))
}

fn parse_session_id(args: &Value) -> Option<String> {
    args.get("session_id")
        .and_then(|v| v.as_str())
        .map(String::from)
}

fn apply_session_and_project(
    db: &Database,
    results: &mut Vec<agentroot_core::SearchResult>,
    detail: DetailLevel,
    session_id: Option<&str>,
    query: &str,
) {
    // Apply session awareness (demote already-seen results)
    if let Some(sid) = session_id {
        if let Err(e) =
            agentroot_core::search::session_aware::apply_session_awareness(db, results, sid)
        {
            warn!(session_id = sid, error = %e, "session awareness failed");
        }
    }

    // Project results to detail level
    for r in results.iter_mut() {
        r.project(detail);
    }

    // Log session results (best-effort)
    if let Some(sid) = session_id {
        let detail_str = match detail {
            DetailLevel::L0 => "L0",
            DetailLevel::L1 => "L1",
            DetailLevel::L2 => "L2",
        };
        if let Err(e) = agentroot_core::search::session_aware::log_session_results(
            db, sid, query, results, detail_str,
        ) {
            warn!(session_id = sid, error = %e, "session logging failed");
        }
    }
}

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
                },
                "detail": detail_param(),
                "session_id": session_id_param()
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
                },
                "detail": detail_param(),
                "session_id": session_id_param()
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
                },
                "detail": detail_param(),
                "session_id": session_id_param()
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
                },
                "detail": detail_param(),
                "session_id": session_id_param()
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

    let detail = parse_detail(&args);
    let session_id = parse_session_id(&args);

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
        full_content: detail.is_full_content(),
        detail,
        session_id: session_id.clone(),
        metadata_filters: Vec::new(),
        ..Default::default()
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

    apply_session_and_project(db, &mut results, detail, session_id.as_deref(), query);

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

    let detail = parse_detail(&args);
    let session_id = parse_session_id(&args);

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
        full_content: detail.is_full_content(),
        detail,
        session_id: session_id.clone(),
        metadata_filters: Vec::new(),
        ..Default::default()
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

    apply_session_and_project(db, &mut results, detail, session_id.as_deref(), query);

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

    let detail = parse_detail(&args);
    let session_id = parse_session_id(&args);

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
        full_content: detail.is_full_content(),
        detail,
        session_id: session_id.clone(),
        metadata_filters: Vec::new(),
        ..Default::default()
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

    apply_session_and_project(db, &mut final_results, detail, session_id.as_deref(), query);

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

    let detail = parse_detail(&args);
    let session_id = parse_session_id(&args);

    let options = SearchOptions {
        limit: args.get("limit").and_then(|v| v.as_u64()).unwrap_or(20) as usize,
        min_score: args.get("minScore").and_then(|v| v.as_f64()).unwrap_or(0.0),
        collection: args
            .get("collection")
            .and_then(|v| v.as_str())
            .map(String::from),
        provider: None,
        full_content: detail.is_full_content(),
        detail,
        session_id: session_id.clone(),
        metadata_filters: Vec::new(),
        ..Default::default()
    };

    // Use smart_search which handles parsing and fallbacks
    let mut results = agentroot_core::smart_search(db, query, &options).await?;

    apply_session_and_project(db, &mut results, detail, session_id.as_deref(), query);

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
        None => Err(anyhow::anyhow!("No metadata found for document: {}", docid)),
    }
}

// ============================================================================
// Chunk-Level Search Tools
// ============================================================================

pub fn search_chunks_tool_definition() -> ToolDefinition {
    ToolDefinition {
        name: "search_chunks".to_string(),
        description: "Search for specific code chunks (functions, methods, classes) using BM25 full-text search. Returns granular results with line numbers and breadcrumbs.".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Search query (keywords or phrases to find in code chunks)"
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
                "label": {
                    "type": "string",
                    "description": "Filter by chunk label (format: key:value, e.g., 'layer:service')"
                },
                "detail": detail_param(),
                "session_id": session_id_param()
            },
            "required": ["query"]
        }),
    }
}

pub fn get_chunk_tool_definition() -> ToolDefinition {
    ToolDefinition {
        name: "get_chunk".to_string(),
        description: "Retrieve a specific code chunk by its hash, including all metadata and surrounding context.".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "chunk_hash": {
                    "type": "string",
                    "description": "Chunk hash identifier"
                },
                "include_context": {
                    "type": "boolean",
                    "description": "Include surrounding chunks (previous/next) (default: true)",
                    "default": true
                }
            },
            "required": ["chunk_hash"]
        }),
    }
}

pub fn navigate_chunks_tool_definition() -> ToolDefinition {
    ToolDefinition {
        name: "navigate_chunks".to_string(),
        description: "Navigate to previous or next chunk within the same document. Useful for exploring code context.".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "chunk_hash": {
                    "type": "string",
                    "description": "Current chunk hash"
                },
                "direction": {
                    "type": "string",
                    "description": "Navigation direction: 'previous' or 'next'",
                    "enum": ["previous", "next"]
                }
            },
            "required": ["chunk_hash", "direction"]
        }),
    }
}

pub async fn handle_search_chunks(db: &Database, args: Value) -> Result<ToolResult> {
    let query = args
        .get("query")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing query"))?;

    let detail = parse_detail(&args);
    let session_id = parse_session_id(&args);

    let mut options = SearchOptions {
        limit: args.get("limit").and_then(|v| v.as_u64()).unwrap_or(20) as usize,
        min_score: args.get("minScore").and_then(|v| v.as_f64()).unwrap_or(0.0),
        collection: args
            .get("collection")
            .and_then(|v| v.as_str())
            .map(String::from),
        provider: None,
        full_content: detail.is_full_content(),
        detail,
        session_id: session_id.clone(),
        metadata_filters: Vec::new(),
        ..Default::default()
    };

    // Handle label filter
    if let Some(label) = args.get("label").and_then(|v| v.as_str()) {
        options
            .metadata_filters
            .push(("label".to_string(), label.to_string()));
    }

    let mut results = db.search_chunks_bm25(query, &options)?;

    apply_session_and_project(db, &mut results, detail, session_id.as_deref(), query);

    let summary = format!("Found {} chunk(s) for \"{}\"", results.len(), query);
    let structured: Vec<Value> = results
        .iter()
        .map(|r| {
            let mut result_json = serde_json::json!({
                "chunk_hash": r.chunk_hash.as_ref().unwrap_or(&"".to_string()),
                "file": r.display_path,
                "breadcrumb": r.chunk_breadcrumb.as_ref().unwrap_or(&"".to_string()),
                "type": r.chunk_type.as_ref().unwrap_or(&"".to_string()),
                "lines": format!("{}-{}",
                    r.chunk_start_line.unwrap_or(0),
                    r.chunk_end_line.unwrap_or(0)
                ),
                "score": (r.score * 100.0).round() / 100.0
            });

            // Include chunk metadata
            if let Some(summary) = &r.chunk_summary {
                result_json["summary"] = Value::String(summary.clone());
            }
            if let Some(purpose) = &r.chunk_purpose {
                result_json["purpose"] = Value::String(purpose.clone());
            }
            if !r.chunk_concepts.is_empty() {
                result_json["concepts"] = serde_json::to_value(&r.chunk_concepts).unwrap();
            }
            if !r.chunk_labels.is_empty() {
                result_json["labels"] = serde_json::to_value(&r.chunk_labels).unwrap();
            }
            if let Some(content) = &r.body {
                result_json["content"] = Value::String(content.clone());
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

pub async fn handle_get_chunk(db: &Database, args: Value) -> Result<ToolResult> {
    let chunk_hash = args
        .get("chunk_hash")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing chunk_hash"))?;

    let include_context = args
        .get("include_context")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    // Get the chunk
    let chunk = db
        .get_chunk(chunk_hash)?
        .ok_or_else(|| anyhow::anyhow!("Chunk not found: {}", chunk_hash))?;

    let mut result_json = serde_json::json!({
        "chunk_hash": chunk.hash,
        "document_hash": chunk.document_hash,
        "type": chunk.chunk_type.as_ref().unwrap_or(&"".to_string()),
        "breadcrumb": chunk.breadcrumb.as_ref().unwrap_or(&"".to_string()),
        "lines": format!("{}-{}", chunk.start_line, chunk.end_line),
        "language": chunk.language.as_ref().unwrap_or(&"".to_string()),
        "content": chunk.content
    });

    // Add LLM metadata
    if let Some(summary) = &chunk.llm_summary {
        result_json["summary"] = Value::String(summary.clone());
    }
    if let Some(purpose) = &chunk.llm_purpose {
        result_json["purpose"] = Value::String(purpose.clone());
    }
    if !chunk.llm_concepts.is_empty() {
        result_json["concepts"] = serde_json::to_value(&chunk.llm_concepts).unwrap();
    }
    if !chunk.llm_labels.is_empty() {
        result_json["labels"] = serde_json::to_value(&chunk.llm_labels).unwrap();
    }

    // Get surrounding chunks if requested
    let mut context_text = String::new();
    if include_context {
        let (prev, next) = db.get_surrounding_chunks(chunk_hash)?;

        if let Some(prev_chunk) = prev {
            result_json["previous_chunk"] = serde_json::json!({
                "hash": prev_chunk.hash,
                "breadcrumb": prev_chunk.breadcrumb.as_ref().unwrap_or(&"".to_string())
            });
            context_text.push_str(&format!(
                "\n[Previous: {}]",
                prev_chunk.breadcrumb.as_ref().unwrap_or(&"".to_string())
            ));
        }

        if let Some(next_chunk) = next {
            result_json["next_chunk"] = serde_json::json!({
                "hash": next_chunk.hash,
                "breadcrumb": next_chunk.breadcrumb.as_ref().unwrap_or(&"".to_string())
            });
            context_text.push_str(&format!(
                "\n[Next: {}]",
                next_chunk.breadcrumb.as_ref().unwrap_or(&"".to_string())
            ));
        }
    }

    let summary = format!(
        "Chunk: {} ({})\nLines: {}-{}{}",
        chunk.breadcrumb.as_ref().unwrap_or(&"Unknown".to_string()),
        chunk.chunk_type.as_ref().unwrap_or(&"".to_string()),
        chunk.start_line,
        chunk.end_line,
        context_text
    );

    Ok(ToolResult {
        content: vec![Content::Text { text: summary }],
        structured_content: Some(result_json),
        is_error: None,
    })
}

pub async fn handle_navigate_chunks(db: &Database, args: Value) -> Result<ToolResult> {
    let chunk_hash = args
        .get("chunk_hash")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing chunk_hash"))?;

    let direction = args
        .get("direction")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing direction"))?;

    let (prev, next) = db.get_surrounding_chunks(chunk_hash)?;

    let target_chunk = match direction {
        "previous" => prev.ok_or_else(|| anyhow::anyhow!("No previous chunk"))?,
        "next" => next.ok_or_else(|| anyhow::anyhow!("No next chunk"))?,
        _ => return Err(anyhow::anyhow!("Invalid direction: {}", direction)),
    };

    let result_json = serde_json::json!({
        "chunk_hash": target_chunk.hash,
        "document_hash": target_chunk.document_hash,
        "type": target_chunk.chunk_type.as_ref().unwrap_or(&"".to_string()),
        "breadcrumb": target_chunk.breadcrumb.as_ref().unwrap_or(&"".to_string()),
        "lines": format!("{}-{}", target_chunk.start_line, target_chunk.end_line),
        "content": target_chunk.content,
        "summary": target_chunk.llm_summary.as_ref().unwrap_or(&"".to_string()),
        "purpose": target_chunk.llm_purpose.as_ref().unwrap_or(&"".to_string()),
        "concepts": target_chunk.llm_concepts,
        "labels": target_chunk.llm_labels
    });

    let summary = format!(
        "{} chunk: {} ({})\nLines: {}-{}",
        if direction == "previous" {
            "Previous"
        } else {
            "Next"
        },
        target_chunk
            .breadcrumb
            .as_ref()
            .unwrap_or(&"Unknown".to_string()),
        target_chunk.chunk_type.as_ref().unwrap_or(&"".to_string()),
        target_chunk.start_line,
        target_chunk.end_line
    );

    Ok(ToolResult {
        content: vec![Content::Text { text: summary }],
        structured_content: Some(result_json),
        is_error: None,
    })
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

// ============================================================================
// Session Tools
// ============================================================================

pub fn session_start_tool_definition() -> ToolDefinition {
    ToolDefinition {
        name: "session_start".to_string(),
        description: "Start a new search session for multi-turn context tracking. Returns a session_id to pass to subsequent search calls.".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "ttl_seconds": {
                    "type": "integer",
                    "description": "Session time-to-live in seconds (default: 3600)",
                    "default": 3600
                }
            }
        }),
    }
}

pub fn session_context_tool_definition() -> ToolDefinition {
    ToolDefinition {
        name: "session_context".to_string(),
        description: "Get or set session context key-value pairs, and view past queries."
            .to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "session_id": {
                    "type": "string",
                    "description": "Session ID from session_start"
                },
                "action": {
                    "type": "string",
                    "enum": ["get", "set"],
                    "description": "Action: 'get' returns context and query history, 'set' updates a context key"
                },
                "key": {
                    "type": "string",
                    "description": "Context key (required for 'set' action)"
                },
                "value": {
                    "type": "string",
                    "description": "Context value (required for 'set' action)"
                }
            },
            "required": ["session_id", "action"]
        }),
    }
}

pub fn session_end_tool_definition() -> ToolDefinition {
    ToolDefinition {
        name: "session_end".to_string(),
        description: "End a search session and clean up resources.".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "session_id": {
                    "type": "string",
                    "description": "Session ID to end"
                }
            },
            "required": ["session_id"]
        }),
    }
}

pub async fn handle_session_start(db: &Database, args: Value) -> Result<ToolResult> {
    let ttl = args
        .get("ttl_seconds")
        .and_then(|v| v.as_i64())
        .or(Some(3600));

    let session_id = db.create_session(ttl)?;

    Ok(ToolResult {
        content: vec![Content::Text {
            text: format!("Session started: {}", session_id),
        }],
        structured_content: Some(serde_json::json!({
            "session_id": session_id,
            "ttl_seconds": ttl
        })),
        is_error: None,
    })
}

pub async fn handle_session_context(db: &Database, args: Value) -> Result<ToolResult> {
    let session_id = args
        .get("session_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing session_id"))?;

    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing action"))?;

    match action {
        "get" => {
            let session = db
                .get_session(session_id)?
                .ok_or_else(|| anyhow::anyhow!("Session not found: {}", session_id))?;
            let queries = db.get_session_queries(session_id)?;
            let seen = db.get_seen_hashes(session_id)?;

            let queries_json: Vec<Value> = queries
                .iter()
                .map(|q| {
                    serde_json::json!({
                        "query": q.query,
                        "result_count": q.result_count,
                        "created_at": q.created_at
                    })
                })
                .collect();

            Ok(ToolResult {
                content: vec![Content::Text {
                    text: format!(
                        "Session {}: {} queries, {} seen documents",
                        session_id,
                        queries.len(),
                        seen.len()
                    ),
                }],
                structured_content: Some(serde_json::json!({
                    "session_id": session_id,
                    "created_at": session.created_at,
                    "last_active_at": session.last_active_at,
                    "ttl_seconds": session.ttl_seconds,
                    "context": session.context,
                    "queries": queries_json,
                    "seen_count": seen.len()
                })),
                is_error: None,
            })
        }
        "set" => {
            let key = args
                .get("key")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Missing key for 'set' action"))?;
            let value = args
                .get("value")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Missing value for 'set' action"))?;

            db.set_session_context(session_id, key, value)?;

            Ok(ToolResult {
                content: vec![Content::Text {
                    text: format!("Set {}={} on session {}", key, value, session_id),
                }],
                structured_content: Some(serde_json::json!({
                    "session_id": session_id,
                    "key": key,
                    "value": value
                })),
                is_error: None,
            })
        }
        _ => Err(anyhow::anyhow!(
            "Invalid action: {}. Use 'get' or 'set'",
            action
        )),
    }
}

pub async fn handle_session_end(db: &Database, args: Value) -> Result<ToolResult> {
    let session_id = args
        .get("session_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing session_id"))?;

    db.delete_session(session_id)?;

    Ok(ToolResult {
        content: vec![Content::Text {
            text: format!("Session ended: {}", session_id),
        }],
        structured_content: Some(serde_json::json!({
            "session_id": session_id,
            "ended": true
        })),
        is_error: None,
    })
}

// ============================================================================
// Directory Browsing Tools
// ============================================================================

pub fn browse_directory_tool_definition() -> ToolDefinition {
    ToolDefinition {
        name: "browse_directory".to_string(),
        description: "Browse the directory structure of indexed collections. Shows files, subdirectories, and metadata for a given path.".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "collection": {
                    "type": "string",
                    "description": "Collection name to browse"
                },
                "path": {
                    "type": "string",
                    "description": "Directory path within the collection (empty for root)"
                },
                "max_depth": {
                    "type": "integer",
                    "description": "Maximum depth of subdirectories to return (default: 2)",
                    "default": 2
                }
            },
            "required": ["collection"]
        }),
    }
}

pub fn search_directories_tool_definition() -> ToolDefinition {
    ToolDefinition {
        name: "search_directories".to_string(),
        description: "Search directories by name, concepts, or content using full-text search."
            .to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Search query for directories"
                },
                "collection": {
                    "type": "string",
                    "description": "Filter by collection name"
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum results (default: 10)",
                    "default": 10
                }
            },
            "required": ["query"]
        }),
    }
}

pub async fn handle_browse_directory(db: &Database, args: Value) -> Result<ToolResult> {
    let collection = args
        .get("collection")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing collection"))?;

    let path = args.get("path").and_then(|v| v.as_str());
    let max_depth = args.get("max_depth").and_then(|v| v.as_u64()).unwrap_or(2) as usize;

    let dirs = db.list_directories(collection, path, Some(max_depth))?;

    let summary = format!(
        "Found {} directories in {}/{}",
        dirs.len(),
        collection,
        path.unwrap_or("")
    );

    let structured: Vec<Value> = dirs
        .iter()
        .map(|d| {
            let mut dir_json = serde_json::json!({
                "path": d.path,
                "depth": d.depth,
                "file_count": d.file_count,
                "child_dir_count": d.child_dir_count
            });
            if let Some(lang) = &d.dominant_language {
                dir_json["language"] = Value::String(lang.clone());
            }
            if let Some(cat) = &d.dominant_category {
                dir_json["category"] = Value::String(cat.clone());
            }
            if let Some(summary) = &d.summary {
                dir_json["summary"] = Value::String(summary.clone());
            }
            if !d.concepts.is_empty() {
                dir_json["concepts"] = serde_json::to_value(&d.concepts).unwrap();
            }
            dir_json
        })
        .collect();

    Ok(ToolResult {
        content: vec![Content::Text { text: summary }],
        structured_content: Some(serde_json::json!({ "directories": structured })),
        is_error: None,
    })
}

pub async fn handle_search_directories(db: &Database, args: Value) -> Result<ToolResult> {
    let query = args
        .get("query")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing query"))?;

    let collection = args.get("collection").and_then(|v| v.as_str());
    let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(10) as usize;

    let dirs = db.search_directories_fts(query, collection, limit)?;

    let summary = format!("Found {} directories matching \"{}\"", dirs.len(), query);

    let structured: Vec<Value> = dirs
        .iter()
        .map(|d| {
            let mut dir_json = serde_json::json!({
                "path": d.path,
                "collection": d.collection,
                "file_count": d.file_count,
                "child_dir_count": d.child_dir_count
            });
            if let Some(lang) = &d.dominant_language {
                dir_json["language"] = Value::String(lang.clone());
            }
            if let Some(cat) = &d.dominant_category {
                dir_json["category"] = Value::String(cat.clone());
            }
            if let Some(summary) = &d.summary {
                dir_json["summary"] = Value::String(summary.clone());
            }
            if !d.concepts.is_empty() {
                dir_json["concepts"] = serde_json::to_value(&d.concepts).unwrap();
            }
            dir_json
        })
        .collect();

    Ok(ToolResult {
        content: vec![Content::Text { text: summary }],
        structured_content: Some(serde_json::json!({ "directories": structured })),
        is_error: None,
    })
}

// ============================================================================
// Batch & Explore Tools
// ============================================================================

pub fn batch_search_tool_definition() -> ToolDefinition {
    ToolDefinition {
        name: "batch_search".to_string(),
        description: "Execute multiple search queries in a single call. Each query runs independently with its own parameters.".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "queries": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "query": {
                                "type": "string",
                                "description": "Search query"
                            },
                            "limit": {
                                "type": "integer",
                                "description": "Maximum results for this query (default: 5)",
                                "default": 5
                            },
                            "collection": {
                                "type": "string",
                                "description": "Filter by collection"
                            }
                        },
                        "required": ["query"]
                    },
                    "description": "Array of search queries to execute"
                },
                "detail": detail_param(),
                "session_id": session_id_param()
            },
            "required": ["queries"]
        }),
    }
}

pub fn explore_tool_definition() -> ToolDefinition {
    ToolDefinition {
        name: "explore".to_string(),
        description: "Explore the knowledge base starting from a search query. Returns results plus suggestions for related directories, concepts, and follow-up queries.".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Search query to explore from"
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum results (default: 10)",
                    "default": 10
                },
                "collection": {
                    "type": "string",
                    "description": "Filter by collection"
                },
                "detail": detail_param(),
                "session_id": session_id_param()
            },
            "required": ["query"]
        }),
    }
}

pub async fn handle_batch_search(db: &Database, args: Value) -> Result<ToolResult> {
    let queries = args
        .get("queries")
        .and_then(|v| v.as_array())
        .ok_or_else(|| anyhow::anyhow!("Missing queries array"))?;

    let detail = parse_detail(&args);
    let session_id = parse_session_id(&args);

    let mut all_results: Vec<Value> = Vec::new();

    for query_obj in queries {
        let query = query_obj
            .get("query")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if query.is_empty() {
            continue;
        }

        let limit = query_obj.get("limit").and_then(|v| v.as_u64()).unwrap_or(5) as usize;
        let collection = query_obj
            .get("collection")
            .and_then(|v| v.as_str())
            .map(String::from);

        let options = SearchOptions {
            limit,
            min_score: 0.0,
            collection,
            provider: None,
            full_content: detail.is_full_content(),
            detail,
            session_id: session_id.clone(),
            metadata_filters: Vec::new(),
            ..Default::default()
        };

        let mut results = db.search_fts(query, &options)?;
        apply_session_and_project(db, &mut results, detail, session_id.as_deref(), query);

        let results_json: Vec<Value> = results
            .iter()
            .map(|r| {
                let mut rj = serde_json::json!({
                    "docid": format!("#{}", r.docid),
                    "file": r.display_path,
                    "title": r.title,
                    "score": (r.score * 100.0).round() / 100.0
                });
                if let Some(s) = &r.llm_summary {
                    rj["summary"] = Value::String(s.clone());
                }
                rj
            })
            .collect();

        all_results.push(serde_json::json!({
            "query": query,
            "count": results_json.len(),
            "results": results_json
        }));
    }

    let total: usize = all_results
        .iter()
        .filter_map(|r| r.get("count").and_then(|c| c.as_u64()))
        .sum::<u64>() as usize;

    Ok(ToolResult {
        content: vec![Content::Text {
            text: format!(
                "Batch search: {} queries, {} total results",
                all_results.len(),
                total
            ),
        }],
        structured_content: Some(serde_json::json!({ "batches": all_results })),
        is_error: None,
    })
}

pub async fn handle_explore(db: &Database, args: Value) -> Result<ToolResult> {
    let query = args
        .get("query")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing query"))?;

    let detail = parse_detail(&args);
    let session_id = parse_session_id(&args);

    let options = SearchOptions {
        limit: args.get("limit").and_then(|v| v.as_u64()).unwrap_or(10) as usize,
        min_score: 0.0,
        collection: args
            .get("collection")
            .and_then(|v| v.as_str())
            .map(String::from),
        provider: None,
        full_content: detail.is_full_content(),
        detail,
        session_id: session_id.clone(),
        metadata_filters: Vec::new(),
        ..Default::default()
    };

    let mut results = db.search_fts(query, &options)?;
    apply_session_and_project(db, &mut results, detail, session_id.as_deref(), query);

    let suggestions = agentroot_core::search::suggestions::compute_suggestions(
        db,
        &results,
        query,
        session_id.as_deref(),
    )?;

    let results_json: Vec<Value> = results
        .iter()
        .map(|r| {
            let mut rj = serde_json::json!({
                "docid": format!("#{}", r.docid),
                "file": r.display_path,
                "title": r.title,
                "score": (r.score * 100.0).round() / 100.0
            });
            if let Some(s) = &r.llm_summary {
                rj["summary"] = Value::String(s.clone());
            }
            if let Some(cat) = &r.llm_category {
                rj["category"] = Value::String(cat.clone());
            }
            rj
        })
        .collect();

    let mut summary_parts = vec![format!("Found {} results for \"{}\"", results.len(), query)];
    if !suggestions.related_directories.is_empty() {
        summary_parts.push(format!(
            "Related dirs: {}",
            suggestions.related_directories.join(", ")
        ));
    }
    if !suggestions.refinement_queries.is_empty() {
        summary_parts.push(format!(
            "Try also: {}",
            suggestions.refinement_queries.join(", ")
        ));
    }
    if suggestions.unseen_related > 0 {
        summary_parts.push(format!(
            "{} unseen related documents",
            suggestions.unseen_related
        ));
    }

    Ok(ToolResult {
        content: vec![Content::Text {
            text: summary_parts.join("\n"),
        }],
        structured_content: Some(serde_json::json!({
            "results": results_json,
            "suggestions": {
                "related_directories": suggestions.related_directories,
                "related_concepts": suggestions.related_concepts,
                "refinement_queries": suggestions.refinement_queries,
                "unseen_related": suggestions.unseen_related
            }
        })),
        is_error: None,
    })
}
