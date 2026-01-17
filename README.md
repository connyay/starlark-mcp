# Starlark MCP

A Starlark-powered MCP (Model Context Protocol) server that enables dynamic tool loading through Starlark scripting.

## Overview

**starlark-mcp** is a meta-server that combines the [Starlark](https://github.com/bazelbuild/starlark) scripting language with the [Model Context Protocol (MCP)](https://modelcontextprotocol.io/) to enable dynamic tool loading. Instead of compiling separate MCP servers for each integration, you write simple `.star` scripts that are loaded and executed at runtime.

### Why starlark-mcp?

- **No Compilation**: Write tools in Python-like Starlark, no Rust/Go/TypeScript required
- **Hot Reload**: Changes to extensions are automatically detected and reloaded
- **Rich Runtime**: Built-in modules for HTTP, databases, system commands, and more
- **Secure**: Sandboxed execution with explicit command whitelisting
- **Testable**: Convention-based testing framework included
- **Production-Ready**: Single binary, minimal dependencies, cross-platform

### Architecture at a Glance

```
MCP Client (Claude Desktop, Zed, etc.)
    ↓ stdio (JSON-RPC)
MCP Server Layer (rmcp)
    ↓
Starlark Engine (loads & executes .star files)
    ↓
Extensions (.star files in extensions/)
    ↓ via modules
Built-in Capabilities (http, exec, sqlite, postgres, etc.)
```

See [docs/ARCHITECTURE.md](./docs/ARCHITECTURE.md) for detailed architecture documentation.

## Key Features

### Dynamic Extension Loading

- Drop `.star` files into `extensions/` directory
- No compilation or server restart needed
- Hot reload automatically detects changes

### Rich Module System

- **http**: REST API integration (`http.get()`, `http.post()`)
- **exec**: CLI tool wrappers with whitelist (`exec.run()`)
- **sqlite/postgres**: Database integration
- **math**: Mathematical operations
- **time**: Timestamps
- **env**: Environment variable access

See [docs/MODULES.md](./docs/MODULES.md) for complete module reference.

### Built-in Testing Framework

- Write tests in `*_test.star` files
- Convention-based test discovery
- Assertion library included
- Run with `starlark-mcp --test`

See [docs/TESTING.md](./docs/TESTING.md) for testing guide.

### Security

- **Starlark Sandboxing**: No file I/O or network access by default
- **Exec Whitelist**: Extensions must declare allowed commands
- **Process Isolation**: Separate process from MCP client

## Quick Start

### Installation

#### npm (Recommended)

```bash
# Run directly with npx (no install needed)
npx starlark-mcp

# Or install globally
npm install -g starlark-mcp
```

#### From Source

```bash
# Clone repository
git clone https://github.com/connyay/starlark-mcp.git
cd starlark-mcp

# Build release binary
cargo build --release

# Binary will be at: target/release/starlark-mcp
```

#### Pre-built Binaries

Download from [GitHub Releases](https://github.com/connyay/starlark-mcp/releases) for your platform:

- Linux (x86_64, ARM64)
- macOS (x86_64, ARM64)
- Windows (x86_64)

### Running the Server

```bash
# Start server with default extensions directory
starlark-mcp

# Use custom extensions directory
starlark-mcp --extensions-dir /path/to/extensions

# Run tests
starlark-mcp --test
```

**What happens when you start the server:**

1. Loads all `.star` extensions from `./extensions/` directory
2. Starts MCP server listening on stdin
3. Watches for extension file changes (hot reload enabled)
4. Logs to stderr

See [docs/CLI_REFERENCE.md](./docs/CLI_REFERENCE.md) for complete CLI documentation.

### Integrating with MCP Clients

#### Claude Desktop

Add to `~/Library/Application Support/Claude/claude_desktop_config.json` (macOS):

```json
{
  "mcpServers": {
    "starlark": {
      "command": "/path/to/starlark-mcp",
      "args": ["--extensions-dir", "/path/to/extensions"],
      "env": {
        "MY_API_KEY": "your-api-key-here"
      }
    }
  }
}
```

#### Zed Editor

Add to Zed configuration:

```json
{
  "context_servers": {
    "starlark-mcp": {
      "command": {
        "path": "/path/to/starlark-mcp",
        "args": ["--extensions-dir", "/path/to/extensions"]
      }
    }
  }
}
```

### Your First Extension

Create `extensions/hello.star`:

```python
def say_hello(params):
    """Say hello to someone"""
    name = params.get("name", "World")
    return {
        "content": [{"type": "text", "text": "Hello, {}!".format(name)}],
    }

def describe_extension():
    return Extension(
        name = "hello",
        version = "1.0.0",
        description = "Simple greeting extension",
        tools = [
            Tool(
                name = "say_hello",
                description = "Say hello to someone",
                parameters = [
                    ToolParameter(
                        name = "name",
                        param_type = "string",
                        required = False,
                        default = "World",
                        description = "Name to greet",
                    ),
                ],
                handler = say_hello,
            ),
        ],
    )
```

The server will automatically detect and load the new extension (hot reload). No restart needed!

## Extension Examples

### Simple Extension (No Dependencies)

The included `cat_facts.star` demonstrates a basic extension:

```python
def get_cat_fact(params):
    """Returns a random cat fact"""
    facts = [
        "Cats sleep 12-16 hours a day.",
        "A group of cats is called a 'clowder'.",
        "Cats have over 30 muscles in each ear.",
    ]

    index = time.now() % len(facts)

    return {
        "content": [{"type": "text", "text": facts[index]}],
    }

def describe_extension():
    return Extension(
        name = "cat_facts",
        version = "1.0.0",
        description = "Fun facts about cats",
        tools = [
            Tool(
                name = "get_cat_fact",
                description = "Get a random fact about cats",
                handler = get_cat_fact,
            ),
        ],
    )
```

### HTTP API Integration

```python
def get_weather(params):
    """Get weather from an API"""
    city = params.get("city", "")

    if not city:
        return error_response("city parameter is required")

    response = http.get(
        url = "https://api.weather.gov/...",
        headers = {"User-Agent": "starlark-mcp"}
    )

    if response.get("status_code", 0) != 200:
        return error_response("API request failed")

    data = response.get("json", {})
    # Format and return data...
```

### CLI Tool Wrapper

```python
def list_repos(params):
    """List GitHub repositories using gh CLI"""
    org = params.get("org", "")

    if not org:
        return error_response("org parameter is required")

    result = exec.run("gh", ["repo", "list", org, "--json", "name"])

    if not result["success"]:
        return error_response(result["stderr"])

    repos = json.decode(result["stdout"])
    # Format and return repos...

def describe_extension():
    return Extension(
        name = "github",
        allowed_exec = ["gh"],  # Required for exec.run()
        tools = [...],
    )
```

### Database Integration

```python
def query_users(params):
    """Query SQLite database"""
    db_path = params.get("db_path", "")

    query = "SELECT * FROM users WHERE active = ?"
    results = sqlite.query(db_path, query, [True])

    # Format and return results...
```

See [docs/EXTENSION_DEVELOPMENT.md](./docs/EXTENSION_DEVELOPMENT.md) for comprehensive extension development guide with patterns, best practices, and anti-patterns.

## Example Extensions

The repository includes 11 example extensions:

- **cat_facts**: Simple facts (no dependencies)
- **weather**: National Weather Service API integration
- **github**: GitHub CLI wrapper
- **docker**: Docker CLI wrapper
- **kubectl**: Kubernetes CLI wrapper
- **postgres**: PostgreSQL database tools
- **sqlite**: SQLite database tools
- **plane**: Plane.so API integration
- And more...

Explore the `extensions/` directory for complete examples.

## Documentation

### Core Documentation

- [README.md](./README.md) - This file (overview and quick start)
- [docs/ARCHITECTURE.md](./docs/ARCHITECTURE.md) - System architecture and design
- [docs/CLI_REFERENCE.md](./docs/CLI_REFERENCE.md) - Complete CLI documentation

### Extension Development

- [docs/EXTENSION_DEVELOPMENT.md](./docs/EXTENSION_DEVELOPMENT.md) - Extension development guide
- [docs/MODULES.md](./docs/MODULES.md) - Built-in module reference
- [docs/TESTING.md](./docs/TESTING.md) - Testing framework guide

### Claude Code Integration

- [.claude/skills/create-extension/SKILL.md](./.claude/skills/create-extension/SKILL.md) - AI-assisted extension creation

## Development Workflow

### 1. Write Extension

```bash
# Create new extension
vim extensions/my_extension.star

# Extension is automatically loaded (hot reload)
```

### 2. Test Extension

```bash
# Write tests
vim extensions/my_extension_test.star

# Run tests
starlark-mcp --test
```

### 3. Use in MCP Client

Configure your MCP client (Claude Desktop, Zed, etc.) to use starlark-mcp, then use your tools naturally in conversations.

## Contributing

Contributions welcome! Please:

1. Read [docs/EXTENSION_DEVELOPMENT.md](./docs/EXTENSION_DEVELOPMENT.md) for development patterns
2. Write tests for new functionality
3. Follow existing code style

## MCP Protocol Support

Supported MCP protocol versions:

- 2024-11-05
- 2025-03-26
- 2025-06-18

Supported capabilities:

- `tools` with `listChanged: true` (hot reload support)

Supported MCP methods:

- `initialize` - Server initialization and capability negotiation
- `initialized` - Initialization confirmation
- `tools/list` - List available tools
- `tools/call` - Execute a tool

## License

MIT
