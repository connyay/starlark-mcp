# Starlark Extension Testing

This document describes the unit testing functionality for Starlark extensions.

## Overview

The Starlark MCP server supports convention-based unit testing for extensions. Test files are automatically discovered and can import functions from regular extension files to test them.

## Test File Convention

Test files follow these conventions:

- **File naming**: Test files must end with `_test.star` (e.g., `cat_facts_test.star`)
- **Test functions**: Test functions must start with `test_` (e.g., `def test_get_cat_fact():`)
- **Location**: Test files are placed in the same directory as the extensions they test

## Running Tests

Run all tests with the `--test` (or `-t`) flag:

```bash
starlark-mcp --test
starlark-mcp -t -e ./extensions
```

## Test File Structure

Test files can load functions from extension files using the `load()` statement:

```python
# cat_facts_test.star
load("cat_facts", "get_cat_fact")

def test_get_cat_fact_returns_content():
    """Test that get_cat_fact returns a valid response structure."""
    result = get_cat_fact({})

    testing.is_true(type(result) == "dict", "Result should be a dict")
    testing.contains(result, "content", "Result should have 'content' key")
```

## Testing Module

The `testing` module is automatically available in test files and provides assertion methods. This module is **only available when running tests** (with the `--test` flag) and is not exposed in normal server mode.

### testing.eq(actual, expected, message="")

Asserts that two values are equal.

```python
testing.eq(1 + 1, 2)
testing.eq("hello", "hello", "Strings should match")
```

### testing.ne(actual, expected, message="")

Asserts that two values are not equal.

```python
testing.ne(1, 2)
testing.ne("hello", "world", "Strings should differ")
```

### testing.is_true(value, message="")

Asserts that a value is truthy.

```python
testing.is_true(True)
testing.is_true(1 > 0, "1 should be greater than 0")
```

### testing.is_false(value, message="")

Asserts that a value is falsy.

```python
testing.is_false(False)
testing.is_false(1 > 2, "1 should not be greater than 2")
```

### testing.contains(container, item, message="")

Asserts that a container contains an item.

```python
testing.contains([1, 2, 3], 1)
testing.contains("concatenate", "cat")
testing.contains({"key": "value"}, "key")
```

### testing.fail(message)

Immediately fails the test with the given message.

```python
if some_condition:
    testing.fail("This should not happen")
```

## Example Test File

Here's a complete example testing the cat_facts extension:

```python
load("cat_facts", "get_cat_fact")

def test_get_cat_fact_returns_content():
    """Test that get_cat_fact returns a valid response structure."""
    result = get_cat_fact({})

    # Check result structure
    testing.is_true(type(result) == "dict", "Result should be a dict")
    testing.contains(result, "content", "Result should have 'content' key")

    # Check content structure
    content = result["content"]
    testing.is_true(type(content) == "list", "Content should be a list")
    testing.is_true(len(content) > 0, "Content should have at least one item")

    # Check first item
    first_item = content[0]
    testing.contains(first_item, "type", "Content item should have 'type' key")
    testing.contains(first_item, "text", "Content item should have 'text' key")
    testing.eq(first_item["type"], "text", "Content type should be 'text'")

def test_get_cat_fact_returns_non_empty_text():
    """Test that get_cat_fact returns non-empty text."""
    result = get_cat_fact({})
    fact = result["content"][0]["text"]

    testing.is_true(len(fact) > 0, "Cat fact should not be empty")
    testing.is_true(type(fact) == "string", "Cat fact should be a string")
```

## Test Output

When tests run, the output shows:

1. Test discovery information
2. Individual test results (✓ for pass, ✗ for fail)
3. Error messages for failed tests
4. Summary with total, passed, and failed counts

Example output:

```console
Discovering tests in: ./extensions
Found 1 test file(s)

Running tests from: cat_facts_test.star
  Found 2 test(s)
    ✓ test_get_cat_fact_returns_content
    ✓ test_get_cat_fact_returns_non_empty_text

============================================================
Test Summary
============================================================
✓ PASS cat_facts_test.star::test_get_cat_fact_returns_content
✓ PASS cat_facts_test.star::test_get_cat_fact_returns_non_empty_text
============================================================
Total: 2 | Passed: 2 | Failed: 0
============================================================
```

## Test Isolation

Each test function runs in isolation:

- Tests cannot affect each other's state
- Tests can import and call functions from extension files
- All standard Starlark modules (json, time, http, math, etc.) are available in tests
- The `testing` module is automatically available without importing

## Available Modules in Tests

Test files have access to all standard modules available in extensions:

- `testing` - Assertion methods (test-only)
- `math` - Mathematical functions
- `time` - Time utilities
- `env` - Environment variables
- `http` - HTTP requests
- `json` - JSON encoding/decoding
- `struct` - Structured data
- `debug` - Debugging utilities

## Server Mode

When running in normal server mode (without the `--test` flag):

- Test files are automatically filtered out and not loaded as extensions
- The `testing` module is not available
- This prevents test code from being exposed as MCP tools
