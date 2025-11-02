def test_assertion_functions():
    """Test various assertion functions to demonstrate their usage."""
    testing.eq(2, 1 + 1, "1 + 1 should equal 2")
    testing.eq("hello", "hello")

    testing.ne(2, 1, "1 should not equal 2")
    testing.ne("world", "hello")

    testing.is_true(True)
    testing.is_true(1 > 0)
    testing.is_true(len([1, 2, 3]) == 3)

    testing.is_false(False)
    testing.is_false(1 > 2)
    testing.is_false(len([]) > 0)

    testing.contains([1, 2, 3], 1)
    testing.contains(["a", "b", "c"], "a")
    testing.contains("concatenate", "cat")
