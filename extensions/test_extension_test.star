load("test_extension", "echo_handler", "structured_response_handler")

def test_echo_handler_default_message():
    """Test that echo_handler returns default message when no params."""
    result = echo_handler({})

    testing.is_true(type(result) == "dict", "Result should be a dict")
    testing.contains(result, "content", "Result should have 'content' key")
    testing.eq(result["content"][0]["text"], "Hello from test extension!")

def test_echo_handler_custom_message():
    """Test that echo_handler echoes custom message."""
    result = echo_handler({"message": "Custom message"})

    testing.eq(result["content"][0]["text"], "Custom message")

def test_structured_response_handler_returns_content():
    """Test that structured_response_handler returns valid content."""
    result = structured_response_handler({})

    testing.is_true(type(result) == "dict", "Result should be a dict")
    testing.contains(result, "content", "Result should have 'content' key")

    content = result["content"]
    testing.is_true(type(content) == "list", "Content should be a list")
    testing.is_true(len(content) > 0, "Content should have at least one item")

    first_item = content[0]
    testing.eq(first_item["type"], "text", "Content type should be 'text'")
    testing.contains(first_item["text"], "Greeting:", "Text should contain greeting")

def test_structured_response_handler_returns_structured_content():
    """Test that structured_response_handler returns structuredContent.

    The structuredContent field allows extensions to return machine-readable
    data alongside human-readable content. This is useful for programmatic
    consumption of tool results.
    """
    result = structured_response_handler({"name": "Test", "count": 3})

    testing.contains(result, "structuredContent", "Result should have 'structuredContent' key")

    data = result["structuredContent"]
    testing.is_true(type(data) == "dict", "structuredContent should be a dict")

    testing.eq(data["greeting"], "Hello, Test!", "Greeting should match")
    testing.eq(data["name"], "Test", "Name should match")
    testing.eq(data["count"], 3, "Count should match")

    testing.is_true(type(data["items"]) == "list", "Items should be a list")
    testing.eq(len(data["items"]), 3, "Should have 3 items")
    testing.eq(data["items"][0], "item_0", "First item should be item_0")
    testing.eq(data["items"][1], "item_1", "Second item should be item_1")
    testing.eq(data["items"][2], "item_2", "Third item should be item_2")

def test_structured_response_handler_defaults():
    """Test structured_response_handler with default parameters."""
    result = structured_response_handler({})

    data = result["structuredContent"]
    testing.eq(data["name"], "World", "Default name should be World")
    testing.eq(data["count"], 1, "Default count should be 1")
    testing.eq(len(data["items"]), 1, "Should have 1 item by default")
