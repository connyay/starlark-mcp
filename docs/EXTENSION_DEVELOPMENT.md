# Extension Development Guide

This guide covers best practices, common patterns, and practical examples for developing starlark-mcp extensions.

## Table of Contents

- [Extension Anatomy](#extension-anatomy)
- [Quick Start](#quick-start)
- [Common Patterns](#common-patterns)
- [Best Practices](#best-practices)
- [Anti-Patterns](#anti-patterns)
- [Error Handling](#error-handling)
- [Testing Extensions](#testing-extensions)
- [Debugging Tips](#debugging-tips)

## Extension Anatomy

Every extension is a `.star` file with two required parts:

### 1. Handler Functions

Functions that implement tool logic and return MCP-compliant responses:

```starlark
def my_tool_handler(params):
    """Tool implementation"""
    # Extract parameters
    name = params.get("name", "default")

    # Do work here...

    # Return MCP response
    return {
        "content": [{"type": "text", "text": "Result"}],
        "isError": False,  # Optional, defaults to False
    }
```

### 2. Extension Descriptor

The `describe_extension()` function that declares metadata and tools:

```starlark
def describe_extension():
    """Extension metadata and tool registration"""
    return Extension(
        name = "my_extension",
        version = "1.0.0",
        description = "What this extension does",
        allowed_exec = ["cmd1", "cmd2"],  # Optional: commands for exec.run()
        tools = [
            Tool(
                name = "my_tool",
                description = "What this tool does",
                parameters = [
                    ToolParameter(
                        name = "param_name",
                        param_type = "string",  # string, integer, number, boolean
                        required = True,
                        description = "What this parameter does"
                    ),
                ],
                handler = my_tool_handler,
            ),
        ],
    )
```

## Quick Start

Create a minimal extension in `extensions/hello.star`:

```starlark
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

Start the server with hot reload:

```bash
starlark-mcp --extensions-dir ./extensions
```

The extension will be automatically loaded and available to MCP clients.

## Common Patterns

### Pattern 1: Simple Self-Contained Extension

**Use case**: No external dependencies, pure computation or static data.

**Example**: `extensions/cat_facts.star`

```starlark
def get_cat_fact(params):
    """Returns a random cat fact"""
    facts = [
        "Cats sleep 12-16 hours a day.",
        "A group of cats is called a 'clowder'.",
        "Cats have over 30 muscles in each ear.",
    ]

    # Pseudo-random selection using timestamp
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

**Key Points**:

- No parameters needed
- Uses built-in `time` module
- No external calls
- Fast and reliable

### Pattern 2: HTTP API Wrapper

**Use case**: Integrate with REST APIs.

**Example**: Weather API integration (see `extensions/weather.star`)

```starlark
# Define constants
HEADERS = {
    "User-Agent": "my-extension/1.0",
    "Accept": "application/json",
}

def get_weather(params):
    """Get weather for a location"""
    city = params.get("city", "")

    # Validate parameters
    if not city:
        return error_response("city parameter is required")

    # Make HTTP request
    response = http.get(
        url = "https://api.example.com/weather?city={}".format(city),
        headers = HEADERS,
    )

    # Check HTTP status
    if response.get("status_code", 0) != 200:
        return error_response("API request failed: {}".format(
            response.get("body", "Unknown error")
        ))

    # Parse JSON response
    data = parse_json(response)
    if not data:
        return error_response("Failed to parse API response")

    # Extract relevant data
    temp = data.get("temperature", "?")
    conditions = data.get("conditions", "unknown")

    # Format output
    output = "Weather in {}:\n".format(city)
    output += "Temperature: {}°F\n".format(temp)
    output += "Conditions: {}\n".format(conditions)

    return {
        "content": [{"type": "text", "text": output}],
    }

# Helper function
def parse_json(response):
    """Parse JSON from HTTP response"""
    if response.get("json"):
        return response["json"]
    body = response.get("body", "")
    if body.strip():
        return json.decode(body)
    return None

def error_response(message):
    """Create standardized error response"""
    return {
        "content": [{"type": "text", "text": "Error: " + message}],
        "isError": True,
    }

def describe_extension():
    return Extension(
        name = "weather",
        version = "1.0.0",
        description = "Weather information",
        tools = [
            Tool(
                name = "get_weather",
                description = "Get weather for a city",
                parameters = [
                    ToolParameter(
                        name = "city",
                        param_type = "string",
                        required = True,
                        description = "City name",
                    ),
                ],
                handler = get_weather,
            ),
        ],
    )
```

**Key Points**:

- Validate parameters early
- Check HTTP status codes
- Parse JSON responses safely
- Format output for readability
- Use helper functions for common operations
- Provide clear error messages

### Pattern 3: CLI Tool Wrapper

**Use case**: Integrate existing command-line tools.

**Example**: GitHub CLI integration (see `extensions/github.star`)

```starlark
def list_repos(params):
    """List GitHub repositories"""
    org = params.get("org", "")
    limit = params.get("limit", "10")

    if not org:
        return error_response("org parameter is required")

    # Execute CLI command
    result = exec.run("gh", [
        "repo",
        "list",
        org,
        "--limit",
        str(limit),
        "--json",
        "name,description,stars",
    ])

    # Check execution success
    if not result["success"]:
        return error_response("gh command failed: " + result["stderr"])

    # Parse JSON output
    repos = json.decode(result["stdout"])

    # Format output
    output = "Repositories in {}:\n".format(org)
    output += "=" * 50 + "\n\n"

    for repo in repos:
        name = repo.get("name", "?")
        desc = repo.get("description", "No description")
        stars = repo.get("stars", 0)
        output += "{} ({} stars)\n".format(name, stars)
        output += "  {}\n\n".format(desc)

    return {
        "content": [{"type": "text", "text": output}],
    }

def error_response(message):
    """Create standardized error response"""
    return {
        "content": [{"type": "text", "text": "Error: " + message}],
        "isError": True,
    }

def describe_extension():
    return Extension(
        name = "github",
        version = "1.0.0",
        description = "GitHub CLI integration",
        allowed_exec = ["gh"],  # Required for exec.run()
        tools = [
            Tool(
                name = "list_repos",
                description = "List repositories for an organization",
                parameters = [
                    ToolParameter(
                        name = "org",
                        param_type = "string",
                        required = True,
                        description = "GitHub organization name",
                    ),
                    ToolParameter(
                        name = "limit",
                        param_type = "string",
                        required = False,
                        default = "10",
                        description = "Maximum repositories to list",
                    ),
                ],
                handler = list_repos,
            ),
        ],
    )
```

**Key Points**:

- Declare `allowed_exec` with commands you'll use
- Check `result["success"]` before using output
- Use `result["stderr"]` for error messages
- Prefer JSON output from CLI tools when available
- Convert numeric parameters to strings for CLI args

### Pattern 4: Database Integration

**Use case**: Query and manipulate databases.

**Example**: SQLite integration

```starlark
def query_database(params):
    """Execute a SQL query"""
    db_path = params.get("db_path", "")
    query = params.get("query", "")

    if not db_path:
        return error_response("db_path parameter is required")

    if not query:
        return error_response("query parameter is required")

    # Validate query is read-only (security)
    query_lower = query.lower().strip()
    if not query_lower.startswith("select"):
        return error_response("Only SELECT queries are allowed")

    # Execute query
    try:
        results = sqlite.query(db_path, query, [])
    except Exception as e:
        return error_response("Query failed: " + str(e))

    # Format results as table
    if not results:
        return {
            "content": [{"type": "text", "text": "Query returned no results"}],
        }

    # Get column names from first row keys
    columns = list(results[0].keys()) if results else []

    # Format as markdown table
    output = "Query Results:\n\n"
    output += "| " + " | ".join(columns) + " |\n"
    output += "| " + " | ".join(["---"] * len(columns)) + " |\n"

    for row in results:
        values = [str(row.get(col, "")) for col in columns]
        output += "| " + " | ".join(values) + " |\n"

    output += "\nTotal rows: {}\n".format(len(results))

    return {
        "content": [{"type": "text", "text": output}],
    }

def error_response(message):
    """Create standardized error response"""
    return {
        "content": [{"type": "text", "text": "Error: " + message}],
        "isError": True,
    }

def describe_extension():
    return Extension(
        name = "sqlite",
        version = "1.0.0",
        description = "SQLite database tools",
        tools = [
            Tool(
                name = "query_database",
                description = "Execute a SELECT query on a SQLite database",
                parameters = [
                    ToolParameter(
                        name = "db_path",
                        param_type = "string",
                        required = True,
                        description = "Path to SQLite database file",
                    ),
                    ToolParameter(
                        name = "query",
                        param_type = "string",
                        required = True,
                        description = "SQL SELECT query to execute",
                    ),
                ],
                handler = query_database,
            ),
        ],
    )
```

**Key Points**:

- Validate database paths exist
- Restrict dangerous operations (INSERT, DELETE, DROP)
- Handle query errors gracefully
- Format results for readability (tables, JSON, etc.)
- Consider query parameter safety

### Pattern 5: Multi-Tool Extension

**Use case**: Related tools that share configuration or helpers.

```starlark
# Shared configuration
def get_config():
    """Get extension configuration from environment"""
    return {
        "api_key": env.get("MY_API_KEY", ""),
        "api_url": env.get("MY_API_URL", "https://api.example.com"),
    }

# Shared helper functions
def make_api_request(endpoint, method="GET", body=None):
    """Make authenticated API request"""
    config = get_config()

    if not config["api_key"]:
        return {"error": "MY_API_KEY environment variable not set", "success": False}

    headers = {
        "Authorization": "Bearer {}".format(config["api_key"]),
        "Content-Type": "application/json",
    }

    url = "{}/{}".format(config["api_url"], endpoint)

    if method == "GET":
        response = http.get(url, headers)
    elif method == "POST":
        response = http.post(url, body, headers)
    else:
        return {"error": "Unsupported HTTP method", "success": False}

    if response.get("status_code", 0) != 200:
        return {"error": response.get("body", "Unknown error"), "success": False}

    return {"data": response.get("json", {}), "success": True}

# Tool implementations
def list_items(params):
    """List items from API"""
    result = make_api_request("items")

    if not result["success"]:
        return error_response(result["error"])

    items = result["data"].get("items", [])
    output = "Items:\n"
    for item in items:
        output += "- {}\n".format(item.get("name", "?"))

    return {
        "content": [{"type": "text", "text": output}],
    }

def get_item(params):
    """Get specific item from API"""
    item_id = params.get("id", "")

    if not item_id:
        return error_response("id parameter is required")

    result = make_api_request("items/{}".format(item_id))

    if not result["success"]:
        return error_response(result["error"])

    item = result["data"]
    output = "Item Details:\n"
    output += "ID: {}\n".format(item.get("id", "?"))
    output += "Name: {}\n".format(item.get("name", "?"))

    return {
        "content": [{"type": "text", "text": output}],
    }

def create_item(params):
    """Create new item via API"""
    name = params.get("name", "")

    if not name:
        return error_response("name parameter is required")

    body = json.encode({"name": name})
    result = make_api_request("items", method="POST", body=body)

    if not result["success"]:
        return error_response(result["error"])

    return {
        "content": [{"type": "text", "text": "Item created successfully"}],
    }

def error_response(message):
    """Create standardized error response"""
    return {
        "content": [{"type": "text", "text": "Error: " + message}],
        "isError": True,
    }

def describe_extension():
    return Extension(
        name = "my_api",
        version = "1.0.0",
        description = "API integration with multiple tools",
        tools = [
            Tool(
                name = "list_items",
                description = "List all items",
                handler = list_items,
            ),
            Tool(
                name = "get_item",
                description = "Get details for a specific item",
                parameters = [
                    ToolParameter(
                        name = "id",
                        param_type = "string",
                        required = True,
                        description = "Item ID",
                    ),
                ],
                handler = get_item,
            ),
            Tool(
                name = "create_item",
                description = "Create a new item",
                parameters = [
                    ToolParameter(
                        name = "name",
                        param_type = "string",
                        required = True,
                        description = "Item name",
                    ),
                ],
                handler = create_item,
            ),
        ],
    )
```

**Key Points**:

- Share configuration logic across tools
- Extract common API interaction patterns
- Reuse helper functions for consistency
- Group related tools in one extension
- Keep each tool handler focused on its specific task

## Best Practices

### 1. Parameter Validation

**Always validate required parameters early:**

```starlark
def my_tool(params):
    # Validate required parameters
    if not params.get("required_param"):
        return error_response("required_param is required")

    # Validate parameter types/formats
    port = params.get("port", "")
    if port and not port.isdigit():
        return error_response("port must be a number")

    # Proceed with validated data
    # ...
```

### 2. Error Handling

**Use consistent error response format:**

```starlark
def error_response(message):
    """Standardized error responses"""
    return {
        "content": [{"type": "text", "text": "Error: " + message}],
        "isError": True,
    }
```

**Check all external calls:**

```starlark
# HTTP requests
response = http.get(url, headers)
if response.get("status_code", 0) != 200:
    return error_response("Request failed")

# Exec calls
result = exec.run("cmd", args)
if not result["success"]:
    return error_response(result["stderr"])

# Database calls
try:
    results = sqlite.query(db_path, query, [])
except Exception as e:
    return error_response("Database error: " + str(e))
```

### 3. Output Formatting

**Make output human-readable:**

```starlark
def format_list_output(items):
    """Format list as readable text"""
    output = "Found {} items:\n\n".format(len(items))

    for i, item in enumerate(items, 1):
        output += "{}. {}\n".format(i, item["name"])
        output += "   {}\n".format(item["description"])
        output += "\n"

    return output
```

**Use markdown for structured data:**

```starlark
def format_table(rows, columns):
    """Format data as markdown table"""
    output = "| " + " | ".join(columns) + " |\n"
    output += "| " + " | ".join(["---"] * len(columns)) + " |\n"

    for row in rows:
        values = [str(row.get(col, "")) for col in columns]
        output += "| " + " | ".join(values) + " |\n"

    return output
```

### 4. Configuration

**Use environment variables for secrets:**

```starlark
def get_api_key():
    """Get API key from environment"""
    api_key = env.get("MY_API_KEY", "")
    if not api_key:
        # Return error or default behavior
        return None
    return api_key
```

**Provide sensible defaults:**

```starlark
def get_config():
    """Get configuration with defaults"""
    return {
        "timeout": env.get("MY_TIMEOUT", "30"),
        "base_url": env.get("MY_BASE_URL", "https://api.example.com"),
        "max_results": env.get("MY_MAX_RESULTS", "100"),
    }
```

### 5. Helper Functions

**Extract reusable logic:**

```starlark
def parse_json(response):
    """Parse JSON from HTTP response"""
    if response.get("json"):
        return response["json"]
    body = response.get("body", "")
    if body.strip():
        return json.decode(body)
    return None

def truncate_text(text, max_length=500):
    """Truncate long text with ellipsis"""
    if len(text) <= max_length:
        return text
    return text[:max_length] + "..."

def format_timestamp(ts):
    """Format Unix timestamp as readable date"""
    # Simple formatting since Starlark is limited
    return "Timestamp: {}".format(ts)
```

### 6. Documentation

**Add docstrings to functions:**

```starlark
def get_weather(params):
    """
    Get current weather for a city.

    Args:
        params: Dict with 'city' parameter

    Returns:
        MCP response with weather information
    """
    # Implementation...
```

**Comment complex logic:**

```starlark
def calculate_something(value):
    # Use modulo for pseudo-random selection since Starlark
    # doesn't have a built-in random number generator
    index = time.now() % len(items)
    return items[index]
```

### 7. Testing

**Write tests for your extensions:**

Create `extensions/my_extension_test.star`:

```starlark
load("my_extension.star", "parse_json", "format_table")

def test_parse_json_with_json_field():
    """Test JSON parsing when json field is present"""
    response = {"json": {"key": "value"}}
    result = parse_json(response)
    testing.eq(result["key"], "value", "Should parse json field")

def test_parse_json_with_body():
    """Test JSON parsing from body field"""
    response = {"body": '{"key": "value"}'}
    result = parse_json(response)
    testing.eq(result["key"], "value", "Should parse body as JSON")

def test_format_table():
    """Test table formatting"""
    rows = [{"name": "Alice", "age": "30"}]
    columns = ["name", "age"]
    result = format_table(rows, columns)
    testing.contains(result, "Alice", "Should contain row data")
    testing.contains(result, "---", "Should contain separator")
```

Run tests:

```bash
starlark-mcp --test
```

See [TESTING.md](./TESTING.md) for complete testing guide.

## Anti-Patterns

### ❌ Missing Parameter Validation

**Bad:**

```starlark
def bad_tool(params):
    # Assumes parameter exists
    city = params["city"]  # Will fail if missing!
    # ...
```

**Good:**

```starlark
def good_tool(params):
    city = params.get("city", "")
    if not city:
        return error_response("city parameter is required")
    # ...
```

### ❌ Ignoring Error Conditions

**Bad:**

```starlark
def bad_tool(params):
    response = http.get(url, headers)
    # Assumes request succeeded
    data = response["json"]
    # ...
```

**Good:**

```starlark
def good_tool(params):
    response = http.get(url, headers)
    if response.get("status_code", 0) != 200:
        return error_response("API request failed")

    data = parse_json(response)
    if not data:
        return error_response("Failed to parse response")
    # ...
```

### ❌ Hardcoded Secrets

**Bad:**

```starlark
API_KEY = "sk-1234567890abcdef"  # Never hardcode secrets!

def bad_tool(params):
    headers = {"Authorization": "Bearer " + API_KEY}
    # ...
```

**Good:**

```starlark
def good_tool(params):
    api_key = env.get("MY_API_KEY", "")
    if not api_key:
        return error_response("MY_API_KEY environment variable not set")

    headers = {"Authorization": "Bearer " + api_key}
    # ...
```

### ❌ Unformatted Output

**Bad:**

```starlark
def bad_tool(params):
    items = get_items()
    # Returns raw data structure
    return {
        "content": [{"type": "text", "text": str(items)}],
    }
    # Output: [{'id': 1, 'name': 'foo'}, {'id': 2, 'name': 'bar'}]
```

**Good:**

```starlark
def good_tool(params):
    items = get_items()

    output = "Found {} items:\n\n".format(len(items))
    for item in items:
        output += "- {} (ID: {})\n".format(item["name"], item["id"])

    return {
        "content": [{"type": "text", "text": output}],
    }
    # Output:
    # Found 2 items:
    #
    # - foo (ID: 1)
    # - bar (ID: 2)
```

### ❌ Missing allowed_exec Declaration

**Bad:**

```starlark
def bad_tool(params):
    # This will fail at runtime!
    result = exec.run("gh", ["--version"])
    # ...

def describe_extension():
    return Extension(
        name = "bad",
        # Missing allowed_exec!
        tools = [Tool(name = "bad_tool", handler = bad_tool)],
    )
```

**Good:**

```starlark
def good_tool(params):
    result = exec.run("gh", ["--version"])
    # ...

def describe_extension():
    return Extension(
        name = "good",
        allowed_exec = ["gh"],  # Explicitly declare commands
        tools = [Tool(name = "good_tool", handler = good_tool)],
    )
```

### ❌ Overly Complex Handlers

**Bad:**

```starlark
def bad_tool(params):
    # 200 lines of complex logic
    # Difficult to test and maintain
    # ...
```

**Good:**

```starlark
def validate_params(params):
    # Extract validation logic
    # ...

def fetch_data(endpoint):
    # Extract API logic
    # ...

def format_output(data):
    # Extract formatting logic
    # ...

def good_tool(params):
    # Compose from smaller functions
    if not validate_params(params):
        return error_response("Invalid parameters")

    data = fetch_data(params["endpoint"])
    return {
        "content": [{"type": "text", "text": format_output(data)}],
    }
```

## Error Handling

### Standard Error Response Pattern

Always use this pattern for errors:

```starlark
def error_response(message):
    """Create standardized error response"""
    return {
        "content": [{"type": "text", "text": "Error: " + message}],
        "isError": True,
    }
```

### Common Error Scenarios

```starlark
def comprehensive_tool(params):
    # 1. Parameter validation errors
    if not params.get("required_param"):
        return error_response("required_param is required")

    # 2. HTTP errors
    response = http.get(url, headers)
    if response.get("status_code", 0) != 200:
        return error_response("HTTP request failed: status {}".format(
            response.get("status_code", "unknown")
        ))

    # 3. Parsing errors
    data = parse_json(response)
    if not data:
        return error_response("Failed to parse API response")

    # 4. Missing expected data
    if not data.get("expected_field"):
        return error_response("API response missing expected_field")

    # 5. Exec errors
    result = exec.run("cmd", args)
    if not result["success"]:
        return error_response("Command failed: {}".format(result["stderr"]))

    # 6. Database errors (use try/except)
    try:
        rows = sqlite.query(db_path, query, [])
    except Exception as e:
        return error_response("Database query failed: {}".format(str(e)))

    # Success path
    return {
        "content": [{"type": "text", "text": "Success!"}],
    }
```

## Testing Extensions

Create test files alongside your extensions:

**File**: `extensions/my_extension_test.star`

```starlark
load("my_extension.star", "function_to_test", "another_function")

def test_basic_functionality():
    """Test that function works with valid input"""
    result = function_to_test({"param": "value"})
    testing.eq(result["content"][0]["text"], "expected", "Should return expected text")

def test_error_handling():
    """Test that function handles errors"""
    result = function_to_test({})  # Missing required param
    testing.eq(result["isError"], True, "Should return error")
    testing.contains(result["content"][0]["text"], "Error", "Should contain error message")

def test_helper_function():
    """Test helper function in isolation"""
    result = another_function("input")
    testing.eq(result, "expected_output", "Helper should transform input correctly")
```

Run all tests:

```bash
starlark-mcp --test
```

See [TESTING.md](./TESTING.md) for complete testing documentation.

## Debugging Tips

### 1. Use Print-Style Debugging

Starlark doesn't have a debugger, but you can include debug info in responses:

```starlark
def debug_tool(params):
    # Include debug information in output
    output = "Debug Info:\n"
    output += "Params: {}\n".format(params)
    output += "Env API_KEY set: {}\n".format(bool(env.get("MY_API_KEY", "")))

    # Your logic here...

    output += "\nResult: ...\n"

    return {
        "content": [{"type": "text", "text": output}],
    }
```

### 2. Test Helper Functions Separately

Create test files to validate helper functions:

```starlark
# my_extension_test.star
load("my_extension.star", "parse_json", "format_output")

def test_parse_json():
    """Test JSON parsing logic"""
    response = {"body": '{"key": "value"}'}
    result = parse_json(response)
    testing.eq(result["key"], "value", "Should parse JSON correctly")
```

### 3. Check Hot Reload

If changes aren't appearing:

```bash
# Check server logs for reload errors
# The server prints "Reloaded extension: <name>" on success

# Verify file name doesn't end in _test.star (excluded from server mode)
ls extensions/

# Restart server if hot reload fails
# Ctrl+C and restart
starlark-mcp --extensions-dir ./extensions
```

### 4. Validate Extension Metadata

Extension loading can fail silently if metadata is invalid:

```starlark
def describe_extension():
    return Extension(
        name = "my_ext",  # Must be valid
        version = "1.0.0",  # Must be present
        description = "Description",  # Must be present
        tools = [  # Must be a list
            Tool(
                name = "tool_name",  # Must be valid
                description = "...",  # Must be present
                parameters = [  # Can be empty list
                    # Parameters must have valid param_type
                ],
                handler = handler_func,  # Must reference actual function
            ),
        ],
    )
```

### 5. Test Exec Commands Manually

Before using `exec.run()`, test commands manually:

```bash
# Test that command works in shell first
gh repo list myorg --limit 5 --json name

# Then use in extension
result = exec.run("gh", ["repo", "list", "myorg", "--limit", "5", "--json", "name"])
```

### 6. Check Exec Whitelist

If exec commands fail with "not allowed":

```starlark
def describe_extension():
    return Extension(
        name = "my_ext",
        allowed_exec = ["gh", "docker", "kubectl"],  # Add all commands you use
        tools = [...],
    )
```

### 7. Validate JSON Parsing

JSON parsing can fail silently:

```starlark
def safe_parse_json(text):
    """Parse JSON with debug info"""
    if not text:
        print("Empty JSON text")  # Will appear in tool output
        return None

    try:
        return json.decode(text)
    except Exception as e:
        print("JSON parse error: " + str(e))
        print("Input was: " + text[:100])  # Show first 100 chars
        return None
```

### 8. Use MCP Inspector Tools

Test your extensions with MCP client tools:

```bash
# Example: Use Claude Desktop's developer tools
# to inspect tool calls and responses

# Or use MCP CLI tools for testing
```

### 9. Common Issues Checklist

- [ ] Extension file saved in `extensions/` directory
- [ ] File doesn't end in `_test.star`
- [ ] `describe_extension()` function exists and returns Extension
- [ ] All handler functions exist and match Tool definitions
- [ ] `allowed_exec` includes all commands used in `exec.run()`
- [ ] Parameters use valid `param_type` values
- [ ] Error responses use `isError: True`
- [ ] Environment variables are set if required

## Advanced Topics

### Dynamic Tool Registration

You cannot dynamically create tools at runtime. All tools must be defined in `describe_extension()`. However, you can make tools flexible:

```starlark
def generic_api_call(params):
    """Generic tool that calls different endpoints"""
    endpoint = params.get("endpoint", "")
    method = params.get("method", "GET")

    # Tool adapts behavior based on parameters
    # instead of having separate tools for each endpoint
    # ...
```

### Sharing Code Between Extensions

Use Starlark's `load()` function in test files, but extensions run independently. To share code, you can:

1. **Copy-paste common helpers** into each extension (simple)
2. **Create a "library" extension** that other extensions can call via MCP (advanced)

### Security Considerations

Quick checklist:

- Use environment variables for secrets
- Validate and sanitize all parameters
- Restrict database operations (read-only when possible)
- Use exec whitelist judiciously
- Be careful with user-provided file paths

## Next Steps

- Read [MODULES.md](./MODULES.md) for complete module reference
- Read [TESTING.md](./TESTING.md) for testing framework details
- Explore `extensions/` directory for real-world examples
- Check [ARCHITECTURE.md](./ARCHITECTURE.md) to understand internals
