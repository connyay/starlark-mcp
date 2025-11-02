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

    assert_true(type(result) == "dict", "Result should be a dict")
    assert_in("content", result, "Result should have 'content' key")
```

## Assertion Functions

The following assertion functions are available in test files:

### assert_eq(actual, expected, message="")
Asserts that two values are equal.

```python
assert_eq(1 + 1, 2)
assert_eq("hello", "hello", "Strings should match")
```

### assert_ne(actual, expected, message="")
Asserts that two values are not equal.

```python
assert_ne(1, 2)
assert_ne("hello", "world", "Strings should differ")
```

### assert_true(value, message="")
Asserts that a value is truthy.

```python
assert_true(True)
assert_true(1 > 0, "1 should be greater than 0")
```

### assert_false(value, message="")
Asserts that a value is falsy.

```python
assert_false(False)
assert_false(1 > 2, "1 should not be greater than 2")
```

### assert_in(item, container, message="")
Asserts that an item is in a container.

```python
assert_in(1, [1, 2, 3])
assert_in("cat", "concatenate")
```

### fail(message)
Immediately fails the test with the given message.

```python
if some_condition:
    fail("This should not happen")
```

## Example Test File

Here's a complete example testing the cat_facts extension:

```python
load("cat_facts", "get_cat_fact")

def test_get_cat_fact_returns_content():
    """Test that get_cat_fact returns a valid response structure."""
    result = get_cat_fact({})

    # Check result structure
    assert_true(type(result) == "dict", "Result should be a dict")
    assert_in("content", result, "Result should have 'content' key")

    # Check content structure
    content = result["content"]
    assert_true(type(content) == "list", "Content should be a list")
    assert_true(len(content) > 0, "Content should have at least one item")

    # Check first item
    first_item = content[0]
    assert_in("type", first_item, "Content item should have 'type' key")
    assert_in("text", first_item, "Content item should have 'text' key")
    assert_eq(first_item["type"], "text", "Content type should be 'text'")

def test_get_cat_fact_returns_non_empty_text():
    """Test that get_cat_fact returns non-empty text."""
    result = get_cat_fact({})
    fact = result["content"][0]["text"]

    assert_true(len(fact) > 0, "Cat fact should not be empty")
    assert_true(type(fact) == "string", "Cat fact should be a string")
```

## Test Output

When tests run, the output shows:

1. Test discovery information
2. Individual test results (✓ for pass, ✗ for fail)
3. Error messages for failed tests
4. Summary with total, passed, and failed counts

Example output:

```
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
- All standard Starlark modules (json, time, http, etc.) are available in tests
- Assertion functions are automatically available without importing

## Server Mode

When running in normal server mode (without the `--test` flag), test files are automatically filtered out and not loaded as extensions. This prevents test code from being exposed as MCP tools.
