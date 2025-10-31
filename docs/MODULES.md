# Starlark Modules

This document describes the built-in modules available to Starlark extensions in mcp-star.

## Core Modules

### `time`

Provides time-related functionality.

**Methods:**

- `time.now()` -> `int` - Returns the current Unix timestamp (seconds since epoch)

**Example:**

```python
current_time = time.now()
print("Current timestamp:", current_time)
```

---

### `env`

Provides access to environment variables.

**Methods:**

- `env.get(name: str, default: str = "") -> str` - Get an environment variable with optional default

**Example:**

```python
api_key = env.get("API_KEY", "")
if not api_key:
    return error_response("API_KEY not set")

# With default value
port = env.get("PORT", "8080")
```

---

### `exec`

Executes external commands with whitelist enforcement.

**Security:** Commands must be explicitly whitelisted in the extension's `allowed_exec` list.

**Methods:**

- `exec.run(command: str, args: list = []) -> dict` - Execute a command and return results

**Returns:**

```python
{
    "stdout": str,      # Standard output
    "stderr": str,      # Standard error
    "exit_code": int,   # Exit code (0 = success)
    "success": bool     # True if exit_code == 0
}
```

**Example:**

```python
# Extension must declare: allowed_exec = ["ls", "cat"]

result = exec.run("ls", ["-la", "/tmp"])
if result["success"]:
    print(result["stdout"])
else:
    print("Error:", result["stderr"])
```

**Whitelist Configuration:**

```python
def describe_extension():
    return Extension(
        name = "my_extension",
        version = "1.0.0",
        description = "My extension",
        allowed_exec = ["ls", "cat", "grep"],  # Only these commands allowed
        tools = [...]
    )
```

**Error Messages:**

- If no whitelist configured: `"Command 'X' cannot be executed: no exec whitelist configured for this extension. Add allowed_exec=['X'] to the Extension definition."`
- If command not whitelisted: `"Command 'X' is not in the allowed exec whitelist. Allowed commands: [...]"`

---

### `http`

Makes HTTP requests.

**Methods:**

- `http.get(url: str, headers: dict = {}) -> dict` - Make a GET request
- `http.post(url: str, body: str = "", headers: dict = {}) -> dict` - Make a POST request

**Returns:**

```python
{
    "status": int,      # HTTP status code
    "body": str,        # Response body
    "success": bool     # True if status 200-299
}
```

**Example:**

```python
# GET request
response = http.get("https://api.example.com/data", {
    "Authorization": "Bearer " + token
})

if response["success"]:
    data = json.decode(response["body"])
    print(data)

# POST request
response = http.post(
    "https://api.example.com/create",
    body = json.encode({"name": "test"}),
    headers = {"Content-Type": "application/json"}
)
```

---

### `sqlite`

Provides SQLite database operations.

**Methods:**

- `sqlite.list_tables(db_path: str) -> list[dict]` - List all tables in database
- `sqlite.describe_table(db_path: str, table: str) -> list[dict]` - Get table schema
- `sqlite.query(db_path: str, sql: str, params: list) -> list[dict]` - Execute SQL query

**Example:**

```python
# List tables
tables = sqlite.list_tables("/path/to/database.db")
for table in tables:
    print(table["name"])

# Describe table
columns = sqlite.describe_table("/path/to/database.db", "users")
for col in columns:
    print(col["name"], col["type"])

# Query with parameters
rows = sqlite.query(
    "/path/to/database.db",
    "SELECT * FROM users WHERE age > ?",
    [18]
)
```

---

### `postgres`

Provides PostgreSQL database operations.

**Methods:**

- `postgres.list_tables(connection_string: str) -> list[dict]` - List all tables
- `postgres.describe_table(connection_string: str, table: str) -> list[dict]` - Get table schema
- `postgres.query(connection_string: str, sql: str, params: list) -> list[dict]` - Execute SQL query

**Example:**

```python
conn_str = "postgresql://user:pass@localhost:5432/dbname"

# List tables
tables = postgres.list_tables(conn_str)

# Query
rows = postgres.query(
    conn_str,
    "SELECT * FROM users WHERE email = $1",
    ["user@example.com"]
)
```

---

## MCP Types

### `Extension`

Defines an MCP extension.

**Constructor:**

