//! MCP server implementation

use crate::protocol::*;
use crate::tools;
use agentroot_core::Database;
use anyhow::Result;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};

pub struct McpServer<'a> {
    db: &'a Database,
}

impl<'a> McpServer<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub async fn run(&self) -> Result<()> {
        let stdin = tokio::io::stdin();
        let stdout = tokio::io::stdout();

        let mut reader = BufReader::new(stdin);
        let mut writer = BufWriter::new(stdout);
        let mut line = String::new();

        loop {
            line.clear();
            let bytes_read = reader.read_line(&mut line).await?;

            if bytes_read == 0 {
                break;
            }

            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            let request: JsonRpcRequest = match serde_json::from_str(trimmed) {
                Ok(r) => r,
                Err(e) => {
                    let response =
                        JsonRpcResponse::error(None, -32700, &format!("Parse error: {}", e));
                    self.write_response(&mut writer, &response).await?;
                    continue;
                }
            };

            let response = self.handle_request(&request).await;
            self.write_response(&mut writer, &response).await?;
        }

        Ok(())
    }

    async fn write_response<W: AsyncWriteExt + Unpin>(
        &self,
        writer: &mut W,
        response: &JsonRpcResponse,
    ) -> Result<()> {
        let json = serde_json::to_string(response)?;
        writer.write_all(json.as_bytes()).await?;
        writer.write_all(b"\n").await?;
        writer.flush().await?;
        Ok(())
    }

    async fn handle_request(&self, request: &JsonRpcRequest) -> JsonRpcResponse {
        match request.method.as_str() {
            "initialize" => self.handle_initialize(request),
            "tools/list" => self.handle_tools_list(request),
            "tools/call" => self.handle_tools_call(request).await,
            "resources/list" => self.handle_resources_list(request),
            "prompts/list" => self.handle_prompts_list(request),
            _ => JsonRpcResponse::error(
                request.id.clone(),
                -32601,
                &format!("Method not found: {}", request.method),
            ),
        }
    }

    fn handle_initialize(&self, request: &JsonRpcRequest) -> JsonRpcResponse {
        let result = serde_json::json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": {},
                "resources": { "subscribe": false },
                "prompts": {}
            },
            "serverInfo": {
                "name": "agentroot",
                "version": env!("CARGO_PKG_VERSION")
            }
        });
        JsonRpcResponse::success(request.id.clone(), result)
    }

    fn handle_tools_list(&self, request: &JsonRpcRequest) -> JsonRpcResponse {
        let tools = vec![
            tools::search_tool_definition(),
            tools::vsearch_tool_definition(),
            tools::query_tool_definition(),
            tools::get_tool_definition(),
            tools::multi_get_tool_definition(),
            tools::status_tool_definition(),
            tools::collection_add_tool_definition(),
            tools::collection_remove_tool_definition(),
            tools::collection_update_tool_definition(),
        ];

        JsonRpcResponse::success(request.id.clone(), serde_json::json!({ "tools": tools }))
    }

    async fn handle_tools_call(&self, request: &JsonRpcRequest) -> JsonRpcResponse {
        let name = request
            .params
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let arguments = request
            .params
            .get("arguments")
            .cloned()
            .unwrap_or(serde_json::json!({}));

        let result = match name {
            "search" => tools::handle_search(self.db, arguments).await,
            "vsearch" => tools::handle_vsearch(self.db, arguments).await,
            "query" => tools::handle_query(self.db, arguments).await,
            "get" => tools::handle_get(self.db, arguments).await,
            "multi_get" => tools::handle_multi_get(self.db, arguments).await,
            "status" => tools::handle_status(self.db).await,
            "collection_add" => tools::handle_collection_add(self.db, arguments).await,
            "collection_remove" => tools::handle_collection_remove(self.db, arguments).await,
            "collection_update" => tools::handle_collection_update(self.db, arguments).await,
            _ => Err(anyhow::anyhow!("Unknown tool: {}", name)),
        };

        match result {
            Ok(tool_result) => JsonRpcResponse::success(
                request.id.clone(),
                serde_json::to_value(tool_result).unwrap(),
            ),
            Err(e) => {
                let error_result = ToolResult {
                    content: vec![Content::Text {
                        text: format!("Error: {}", e),
                    }],
                    structured_content: None,
                    is_error: Some(true),
                };
                JsonRpcResponse::success(
                    request.id.clone(),
                    serde_json::to_value(error_result).unwrap(),
                )
            }
        }
    }

    fn handle_resources_list(&self, request: &JsonRpcRequest) -> JsonRpcResponse {
        JsonRpcResponse::success(request.id.clone(), serde_json::json!({ "resources": [] }))
    }

    fn handle_prompts_list(&self, request: &JsonRpcRequest) -> JsonRpcResponse {
        let prompts = vec![serde_json::json!({
            "name": "query",
            "title": "Agentroot Query Guide",
            "description": "How to effectively search your knowledge base"
        })];
        JsonRpcResponse::success(
            request.id.clone(),
            serde_json::json!({ "prompts": prompts }),
        )
    }
}

pub async fn start_server(db: &Database) -> Result<()> {
    let server = McpServer::new(db);
    server.run().await
}
