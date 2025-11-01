use anyhow::Result;
use argh::FromArgs;
use tracing::info;

use mcp_star::ExtensionLoader;

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

    if args.version {
        println!("mcp-star {}", env!("MCP_STAR_VERSION"));
        return Ok(());
    }

    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .init();

    info!("Starting Starlark MCP Server");

    let tool_executor = mcp_star::ToolExecutor::new();
    let engine = tool_executor.engine();

    let loader = ExtensionLoader::new(args.extensions_dir);
    loader.load_all(&engine).await?;

    let handler = mcp_star::StarlarkMcpHandler::new(tool_executor);

    // Register all tools from loaded extensions
    let extensions = engine.get_all_extensions().await;
    for extension in extensions {
        info!(
            "Registering extension '{}' with {} tools",
            extension.name,
            extension.tools.len()
        );

        for tool in extension.to_mcp_tools() {
            handler.register_tool(tool).await;
        }
    }

    info!("Server ready, starting main loop");
    mcp_star::run_rmcp_server(handler).await?;

    Ok(())
}
