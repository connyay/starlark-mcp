load("cat_facts", "get_cat_fact")

def test_get_cat_fact_returns_content():
    """Test that get_cat_fact returns a valid response structure."""
    result = get_cat_fact({})

    testing.is_true(type(result) == "dict", "Result should be a dict")
    testing.contains(result, "content", "Result should have 'content' key")

    content = result["content"]
    testing.is_true(type(content) == "list", "Content should be a list")
    testing.is_true(len(content) > 0, "Content should have at least one item")

    first_item = content[0]
    testing.contains(first_item, "type", "Content item should have 'type' key")
    testing.contains(first_item, "text", "Content item should have 'text' key")
    testing.eq("text", first_item["type"], "Content type should be 'text'")

def test_get_cat_fact_returns_valid_fact():
    """Test that get_cat_fact returns one of the known facts."""
    result = get_cat_fact({})
    fact = result["content"][0]["text"]

    # Hardcoded list must stay in sync with cat_facts.star to catch unintended changes
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

    testing.contains(valid_facts, fact, "Returned fact should be one of the known cat facts")

def test_get_cat_fact_returns_non_empty_text():
    """Test that get_cat_fact returns non-empty text."""
    result = get_cat_fact({})
    fact = result["content"][0]["text"]

    testing.is_true(len(fact) > 0, "Cat fact should not be empty")
    testing.is_true(type(fact) == "string", "Cat fact should be a string")
