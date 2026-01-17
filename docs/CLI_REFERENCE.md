# CLI Reference

Complete command-line interface documentation for starlark-mcp.

## Synopsis

```bash
starlark-mcp [OPTIONS]
```

## Description

starlark-mcp is a Starlark-based MCP (Model Context Protocol) server that dynamically loads extensions from `.star` files and exposes them as MCP tools.

The server runs in one of two modes:

1. **Server Mode** (default): Starts the MCP server and loads extensions
2. **Test Mode** (`--test`): Runs tests from `*_test.star` files

## Options

### `-e, --extensions-dir <PATH>`

**Description**: Path to the directory containing Starlark extension files (`.star`).

**Default**: `./extensions`

**Examples**:

```bash
# Use default extensions directory
starlark-mcp

# Use custom extensions directory
starlark-mcp --extensions-dir /path/to/extensions

# Short form
starlark-mcp -e ./my-extensions
```

**Behavior**:

- In **server mode**: Loads all `*.star` files except `*_test.star`
- In **test mode**: Loads all `*_test.star` files
- Extensions are loaded at startup and watched for changes (hot reload)
- Directory must exist or server will fail to start

**Extension Discovery**:

The loader finds extensions using these rules:

1. Scans for files matching `*.star`
2. In server mode: Excludes files matching `*_test.star`
3. In test mode: Only includes files matching `*_test.star`
4. Files are loaded in alphabetical order

### `-v, --version`

**Description**: Print version information and exit.

**Examples**:

```bash
starlark-mcp --version
# Output: starlark-mcp 0.1.0

starlark-mcp -v
# Output: starlark-mcp 0.1.0
```

**Behavior**:

- Prints version string to stdout
- Exits immediately with code 0
- Does not start server or load extensions

### `-t, --test`

**Description**: Run tests instead of starting the server.

**Examples**:

```bash
# Run tests from default extensions directory
starlark-mcp --test

# Run tests from custom directory
starlark-mcp --test --extensions-dir ./my-extensions

# Short form
starlark-mcp -t -e ./my-extensions
```

**Behavior**:

- Discovers `*_test.star` files in extensions directory
- Loads each test file as a Starlark module
- Discovers test functions (functions starting with `test_`)
- Runs each test in isolated environment
- Prints test results to stderr
- Exits with code 0 if all tests pass, 1 if any fail

**Test Discovery**:

Tests are discovered using these rules:

1. Files must match `*_test.star`
2. Test functions must start with `test_`
3. Test files can `load()` functions from extension files
4. Each test runs in isolation

**Example Test Output**:

```
Running tests from ./extensions
  ✓ cat_facts_test.star::test_basic_fact
  ✓ weather_test.star::test_parse_json
  ✓ weather_test.star::test_error_response
✓ All tests passed (3 tests)
```

See [TESTING.md](./TESTING.md) for complete testing documentation.

## Environment Variables

starlark-mcp does not use environment variables for configuration. However, extensions can access environment variables using the `env` module:

```starlark
# In extension code
api_key = env.get("MY_API_KEY", "")
```

**Common Environment Variables** (extension-specific):

- `GITHUB_DEFAULT_REPO`: Default repository for GitHub extension
- `DATABASE_URL`: PostgreSQL connection string
- API keys and tokens for various services

See extension documentation for specific environment variables.

## Exit Codes

- `0`: Success
  - Server shutdown gracefully
  - All tests passed
  - Version printed successfully

- `1`: Error
  - Extension loading failed
  - Test failures
  - Server error
  - Invalid command-line arguments

## Usage Examples

### Basic Usage

Start server with default settings:

```bash
starlark-mcp
```

**What happens**:

1. Loads extensions from `./extensions/`
2. Starts MCP server on stdio
3. Watches for extension file changes (hot reload)
4. Logs to stderr

**Expected output** (stderr):

```
Starting Starlark MCP Server
Registering extension 'cat_facts' with 1 tools
Registering extension 'weather' with 3 tools
Registering extension 'github' with 4 tools
Server ready, starting main loop
```

### Custom Extensions Directory

Use extensions from a different location:

```bash
starlark-mcp --extensions-dir /opt/my-extensions
```

### Running Tests

Run all tests:

```bash
starlark-mcp --test
```

**Expected output** (stderr):

```
Running tests from ./extensions
  ✓ cat_facts_test.star::test_basic_fact
  ✓ testing_test.star::test_eq_pass
  ✓ testing_test.star::test_contains_pass
✓ All tests passed (3 tests)
```

### Development Workflow

Typical development workflow:

