use anyhow::Result;
use rmcp::model::{
    CallToolRequestParam, CallToolResult, Content, Implementation, InitializeRequestParam,
    InitializeResult, ListToolsResult, PaginatedRequestParam, ProtocolVersion, ServerCapabilities,
    Tool as RmcpTool,
};
use rmcp::service::{RequestContext, RoleServer};
use rmcp::{ErrorData as McpError, ServerHandler};
use serde_json::{json, Map, Value};
use std::borrow::Cow;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info};

use crate::mcp::Tool;
use crate::starlark::engine::ToolExecutor;

/// Adapter that bridges rmcp's ServerHandler with our Starlark ToolExecutor
pub struct StarlarkMcpHandler {
    tools: Arc<RwLock<Vec<Tool>>>,
    tool_executor: ToolExecutor,
}

impl StarlarkMcpHandler {
    pub fn new(tool_executor: ToolExecutor) -> Self {
        Self {
            tools: Arc::new(RwLock::new(Vec::new())),
            tool_executor,
        }
    }

    pub async fn register_tool(&self, tool: Tool) {
        let mut tools = self.tools.write().await;
        info!("Registering tool: {}", tool.name);
        tools.push(tool);
    }

    /// Convert our custom Tool to rmcp's Tool format
    fn convert_to_rmcp_tool(tool: &Tool) -> RmcpTool {
        let mut schema_map = Map::new();
        schema_map.insert(
            "type".to_string(),
            Value::String(tool.input_schema.schema_type.clone()),
        );
        schema_map.insert(
            "properties".to_string(),
            Value::Object(tool.input_schema.properties.clone().into_iter().collect()),
        );
        schema_map.insert(
            "required".to_string(),
            Value::Array(
                tool.input_schema
                    .required
                    .iter()
                    .map(|s| Value::String(s.clone()))
                    .collect(),
            ),
        );

        RmcpTool {
            name: Cow::Owned(tool.name.clone()),
            title: None,
            description: Some(Cow::Owned(tool.description.clone())),
            input_schema: Arc::new(schema_map),
            output_schema: None,
            annotations: None,
            icons: None,
        }
    }
}

impl ServerHandler for StarlarkMcpHandler {
    async fn initialize(
        &self,
        request: InitializeRequestParam,
        context: RequestContext<RoleServer>,
    ) -> Result<InitializeResult, McpError> {
        info!(
            "Initialize request received from {} with protocol version {}",
            request.client_info.name, request.protocol_version
        );

        if context.peer.peer_info().is_none() {
            context.peer.set_peer_info(request.clone());
        }

        // Negotiate protocol version: prefer client's if supported, fallback to latest
        let client_version = request.protocol_version.to_string();
        let protocol_version = match client_version.as_str() {
            "2025-06-18" => ProtocolVersion::V_2025_06_18,
            "2025-03-26" => ProtocolVersion::V_2025_03_26,
            "2024-11-05" => ProtocolVersion::V_2024_11_05,
            _ => {
                info!(
                    "Client requested unsupported version {}, using latest supported",
                    client_version
                );
                ProtocolVersion::LATEST
            }
        };

        info!("Negotiated protocol version: {}", protocol_version);

        Ok(InitializeResult {
            protocol_version,
            capabilities: ServerCapabilities {
                tools: Some(serde_json::from_value(json!({ "listChanged": false })).unwrap()),
                ..Default::default()
            },
            server_info: Implementation {
                name: "starlark-mcp".to_string(),
                version: env!("STARLARK_MCP_VERSION").to_string(),
                title: None,
                icons: None,
                website_url: None,
            },
            instructions: None,
        })
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        info!("List tools request received");

        let tools = self.tools.read().await;
        let rmcp_tools: Vec<RmcpTool> = tools.iter().map(Self::convert_to_rmcp_tool).collect();

        Ok(ListToolsResult {
            tools: rmcp_tools,
            next_cursor: None,
        })
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        info!("Call tool request received: {}", request.name);

        let tools = self.tools.read().await;
        if !tools.iter().any(|t| t.name == request.name) {
            error!("Tool not found: {}", request.name);
            return Ok(CallToolResult {
                content: vec![Content::text(format!("Tool not found: {}", request.name))],
                is_error: Some(true),
                meta: None,
                structured_content: None,
            });
        }
        drop(tools);

        let arguments = request
            .arguments
            .map(serde_json::Value::Object)
            .unwrap_or(json!({}));

        match self
            .tool_executor
            .execute_tool(&request.name, arguments)
            .await
        {
            Ok(result) => {
                let content: Vec<Content> = result
                    .content
                    .iter()
                    .map(|c| match c {
                        crate::mcp::ToolContent::Text { text } => Content::text(text.clone()),
                    })
                    .collect();

                Ok(CallToolResult {
                    content,
                    is_error: result.is_error,
                    meta: None,
                    structured_content: None,
                })
            }
            Err(e) => {
                error!("Tool execution failed: {}", e);
                Ok(CallToolResult {
                    content: vec![Content::text(format!("Error: {}", e))],
                    is_error: Some(true),
                    meta: None,
                    structured_content: None,
                })
            }
        }
    }

    fn get_info(&self) -> InitializeResult {
        InitializeResult {
            protocol_version: ProtocolVersion::LATEST,
            capabilities: ServerCapabilities {
                tools: Some(serde_json::from_value(json!({ "listChanged": false })).unwrap()),
                ..Default::default()
            },
            server_info: Implementation {
                name: "starlark-mcp".to_string(),
                version: env!("STARLARK_MCP_VERSION").to_string(),
                title: None,
                icons: None,
                website_url: None,
            },
            instructions: None,
        }
    }
}

pub async fn run_server(handler: StarlarkMcpHandler) -> Result<()> {
    info!("Starting rmcp-based MCP server...");

    use rmcp::transport::stdio;
    use rmcp::ServiceExt;

    let service = handler.serve(stdio()).await?;

    // Block until shutdown - rmcp requires this to keep the server alive
    service.waiting().await?;

    Ok(())
}
