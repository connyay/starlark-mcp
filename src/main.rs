use anyhow::Result;
use argh::FromArgs;
use tracing::info;

use mcp_star::{ExtensionLoader, McpServer};

#[derive(FromArgs)]
/// Starlark-based MCP server
struct Args {
    /// path to the extensions directory
    #[argh(option, short = 'e', default = "\"./extensions\".to_string()")]
    extensions_dir: String,

    /// print version and exit
    #[argh(switch, short = 'v')]
    version: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args: Args = argh::from_env();

    // Handle version flag
    if args.version {
        println!("mcp-star {}", env!("MCP_STAR_VERSION"));
        return Ok(());
    }

    // Initialize logging
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .init();

    info!("Starting Starlark MCP Server");

    // Create the tool executor and engine
    let tool_executor = mcp_star::ToolExecutor::new();
    let engine = tool_executor.engine();

    // Load extensions
    let loader = ExtensionLoader::new(args.extensions_dir);
    loader.load_all(&engine).await?;

    // Create the MCP server with the tool executor
    let server = McpServer::new(tool_executor);

    // Register all tools from loaded extensions
    let extensions = engine.get_all_extensions().await;
    for extension in extensions {
        info!(
            "Registering extension '{}' with {} tools",
            extension.name,
            extension.tools.len()
        );

        for tool in extension.to_mcp_tools() {
            server.register_tool(tool).await;
        }
    }

    // Run the server
    info!("Server ready, starting main loop");
    server.run().await?;

    Ok(())
}
