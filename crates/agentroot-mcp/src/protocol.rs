//! MCP protocol types

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// JSON-RPC 2.0 Request
#[derive(Debug, Deserialize)]
pub struct JsonRpcRequest {
    #[allow(dead_code)]
    pub jsonrpc: String,
    pub id: Option<Value>,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

/// JSON-RPC 2.0 Response
#[derive(Debug, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl JsonRpcResponse {
    pub fn success(id: Option<Value>, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn error(id: Option<Value>, code: i32, message: &str) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(JsonRpcError {
                code,
                message: message.to_string(),
                data: None,
            }),
        }
    }
}

/// MCP Tool Definition
#[derive(Debug, Serialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
}

/// MCP Content Types
#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum Content {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "resource")]
    Resource { resource: ResourceContent },
}

#[derive(Debug, Serialize)]
pub struct ResourceContent {
    pub uri: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(rename = "mimeType")]
    pub mime_type: String,
    pub text: String,
}

/// Tool call result
#[derive(Debug, Serialize)]
pub struct ToolResult {
    pub content: Vec<Content>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "structuredContent")]
    pub structured_content: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "isError")]
    pub is_error: Option<bool>,
}
