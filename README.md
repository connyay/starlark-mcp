# Starlark MCP

A Starlark-powered MCP (Model Context Protocol) server that enables dynamic tool loading through Starlark scripting.

## Overview

This project combines the Starlark scripting language with MCP to create a flexible meta-server for dynamic tool loading. Extensions are written as `.star` files that define MCP tools using a simple, Python-like syntax.

## Features

- Minimal Starlark runtime optimized for MCP tool definitions
- JSON-RPC 2.0 protocol implementation
- Stdio transport for MCP communication
- Dynamic extension loading from `.star` files

## Building

```bash
cargo build --release
```

## Running

```bash
cargo run
```

The server will:

1. Load all `.star` extensions from the `./extensions/` directory
2. Start listening on stdin for MCP requests
3. Respond to MCP protocol messages on stdout

## Writing Extensions

Extensions are Starlark files that define tools. Here's the basic pattern:

```python
def my_tool_handler(params):
    # Your tool logic here
    return {
        "content": [{"type": "text", "text": "Tool result"}]
    }

def describe_extension():
    return Extension(
        name = "my_extension",
        version = "1.0.0",
        description = "My extension description",
        tools = [
            Tool(
                name = "my_tool",
                description = "What my tool does",
                handler = my_tool_handler,
            ),
        ],
    )
```

## Example: Cat Facts Extension

The included `cat_facts.star` extension demonstrates the pattern:

```python
def get_cat_fact(params):
    facts = [
        "Cats sleep 12-16 hours a day.",
        "A group of cats is called a 'clowder'.",
        # ... more facts
    ]

    index = time.now() % len(facts)

    return {
        "content": [{"type": "text", "text": facts[index]}]
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

## MCP Protocol Support

Currently supported MCP methods:

- `initialize` - Server initialization
- `initialized` - Initialization confirmation
- `tools/list` - List available tools
- `tools/call` - Execute a tool

## License

MIT
