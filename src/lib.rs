pub mod extensions;
pub mod mcp;
pub mod starlark;

pub use extensions::ExtensionLoader;
pub use mcp::rmcp_server::{run_server as run_rmcp_server, StarlarkMcpHandler};
pub use starlark::{StarlarkEngine, ToolExecutor};

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_extension_loader_can_load_extension() {
        let engine = StarlarkEngine::new();

        let extension_content = r#"
def test_handler(params):
    return {
        "content": [{"type": "text", "text": "test response"}],
    }

def describe_extension():
    return Extension(
        name = "test",
        version = "1.0.0",
        description = "Test extension",
        tools = [
            Tool(
                name = "test_tool",
                description = "A test tool",
                handler = test_handler,
            ),
        ],
    )
"#;

        let result = engine.load_extension("test", extension_content).await;
        assert!(result.is_ok(), "Extension should load successfully");

        let extension = result.unwrap();
        assert_eq!(extension.name, "test");
        assert_eq!(extension.version, "1.0.0");
        assert_eq!(extension.tools.len(), 1);
        assert_eq!(extension.tools[0].name, "test_tool");
    }

    #[tokio::test]
    async fn test_extension_loader_can_load_from_directory() {
        let extensions_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("extensions");

        // Skip test if extensions directory doesn't exist
        if !extensions_dir.exists() {
            eprintln!("Skipping test: extensions directory not found");
            return;
        }

        let engine = StarlarkEngine::new();
        let loader = ExtensionLoader::new(extensions_dir.to_str().unwrap().to_string());

        let result = loader.load_all(&engine).await;
        assert!(result.is_ok(), "Should load extensions from directory");

        let extensions = engine.get_all_extensions().await;
        assert!(
            !extensions.is_empty(),
            "Should have loaded at least one extension"
        );
    }

    #[tokio::test]
    async fn test_extension_can_retrieve_by_name() {
        let engine = StarlarkEngine::new();

        let extension_content = r#"
def handler(params):
    return {"content": [{"type": "text", "text": "response"}]}

def describe_extension():
    return Extension(
        name = "retrievable",
        version = "1.0.0",
        description = "Extension for retrieval test",
        tools = [Tool(name = "tool1", description = "Tool", handler = handler)],
    )
"#;

        engine
            .load_extension("retrievable", extension_content)
            .await
            .unwrap();

        let retrieved = engine.get_extension("retrievable").await;
        assert!(retrieved.is_some(), "Should retrieve extension by name");
        assert_eq!(retrieved.unwrap().name, "retrievable");
    }

    #[tokio::test]
    async fn test_tool_executor_can_execute_tool() {
        let executor = ToolExecutor::new();
        let engine = executor.engine();

        let extension_content = r#"
def echo_handler(params):
    message = params.get("message", "default")
    return {
        "content": [{"type": "text", "text": message}],
    }

def describe_extension():
    return Extension(
        name = "echo_ext",
        version = "1.0.0",
        description = "Echo extension",
        tools = [
            Tool(
                name = "echo_tool",
                description = "Echoes the message",
                handler = echo_handler,
            ),
        ],
    )
"#;

        engine
            .load_extension("echo_ext", extension_content)
            .await
            .unwrap();

        let args = serde_json::json!({"message": "Hello, world!"});
        let result = executor.execute_tool("echo_tool", args).await;

        assert!(result.is_ok(), "Tool execution should succeed");
        let tool_result = result.unwrap();
        assert_eq!(tool_result.content.len(), 1);

        if let Some(mcp::ToolContent::Text { text }) = tool_result.content.get(0) {
            assert_eq!(text, "Hello, world!");
        } else {
            panic!("Expected text content in tool result");
        }
    }

    #[tokio::test]
    async fn test_extension_loader_handles_missing_directory() {
        let engine = StarlarkEngine::new();
        let loader = ExtensionLoader::new("./nonexistent_directory".to_string());

        let result = loader.load_all(&engine).await;
        assert!(result.is_ok(), "Should handle missing directory gracefully");
    }

    #[tokio::test]
    async fn test_extension_with_multiple_tools() {
        let engine = StarlarkEngine::new();

        let extension_content = r#"
def handler1(params):
    return {"content": [{"type": "text", "text": "tool1"}]}

def handler2(params):
    return {"content": [{"type": "text", "text": "tool2"}]}

def describe_extension():
    return Extension(
        name = "multi_tool",
        version = "1.0.0",
        description = "Extension with multiple tools",
        tools = [
            Tool(name = "tool1", description = "First tool", handler = handler1),
            Tool(name = "tool2", description = "Second tool", handler = handler2),
        ],
    )
"#;

        let result = engine.load_extension("multi_tool", extension_content).await;
        assert!(result.is_ok());

        let extension = result.unwrap();
        assert_eq!(extension.tools.len(), 2);
        assert_eq!(extension.tools[0].name, "tool1");
        assert_eq!(extension.tools[1].name, "tool2");
    }
}