```bash
# Terminal 1: Start server with hot reload
starlark-mcp --extensions-dir ./extensions

# Terminal 2: Edit extensions
vim extensions/my_extension.star

# Terminal 1: Server automatically reloads
# Output: Extension changed, refreshing tools...

# Terminal 2: Run tests
starlark-mcp --test
```

## Configuration Files

starlark-mcp does **not** use configuration files. All configuration is via:

1. **Command-line arguments**: Server options
2. **Environment variables**: Extension-specific secrets and configuration
3. **Extension code**: Tool behavior and metadata

## Logging

**Log Destination**: stderr

**Log Format**: Plain text, one line per event

**Log Levels**: Controlled by `RUST_LOG` environment variable (optional)

```bash
# Default logging (info level)
starlark-mcp

# Enable debug logging
RUST_LOG=debug starlark-mcp

# Enable trace logging (very verbose)
RUST_LOG=trace starlark-mcp

# Silence logs (not recommended)
RUST_LOG=error starlark-mcp
```

**Example Log Output**:

```
Starting Starlark MCP Server
Registering extension 'weather' with 3 tools
Server ready, starting main loop
Extension changed, refreshing tools...
Reloaded extension: weather
```

## Integration with MCP Clients

### Claude Desktop

Add to Claude Desktop configuration (`~/Library/Application Support/Claude/claude_desktop_config.json` on macOS):

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

### Zed Editor

Add to Zed configuration:

```json
{
  "context_servers": {
    "starlark-mcp": {
      "command": {
        "path": "/path/to/starlark-mcp",
        "args": ["--extensions-dir", "/path/to/extensions"],
        "env": {
          "MY_API_KEY": "your-api-key-here"
        }
      }
    }
  }
}
```

### Generic MCP Client

Any MCP client that supports stdio transport can use starlark-mcp:

1. **Transport**: stdio (stdin/stdout)
2. **Protocol**: MCP (JSON-RPC 2.0)
3. **Capabilities**: `tools` with `listChanged: true`

## Troubleshooting

### Server Won't Start

**Problem**: Server exits immediately or fails to start.

**Possible Causes**:

1. Extensions directory doesn't exist
2. Extension syntax errors
3. Invalid extension metadata

**Solutions**:

```bash
# Check if extensions directory exists
ls -la ./extensions

# Enable debug logging
RUST_LOG=debug starlark-mcp

# Check for syntax errors in extensions
# Look for error messages in stderr output
```

### Extensions Not Loading

**Problem**: Extensions in directory but not registered.

**Possible Causes**:

1. Files don't end in `.star`
2. Files end in `_test.star` (excluded in server mode)
3. Missing `describe_extension()` function
4. Errors in extension code

**Solutions**:

```bash
# Verify file names
ls extensions/*.star

# Check logs for error messages
RUST_LOG=debug starlark-mcp 2>&1 | grep -i error

# Test extension syntax
starlark-mcp --test --extensions-dir ./extensions
```

### Hot Reload Not Working

**Problem**: Changes to extension files not detected.

**Possible Causes**:

1. File is `*_test.star` (excluded from server mode)
2. File system watcher not working (rare)
3. Syntax error in new version (old version remains)

**Solutions**:

```bash
# Check server logs for reload messages
# Should see: "Extension changed, refreshing tools..."

# Restart server
# Ctrl+C and restart starlark-mcp

# Verify file is being saved
stat extensions/my_extension.star
```

### Tests Failing

**Problem**: Tests exit with code 1.

**Solutions**:

```bash
# Run tests with verbose output
RUST_LOG=debug starlark-mcp --test

# Check test files exist
ls extensions/*_test.star

# Verify test functions start with test_
grep "^def test_" extensions/*_test.star
```

### MCP Client Can't Connect

**Problem**: MCP client reports connection error.

**Possible Causes**:

1. Incorrect command path in client config
2. Incorrect arguments
3. Server crashes on startup

**Solutions**:

```bash
# Test server runs
starlark-mcp --version

# Test server starts
timeout 5 starlark-mcp 2>&1 | head -n 10

# Check client configuration
cat ~/Library/Application\ Support/Claude/claude_desktop_config.json
```

## Additional Resources

- [README.md](../README.md) - Project overview and quick start
- [ARCHITECTURE.md](./ARCHITECTURE.md) - System architecture
- [EXTENSION_DEVELOPMENT.md](./EXTENSION_DEVELOPMENT.md) - Extension development guide
- [MODULES.md](./MODULES.md) - Built-in module reference
- [TESTING.md](./TESTING.md) - Testing framework guide
