def echo_handler(params):
    """Test handler that echoes back the input."""
    message = params.get("message", "Hello from test extension!")

    return {
        "content": [{"type": "text", "text": message}],
    }

def structured_response_handler(params):
    """Handler that returns both content and structured data."""
    name = params.get("name", "World")
    count = params.get("count", 1)

    data = {
        "greeting": "Hello, " + name + "!",
        "name": name,
        "count": count,
        "items": ["item_" + str(i) for i in range(count)],
    }

    text = "Greeting: {}\nItems: {}".format(data["greeting"], ", ".join(data["items"]))

    return {
        "content": [{"type": "text", "text": text}],
        "structuredContent": data,
    }

def describe_extension():
    """Describes the test extension."""
    return Extension(
        name = "test_extension",
        version = "1.0.0",
        description = "A simple test extension for unit testing",
        tools = [
            Tool(
                name = "test_tool",
                description = "A test tool that echoes back messages",
                handler = echo_handler,
                parameters = [
                    ToolParameter(
                        name = "message",
                        param_type = "string",
                        required = False,
                        description = "The message to echo back",
                    ),
                ],
            ),
            Tool(
                name = "structured_response_tool",
                description = "A test tool that returns structured data alongside content",
                handler = structured_response_handler,
                parameters = [
                    ToolParameter(
                        name = "name",
                        param_type = "string",
                        required = False,
                        description = "Name to greet",
                    ),
                    ToolParameter(
                        name = "count",
                        param_type = "integer",
                        required = False,
                        description = "Number of items to generate",
                    ),
                ],
            ),
        ],
    )
