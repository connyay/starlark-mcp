def test_handler(params):
    """Test handler that echoes back the input."""
    message = params.get("message", "Hello from test extension!")

    return {
        "content": [{"type": "text", "text": message}],
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
                handler = test_handler,
                parameters = [
                    ToolParameter(
                        name = "message",
                        param_type = "string",
                        required = False,
                        description = "The message to echo back",
                    ),
                ],
            ),
        ],
    )
