use anyhow::Result;
use argh::FromArgs;
use tracing::info;

use starlark_mcp::ExtensionLoader;

#[derive(FromArgs)]
/// Starlark-based MCP server
struct Args {
    /// path to the extensions directory
    #[argh(option, short = 'e', default = "\"./extensions\".to_string()")]
    extensions_dir: String,

    /// print version and exit
    #[argh(switch, short = 'v')]
    version: bool,

    /// run tests instead of starting the server
    #[argh(switch, short = 't')]
    test: bool,

    /// run in HTTP mode instead of stdio
    #[argh(switch)]
    http: bool,

    /// port for HTTP server (default: 3000)
    #[argh(option, short = 'p', default = "3000")]
    port: u16,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args: Args = argh::from_env();

    if args.version {
        println!("starlark-mcp {}", env!("STARLARK_MCP_VERSION"));
        return Ok(());
    }

    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .without_time()
        .init();

    if args.test {
        info!("Running tests from {}", args.extensions_dir);
        if let Err(_e) = starlark_mcp::run_tests(&args.extensions_dir).await {
            std::process::exit(1);
        }
        return Ok(());
    }

    info!("Starting Starlark MCP Server");

    let tool_executor =
        starlark_mcp::ToolExecutor::new().with_extensions_dir(args.extensions_dir.clone());
    let engine = tool_executor.engine();

    let loader = ExtensionLoader::new(args.extensions_dir);
    loader.load_all(&engine, false).await?;

    let handler = starlark_mcp::StarlarkMcpHandler::new(tool_executor);
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

    let handler_for_watcher = handler.clone();
    loader.start_watching(engine.clone(), move || {
        let handler = handler_for_watcher.clone();
        tokio::spawn(async move {
            info!("Extension changed, refreshing tools...");
            handler.refresh_tools().await;
        });
    })?;

    info!("Server ready, starting main loop");
    if args.http {
        starlark_mcp::run_rmcp_server_http(handler, args.port).await?;
    } else {
        starlark_mcp::run_rmcp_server(handler).await?;
    }

    Ok(())
}
