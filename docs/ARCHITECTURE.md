# Architecture

This document describes the internal architecture of starlark-mcp, how its components interact, and key design decisions.

## System Overview

starlark-mcp is a meta-server that combines the [Starlark](https://github.com/bazelbuild/starlark) scripting language with the [Model Context Protocol (MCP)](https://modelcontextprotocol.io/) to enable dynamic tool loading. Instead of compiling separate MCP servers for each integration, developers can write simple `.star` scripts that are loaded and executed at runtime.

**Key Value Propositions:**

- **Dynamic Loading**: Drop `.star` files into `extensions/` directory - no compilation needed
- **Hot Reload**: Changes are detected and reloaded automatically without server restart
- **Rich Runtime**: Built-in modules for HTTP, databases, system commands, and more
- **Safety**: Sandboxed Starlark execution with explicit exec whitelisting
- **Testing**: Convention-based test framework included

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                        MCP Client                               │
│                   (Claude Desktop, Zed, etc.)                   │
└────────────────────────────┬────────────────────────────────────┘
                             │ stdio (JSON-RPC)
                             │
┌────────────────────────────▼───────────────────────────────────┐
│                      MCP Server Layer                          │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  rmcp_server.rs (ServerHandler)                         │   │
│  │  - initialize: Protocol negotiation                     │   │
│  │  - list_tools: Return all registered tools              │   │
│  │  - call_tool: Delegate to ToolExecutor                  │   │
│  └──────────────────────────┬──────────────────────────────┘   │
└─────────────────────────────┼──────────────────────────────────┘
                              │
┌─────────────────────────────▼──────────────────────────────────┐
│                    Starlark Engine Layer                       │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │  StarlarkEngine (engine.rs)                              │  │
│  │  - Manages LoadedExtensions                              │  │
│  │  - Provides globals & modules                            │  │
│  │  - Handles tool execution                                │  │
│  └──────────────────┬───────────────────────────────────────┘  │
│                     │                                          │
│  ┌──────────────────▼───────────────────────────────────────┐  │
│  │  ToolExecutor (engine.rs)                                │  │
│  │  - Converts JSON → Starlark values                       │  │
│  │  - Invokes handler function from frozen module           │  │
│  │  - Converts Starlark result → JSON                       │  │
│  └──────────────────────────────────────────────────────────┘  │
└─────────────────────────────┬──────────────────────────────────┘
                              │
┌─────────────────────────────▼──────────────────────────────────┐
│                    Extension Loader Layer                      │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │  ExtensionLoader (loader.rs)                             │  │
│  │  - Discovers .star files in extensions/                  │  │
│  │  - Watches for file changes (notify crate)               │  │
│  │  - Filters out *_test.star files                         │  │
│  │  - Triggers reload on add/modify/remove                  │  │
│  └──────────────────┬───────────────────────────────────────┘  │
└─────────────────────┼──────────────────────────────────────────┘
                      │
┌─────────────────────▼──────────────────────────────────────────┐
│                    Extension Files (.star)                     │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │  describe_extension() → Extension                        │  │
│  │  - name, version, description                            │  │
│  │  - allowed_exec: ["cmd1", "cmd2"]                        │  │
│  │  - tools: [Tool(...), ...]                               │  │
│  │                                                          │  │
│  │  Tool handler functions                                  │  │
│  │  - def handler(params): ...                              │  │
│  │  - Returns {content: [...], isError: bool}               │  │
│  └──────────────────────────────────────────────────────────┘  │
└────────────────────────────────────────────────────────────────┘

┌────────────────────────────────────────────────────────────────┐
│                    Module System (Available to Extensions)     │
│  - time:     time.now()                                        │
│  - env:      env.get(name, default)                            │
│  - exec:     exec.run(cmd, args)  [with whitelist]             │
│  - http:     http.get(), http.post()                           │
│  - sqlite:   sqlite.query(), sqlite.list_tables()              │
│  - postgres: postgres.query(), postgres.execute()              │
│  - math:     math.pow(), math.sqrt(), etc.                     │
│  - json:     json.encode(), json.decode() [Starlark stdlib]    │
└────────────────────────────────────────────────────────────────┘
```

## Core Components

### 1. MCP Server Layer (`src/mcp/rmcp_server.rs`)

**Responsibility**: Implements the MCP protocol and handles client communication.

**Key Functionality**:

- Uses the `rmcp` library for MCP protocol implementation
- Communicates over stdio using JSON-RPC 2.0
- Implements `ServerHandler` trait with three main methods:
  - `initialize()`: Negotiates protocol version with client
  - `list_tools()`: Returns metadata for all registered tools
  - `call_tool()`: Delegates tool execution to ToolExecutor

**Protocol Support**:

- Supports multiple MCP protocol versions: 2024-11-05, 2025-03-26, 2025-06-18
- Announces `tools` capability with `listChanged: true` for hot reload support

**Data Flow**:

```
MCP Client Request → stdio → rmcp deserialize → call_tool(name, args)
    → ToolExecutor.execute() → JSON response → rmcp serialize → stdio
```

### 2. Starlark Engine (`src/starlark/engine.rs`)

**Responsibility**: Manages the Starlark runtime, loaded extensions, and tool execution.

**Key Structures**:

```rust
pub struct StarlarkEngine {
    loaded_extensions: Arc<RwLock<HashMap<String, LoadedExtension>>>,
}

pub struct LoadedExtension {
    extension: Extension,
    frozen_module: FrozenModule,  // Immutable, thread-safe Starlark module
}

pub struct ToolExecutor {
    extension: Extension,
    frozen_module: FrozenModule,
}
```

**Key Functionality**:

1. **Extension Loading** (`load_extension(path)`):
   - Creates new Starlark environment with globals and modules
   - Evaluates `.star` file to produce a Module
   - Calls `describe_extension()` to get Extension metadata
   - Freezes the module for thread-safe execution
   - Stores in `loaded_extensions` HashMap

2. **Tool Execution** (`ToolExecutor.execute(tool_name, params_json)`):
   - Looks up tool handler function in frozen module
   - Converts JSON parameters to Starlark `Dict`
   - Invokes handler function with parameters
   - Converts Starlark return value back to JSON
   - Handles errors and formats MCP response

3. **Value Conversion**:
   - JSON → Starlark: `json_to_starlark_value()`
   - Starlark → JSON: `starlark_value_to_json()`
   - Supports primitives, arrays, objects/dicts

**Thread Safety**:

- `FrozenModule` is immutable and can be safely shared across threads
- `RwLock` allows concurrent reads for tool execution
- Each tool execution gets its own Starlark evaluation context

### 3. Extension Loader (`src/extensions/loader.rs`)

**Responsibility**: Discovers extension files and watches for changes.

**Key Functionality**:

1. **Extension Discovery** (`load_extensions(dir)`):
   - Scans directory for `*.star` files
   - Filters out `*_test.star` files in server mode
   - Calls `StarlarkEngine.load_extension()` for each file
   - Returns Vec of all loaded extensions

2. **Hot Reload** (`watch_extensions(dir, callback)`):
   - Uses `notify` crate to watch extensions directory
   - Detects Create, Modify, Remove events
   - Debounces rapid changes
   - Calls callback with file path on changes
   - MCP server calls `refresh_tools()` to update tool registry

**Watch Events**:

```
File modified → notify event → callback(path) → engine.load_extension(path)
    → mcp_handler.refresh_tools() → clients notified via listChanged
```

**Testing Mode**:

- In test mode (`--test`), includes `*_test.star` files
- Test files can `load()` functions from extension files
- Extensions are loaded but not registered as MCP tools

### 4. Module System (`src/starlark/modules.rs`)

**Responsibility**: Provides built-in functionality to Starlark extensions.

**Module Registration**:
Modules are added to globals before evaluating extensions:

```rust
globals_builder.set("time", time_module());
globals_builder.set("env", env_module());
globals_builder.set("exec", exec_module());
globals_builder.set("http", http_module());
// etc.
```

**Key Modules**:

1. **exec** (`src/starlark/modules.rs`):
   - `exec.run(cmd, args)` executes system commands
   - Uses thread-local storage for exec whitelist
   - Validates command against extension's `allowed_exec` list
   - Returns {stdout, stderr, exit_code}

2. **http** (`src/starlark/http.rs`):
   - `http.get(url, headers)` for GET requests
   - `http.post(url, body, headers)` for POST requests
   - Uses `reqwest` library, blocks on async operations
   - Returns {status, body, headers}

3. **sqlite** (`src/starlark/sqlite.rs`):
   - `sqlite.query(path, sql, params)` for SELECT queries
   - `sqlite.execute(path, sql, params)` for INSERT/UPDATE/DELETE
   - `sqlite.list_tables(path)` and `sqlite.describe_table(path, name)`
   - Uses `rusqlite` library

4. **postgres** (`src/starlark/postgres.rs`):
   - Similar API to sqlite but with connection string
   - `postgres.query(conn_str, sql, params)`
   - Uses `postgres` crate

5. **time**, **env**, **math**:
   - Simple utility modules for common operations
   - See [MODULES.md](./MODULES.md) for complete reference

**Security Model**:

- Starlark provides sandboxing (no file I/O, network, or system access by default)
- Modules explicitly grant capabilities (HTTP, exec, database access)
- `exec` module enforces whitelist to prevent arbitrary command execution

### 5. MCP Type System (`src/starlark/mcp_types.rs`)

**Responsibility**: Defines Starlark-native types for extension metadata.

**Key Types**:

```starlark
Extension(
    name = "string",
    version = "string",
    description = "string",
    allowed_exec = ["cmd1", "cmd2"],  # Optional
    tools = [Tool(...)]
)

Tool(
    name = "string",
    description = "string",
    parameters = [ToolParameter(...)],
    handler = function_reference
)

ToolParameter(
    name = "string",
    param_type = "string",  # "string", "integer", "number", "boolean"
    required = True/False,
    default = value,        # Optional
    description = "string"
)
```

**Serialization**:

- These types implement Starlark's `StarlarkValue` trait
- Can be converted to/from JSON for MCP protocol
- Tool parameters are converted to JSON Schema format for `list_tools` response

## Data Flow

### Extension Loading Flow

```
1. Server startup / File change event
2. ExtensionLoader.load_extensions(dir)
3. For each .star file:
   a. StarlarkEngine.load_extension(path)
   b. Create new Starlark environment with globals + modules
   c. eval(file_contents) → Module
   d. Call describe_extension() → Extension
   e. Freeze module → FrozenModule
   f. Store LoadedExtension {extension, frozen_module}
4. MCPHandler stores all tools for list_tools()
```

### Tool Invocation Flow

```
1. MCP client sends call_tool request via stdio
2. rmcp library deserializes JSON-RPC request
3. MCPHandler.call_tool(name, arguments)
4. Find extension that provides tool
5. ToolExecutor.execute(tool_name, arguments_json)
   a. Look up handler function in frozen_module
   b. Convert JSON arguments → Starlark Dict
   c. Call handler(params_dict) in new eval context
   d. Handler may call modules (http.get, exec.run, etc.)
   e. Convert Starlark return value → JSON
6. Format as MCP tool response: {content: [...], isError: bool}
7. rmcp serializes to JSON-RPC response
8. Send response via stdio to client
```

### Hot Reload Flow

```
1. Developer edits extension .star file
2. notify library detects file modification
3. watch_extensions callback triggered with file path
4. StarlarkEngine.load_extension(path) replaces old version
5. MCPHandler.refresh_tools() updates tool registry
6. MCP client receives notification (listChanged: true)
7. Client calls list_tools() to get updated tool list
8. New/updated tools available for invocation
```

## Hot Reload Mechanism

The hot reload system is built on the `notify` crate and provides automatic extension updates without server restart.

**Implementation Details**:

1. **File Watching**:
   - Uses `notify::recommended_watcher()` for cross-platform file watching
   - Watches entire extensions directory recursively
   - Filters events to only `.star` files (excludes `*_test.star`)

2. **Debouncing**:
   - Multiple rapid changes to same file trigger single reload
   - Prevents duplicate loads during editor save operations

3. **Reload Strategy**:
   - Extension reloading is atomic at the file level
   - Old version remains available until new version loads successfully
   - If new version fails to load, old version continues working
   - Errors during reload are logged but don't crash server

4. **MCP Integration**:
   - After successful reload, `refresh_tools()` updates tool registry
   - Clients with `listChanged: true` are notified to re-fetch tools
   - No interruption to in-flight tool executions

**Concurrency Considerations**:

- Extension HashMap uses `RwLock` for thread-safe updates
- Tool execution holds read lock (many concurrent reads OK)
- Reload takes write lock (blocks briefly during reload)
- FrozenModules are immutable, safe to use during concurrent execution

## Testing System

The testing framework is integrated into the same runtime but operates in a different mode.

**Architecture**:

1. **Test Discovery** (`src/testing/mod.rs`):
   - Scans for `*_test.star` files
   - Loads each test file as a Starlark module
   - Discovers functions starting with `test_`

2. **Test Execution**:
   - Each test runs in isolated Starlark environment
   - `testing` module added to globals (only in test mode)
   - Test files can `load()` functions from extension files
   - All standard modules (http, exec, etc.) available

3. **Test Isolation**:
   - Each test gets fresh Starlark evaluation context
   - No state shared between tests
   - Tests run sequentially (not parallel)

4. **Assertion Module** (`src/testing/testing.rs`):
   - Provides assertion functions: `eq`, `ne`, `is_true`, etc.
   - Failures raise Starlark errors with descriptive messages
   - Test framework catches errors and reports as failures

See [TESTING.md](./TESTING.md) for complete testing documentation.

## Key Design Decisions

### Why Starlark?

1. **Python-like Syntax**: Familiar to most developers, low learning curve
2. **Deterministic**: No I/O, network, or system access by default (security)
3. **Fast**: Optimized for repeated evaluation, suitable for tool execution
4. **Embeddable**: Designed to be embedded in Rust/Go applications
5. **Frozen Modules**: Immutable modules enable safe concurrent execution

**Alternatives Considered**:

- **Lua**: Less familiar syntax, no frozen module concept
- **JavaScript**: Larger runtime, more attack surface
- **Python**: Too heavy, not sandboxed, not embeddable

### Why MCP?

1. **Standard Protocol**: Works with multiple AI clients (Claude Desktop, Zed, etc.)
2. **Tool-Focused**: Designed specifically for LLM tool integration
3. **Simple**: JSON-RPC over stdio, easy to debug
4. **Extensible**: Protocol supports additional capabilities

### Why Stdio Transport?

1. **MCP Standard**: Stdio is the primary transport for MCP servers
2. **Simple**: No network configuration, ports, or authentication needed
3. **Secure**: Process isolation, no network exposure
4. **Compatible**: Works with all MCP clients

### Why Rust for Runtime?

1. **Performance**: Fast startup, low memory overhead
2. **Safety**: Memory safety without garbage collection
3. **Async**: Tokio for concurrent tool execution
4. **Ecosystem**: Great Starlark, MCP, and system libraries
5. **Embeddable**: Single binary, no runtime dependencies

## Security Model

The security model is defense-in-depth with multiple layers:

1. **Starlark Sandbox**: No file I/O, network, or system access by default
2. **Explicit Capabilities**: Modules grant specific capabilities (http, exec, etc.)
3. **Exec Whitelist**: Extensions must declare allowed commands
4. **Process Isolation**: Server runs as separate process from client
5. **No Code Evaluation**: Extensions are loaded at startup/reload, not from client requests

## Extension Lifecycle

```
┌─────────────┐
│   Created   │  .star file written to extensions/
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  Discovered │  ExtensionLoader finds .star file
└──────┬──────┘
       │
       ▼
┌─────────────┐
│   Loaded    │  StarlarkEngine evaluates + freezes module
└──────┬──────┘
       │
       ▼
┌─────────────┐
│ Registered  │  Tools available via list_tools()
└──────┬──────┘
       │
       ▼
┌─────────────┐
│   Active    │  Can receive call_tool requests
└──────┬──────┘
       │
       ├─────► File modified → Hot Reload → Reloaded state
       │
       ▼
┌─────────────┐
│   Removed   │  File deleted → Unregistered
└─────────────┘
```

## Error Handling

### Extension Loading Errors

- Syntax errors: Reported at load time, extension not registered
- Runtime errors in `describe_extension()`: Logged, extension not registered
- Invalid metadata (missing fields): Validation error, extension not registered

### Tool Execution Errors

- Handler function errors: Caught, returned as MCP error response
- Invalid return value: Validation error, returned to client
- Module errors (http, exec): Propagated to handler, handler decides how to handle

### Hot Reload Errors

- Failed reload: Old version remains active, error logged
- File deleted: Extension unregistered, tools removed
- File parse error: Extension disabled until fixed

## References

- **Starlark Spec**: <https://github.com/bazelbuild/starlark>
- **MCP Specification**: <https://modelcontextprotocol.io/>
- **rmcp Library**: <https://docs.rs/rmcp/>
- **Module Reference**: [MODULES.md](./MODULES.md)
- **Testing Guide**: [TESTING.md](./TESTING.md)
