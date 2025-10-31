use serde_json::json;
use tracing::{debug, error};

use super::{
    CallToolParams, InitializeResult, JsonRpcError, JsonRpcRequest, JsonRpcResponse,
    ListToolsResult, ServerCapabilities, ServerInfo, Tool, ToolContent, ToolResult,
    ToolsCapability,
};

pub struct RequestHandler {
    tool_executor: crate::starlark::engine::ToolExecutor,
}

impl RequestHandler {
    pub fn new(tool_executor: crate::starlark::engine::ToolExecutor) -> Self {
        Self { tool_executor }
    }

    pub fn handle_initialize(&self, request: &JsonRpcRequest) -> JsonRpcResponse {
        debug!("Handling initialize request");

        let result = InitializeResult {
            protocol_version: "2024-11-05".to_string(),
            capabilities: ServerCapabilities {
                tools: Some(ToolsCapability {
                    list_changed: Some(false),
                }),
            },
            server_info: ServerInfo {
                name: "starlark-mcp".to_string(),
                version: env!("MCP_STAR_VERSION").to_string(),
            },
        };

        JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: request.id.clone(),
            result: Some(serde_json::to_value(result).unwrap()),
            error: None,
        }
    }

    pub fn handle_initialized(&self, request: &JsonRpcRequest) -> JsonRpcResponse {
        debug!("Handling initialized notification");

        JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: request.id.clone(),
            result: Some(json!({})),
            error: None,
        }
    }

    pub fn handle_list_tools(&self, request: &JsonRpcRequest, tools: &[Tool]) -> JsonRpcResponse {
        debug!("Handling tools/list request");

        let result = ListToolsResult {
            tools: tools.to_vec(),
        };

        JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: request.id.clone(),
            result: Some(serde_json::to_value(result).unwrap()),
            error: None,
        }
    }

    pub async fn handle_call_tool(
        &self,
        request: &JsonRpcRequest,
        tools: &[Tool],
    ) -> JsonRpcResponse {
        debug!("Handling tools/call request");

        let params: CallToolParams = match serde_json::from_value(request.params.clone()) {
            Ok(p) => p,
            Err(e) => {
                error!("Failed to parse tool call params: {}", e);
                return self.error_response(request, -32602, "Invalid params");
            }
        };

        // Find the tool (validate it exists)
        let _tool = match tools.iter().find(|t| t.name == params.name) {
            Some(t) => t,
            None => {
                error!("Tool not found: {}", params.name);
                return self.error_response(request, -32601, "Tool not found");
            }
        };

        // Execute the tool
        match self
            .tool_executor
            .execute_tool(&params.name, params.arguments)
            .await
        {
            Ok(result) => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id.clone(),
                result: Some(serde_json::to_value(result).unwrap()),
                error: None,
            },
            Err(e) => {
                error!("Tool execution failed: {}", e);
                let error_result = ToolResult {
                    content: vec![ToolContent::Text {
                        text: format!("Error: {}", e),
                    }],
                    is_error: Some(true),
                };
                JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: request.id.clone(),
                    result: Some(serde_json::to_value(error_result).unwrap()),
                    error: None,
                }
            }
        }
    }

    pub fn handle_unknown(&self, request: &JsonRpcRequest) -> JsonRpcResponse {
        error!("Unknown method: {}", request.method);
        self.error_response(request, -32601, "Method not found")
    }

    fn error_response(
        &self,
        request: &JsonRpcRequest,
        code: i32,
        message: &str,
    ) -> JsonRpcResponse {
        JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: request.id.clone(),
            result: None,
            error: Some(JsonRpcError {
                code,
                message: message.to_string(),
                data: None,
            }),
        }
    }
}