```python
Extension(
    name: str,
    version: str,
    description: str,
    allowed_exec: list[str] = [],  # Optional: whitelisted commands for exec.run()
    tools: list[Tool]
)
```

**Example:**

```python
def describe_extension():
    return Extension(
        name = "my_extension",
        version = "1.0.0",
        description = "A sample extension",
        allowed_exec = ["ls", "cat"],
        tools = [
            Tool(...)
        ]
    )
```

---

### `Tool`

Defines an MCP tool.

**Constructor:**

```python
Tool(
    name: str,
    description: str,
    parameters: list[ToolParameter] = [],
    handler: function
)
```

**Example:**

```python
Tool(
    name = "greet",
    description = "Greet a user by name",
    parameters = [
        ToolParameter(
            name = "username",
            param_type = "string",
            required = True,
            description = "The user's name"
        )
    ],
    handler = greet_handler
)
```

---

### `ToolParameter`

Defines a tool parameter.

**Constructor:**

```python
ToolParameter(
    name: str,
    param_type: str,  # "string", "integer", "number", "boolean"
    required: bool,
    default: str = None,
    description: str
)
```

**Example:**

```python
ToolParameter(
    name = "limit",
    param_type = "integer",
    required = False,
    default = "10",
    description = "Maximum number of results"
)
```

---

## Standard Library

### `json`

Built-in JSON support (from Starlark standard library).

**Methods:**

- `json.encode(obj) -> str` - Encode object to JSON string
- `json.decode(str) -> obj` - Decode JSON string to object

**Example:**

```python
# Encode
data = {"name": "Alice", "age": 30}
json_str = json.encode(data)

# Decode
obj = json.decode('{"key": "value"}')
print(obj["key"])
```

---

## Writing Tool Handlers

Tool handlers receive a `params` dict and must return a result dict.

**Handler Signature:**

```python
def my_handler(params):
    # params is a dict with tool arguments
    value = params.get("param_name", default_value)

    # Return MCP tool result
    return {
        "content": [
            {"type": "text", "text": "Result text"}
        ],
        "isError": False  # Optional, default False
    }
```

**Error Response:**

```python
def error_response(message):
    return {
        "content": [{"type": "text", "text": "Error: " + message}],
        "isError": True
    }
```

**Complete Example:**

```python
def greet_handler(params):
    name = params.get("name", "")

    if not name:
        return error_response("name parameter is required")

    greeting = "Hello, " + name + "!"

    return {
        "content": [{"type": "text", "text": greeting}]
    }
```

---

## Security Best Practices

1. **Always validate inputs:**

   ```python
   if not db_path:
       return error_response("db_path is required")

   if not is_valid_identifier(table_name):
       return error_response("Invalid table name")
   ```

2. **Use exec whitelist:**

   ```python
   # In Extension definition
   allowed_exec = ["sqlite3", "ls"]  # Only allow specific commands
   ```

3. **Sanitize SQL/command arguments:**

   ```python
   # Use parameterized queries
   sqlite.query(db_path, "SELECT * FROM users WHERE id = ?", [user_id])

   # Validate identifiers
   def is_valid_identifier(name):
       for i in range(len(name)):
           char = name[i]
           if not (char.isalnum() or char == "_"):
               return False
       return not name[0].isdigit()
   ```

4. **Check command results:**

   ```python
   result = exec.run("ls", [path])
   if not result["success"]:
       return error_response("Command failed: " + result["stderr"])
   ```

---

## Example Extension

```python
def list_files(params):
    path = params.get("path", ".")

    result = exec.run("ls", ["-la", path])

    if not result["success"]:
        return {
            "content": [{"type": "text", "text": "Error: " + result["stderr"]}],
            "isError": True
        }

    return {
        "content": [{"type": "text", "text": result["stdout"]}]
    }

def describe_extension():
    return Extension(
        name = "file_browser",
        version = "1.0.0",
        description = "Browse filesystem",
        allowed_exec = ["ls"],
        tools = [
            Tool(
                name = "list_files",
                description = "List files in a directory",
                parameters = [
                    ToolParameter(
                        name = "path",
                        param_type = "string",
                        required = False,
                        default = ".",
                        description = "Directory path to list"
                    )
                ],
                handler = list_files
            )
        ]
    )
```
