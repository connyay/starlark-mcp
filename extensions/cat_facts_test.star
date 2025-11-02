load("cat_facts", "get_cat_fact")

def test_get_cat_fact_returns_content():
    """Test that get_cat_fact returns a valid response structure."""
    result = get_cat_fact({})

    # Check that result is a dict
    assert_true(type(result) == "dict", "Result should be a dict")

    # Check that result has "content" key
    assert_in("content", result, "Result should have 'content' key")

    # Check that content is a list
    content = result["content"]
    assert_true(type(content) == "list", "Content should be a list")

    # Check that content has at least one item
    assert_true(len(content) > 0, "Content should have at least one item")

    # Check the first item structure
    first_item = content[0]
    assert_in("type", first_item, "Content item should have 'type' key")
    assert_in("text", first_item, "Content item should have 'text' key")
    assert_eq(first_item["type"], "text", "Content type should be 'text'")

def test_get_cat_fact_returns_valid_fact():
    """Test that get_cat_fact returns one of the known facts."""
    result = get_cat_fact({})
    fact = result["content"][0]["text"]

    # List of valid facts (should match the ones in cat_facts.star)
    valid_facts = [
        "Cats sleep 12-16 hours a day.",
        "A group of cats is called a 'clowder'.",
        "Cats have over 30 muscles in each ear.",
        "The first cat in space was French and named Felicette.",
        "Cats can rotate their ears 180 degrees.",
        "A cat's purr vibrates at a frequency of 25-150 Hz.",
        "Cats have a third eyelid called a haw.",
        "A cat's nose print is unique, like a human fingerprint.",
        "Cats can jump up to six times their length.",
        "Cats have whiskers on the backs of their front legs.",
    ]

    # Check that the returned fact is one of the valid facts
    assert_in(fact, valid_facts, "Returned fact should be one of the known cat facts")

def test_get_cat_fact_returns_non_empty_text():
    """Test that get_cat_fact returns non-empty text."""
    result = get_cat_fact({})
    fact = result["content"][0]["text"]

    # Check that fact is not empty
    assert_true(len(fact) > 0, "Cat fact should not be empty")

    # Check that fact is a string
    assert_true(type(fact) == "string", "Cat fact should be a string")

def test_assertion_functions():
    """Test various assertion functions to demonstrate their usage."""
    # assert_eq
    assert_eq(1 + 1, 2, "1 + 1 should equal 2")
    assert_eq("hello", "hello")

    # assert_ne
    assert_ne(1, 2, "1 should not equal 2")
    assert_ne("hello", "world")

    # assert_true
    assert_true(True)
    assert_true(1 > 0)
    assert_true(len([1, 2, 3]) == 3)

    # assert_false
    assert_false(False)
    assert_false(1 > 2)
    assert_false(len([]) > 0)

    # assert_in
    assert_in(1, [1, 2, 3])
    assert_in("a", ["a", "b", "c"])
    assert_in("cat", "concatenate")
