# starlark-mcp

A Model Context Protocol (MCP) server that runs Starlark extensions.

## Installation

```bash
npm install -g starlark-mcp
```

Or run directly with npx:

```bash
npx starlark-mcp
```

## Usage

```bash
# Run the MCP server (default, loads extensions from ./extensions)
starlark-mcp

# Run with a custom extensions directory
starlark-mcp -e /path/to/extensions

# Print version
starlark-mcp --version

# Run tests from extensions directory
starlark-mcp --test
```

## Supported Platforms

- Linux x64 (glibc)
- Linux ARM64 (musl)
- macOS x64
- macOS ARM64 (Apple Silicon)
- Windows x64

## License

MIT
