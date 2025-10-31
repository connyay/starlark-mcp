use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

use super::handlers::RequestHandler;
use super::transport::StdioTransport;
use super::{JsonRpcRequest, JsonRpcResponse, Tool};

pub struct McpServer {
    tools: Arc<RwLock<Vec<Tool>>>,
    handler: RequestHandler,
}

impl McpServer {
    pub fn new(tool_executor: crate::starlark::engine::ToolExecutor) -> Self {
        Self {
            tools: Arc::new(RwLock::new(Vec::new())),
            handler: RequestHandler::new(tool_executor),
        }
    }

    pub async fn register_tool(&self, tool: Tool) {
        let mut tools = self.tools.write().await;
        info!("Registering tool: {}", tool.name);
        tools.push(tool);
    }

    pub async fn run(&self) -> Result<()> {
        let mut transport = StdioTransport::new();
        info!("MCP server started, waiting for requests...");

        loop {
            match transport.read_request().await {
                Ok(Some(request)) => {
                    let response = self.handle_request(request).await;
                    if let Err(e) = transport.write_response(&response).await {
                        warn!("Failed to write response: {}", e);
                    }
                }
                Ok(None) => {
                    info!("Connection closed");
                    break;
                }
                Err(e) => {
                    warn!("Error reading request: {}", e);
                    break;
                }
            }
        }

        Ok(())
    }

    async fn handle_request(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        info!("Handling request: {}", request.method);

        match request.method.as_str() {
            "initialize" => self.handler.handle_initialize(&request),
            "initialized" => self.handler.handle_initialized(&request),
            "tools/list" => {
                let tools = self.tools.read().await;
                self.handler.handle_list_tools(&request, &tools)
            }
            "tools/call" => {
                let tools = self.tools.read().await;
                self.handler.handle_call_tool(&request, &tools).await
            }
            _ => self.handler.handle_unknown(&request),
        }
    }
}
