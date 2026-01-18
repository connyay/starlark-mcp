load("fuzzy_extension", "fuzzy_handler")

# Test data.load_json functionality
def test_data_load_json_loads_pokemon_items():
    """Test that data.load_json can load the pokemon_items.json file."""
    items = data.load_json("pokemon_items.json")

    testing.is_true(type(items) == "list", "Items should be a list")
    testing.is_true(len(items) > 0, "Items should not be empty")

    # Check structure of first item
    first_item = items[0]
    testing.is_true(type(first_item) == "dict", "Each item should be a dict")
    testing.contains(first_item, "name", "Item should have 'name' key")
    testing.contains(first_item, "desc", "Item should have 'desc' key")
    testing.contains(first_item, "type", "Item should have 'type' key")

# Test fuzzy.search functionality
def test_fuzzy_search_with_strings():
    """Test fuzzy.search with a list of strings."""
    items = ["Potion", "Super Potion", "Hyper Potion", "Antidote", "Paralyze Heal"]

    results = fuzzy.search("potion", items)

    testing.is_true(type(results) == "list", "Results should be a list")
    testing.is_true(len(results) >= 3, "Should find at least 3 potion items")

    # All results should contain 'Potion'
    for result in results:
        testing.is_true("Potion" in result, "Result should contain 'Potion'")

def test_fuzzy_search_with_dicts_and_key():
    """Test fuzzy.search with dicts and a specific key."""
    items = [
        {"name": "Master Ball", "type": "Pokeballs"},
        {"name": "Ultra Ball", "type": "Pokeballs"},
        {"name": "Potion", "type": "Medicine"},
    ]

    results = fuzzy.search("ball", items, key = "name")

    testing.is_true(type(results) == "list", "Results should be a list")
    testing.eq(len(results), 2, "Should find exactly 2 ball items")

def test_fuzzy_search_with_limit():
    """Test fuzzy.search respects the limit parameter."""
    items = data.load_json("pokemon_items.json")

    results = fuzzy.search("ball", items, key = "name", limit = 3)

    testing.is_true(len(results) <= 3, "Results should be limited to 3")

def test_fuzzy_search_no_matches():
    """Test fuzzy.search returns empty list when no matches."""
    items = ["Potion", "Antidote", "Paralyze Heal"]

    results = fuzzy.search("xyznonexistent", items)

    testing.eq(len(results), 0, "Should return empty list for no matches")

# Test fuzzy.search_with_scores functionality
def test_fuzzy_search_with_scores_returns_scores():
    """Test fuzzy.search_with_scores returns items with scores."""
    items = ["hello", "help", "helicopter"]

    results = fuzzy.search_with_scores("hel", items)

    testing.is_true(type(results) == "list", "Results should be a list")
    testing.is_true(len(results) > 0, "Should have at least one result")

    # Each result should have 'item' and 'score' keys
    for result in results:
        testing.is_true(type(result) == "dict", "Each result should be a dict")
        testing.contains(result, "item", "Result should have 'item' key")
        testing.contains(result, "score", "Result should have 'score' key")
        testing.is_true(type(result["score"]) == "int", "Score should be an integer")

def test_fuzzy_search_with_scores_sorted_by_relevance():
    """Test fuzzy.search_with_scores returns results sorted by score descending."""
    items = ["hello", "helo", "helicopter", "world"]

    results = fuzzy.search_with_scores("hello", items)

    # Scores should be in descending order
    if len(results) > 1:
        for i in range(len(results) - 1):
            testing.is_true(
                results[i]["score"] >= results[i + 1]["score"],
                "Scores should be in descending order",
            )

# Test the fuzzy_handler function from the extension
def test_fuzzy_handler_returns_valid_response():
    """Test that fuzzy_handler returns a valid MCP response."""
    result = fuzzy_handler({"query": "potion"})

    testing.is_true(type(result) == "dict", "Result should be a dict")
    testing.contains(result, "content", "Result should have 'content' key")
    testing.contains(result, "structuredContent", "Result should have 'structuredContent' key")

def test_fuzzy_handler_returns_matching_items():
    """Test that fuzzy_handler returns items matching the query."""
    result = fuzzy_handler({"query": "master ball", "limit": 5})

    structured = result["structuredContent"]
    testing.is_true(type(structured) == "list", "structuredContent should be a list")
    testing.is_true(len(structured) > 0, "Should return at least one matching item")

    # First result should have item details
    first = structured[0]
    testing.contains(first, "name", "Item should have 'name'")
    testing.contains(first, "description", "Item should have 'description'")
    testing.contains(first, "type", "Item should have 'type'")
    testing.contains(first, "score", "Item should have 'score'")

def test_fuzzy_handler_with_empty_query():
    """Test that fuzzy_handler handles empty query gracefully."""
    result = fuzzy_handler({"query": ""})

    testing.is_true(type(result) == "dict", "Result should be a dict")
    testing.contains(result, "isError", "Result should indicate error")
    testing.eq(result["isError"], True, "Should be an error response")

def test_fuzzy_handler_respects_limit():
    """Test that fuzzy_handler respects the limit parameter."""
    result = fuzzy_handler({"query": "ball", "limit": 3})

    structured = result["structuredContent"]
    testing.is_true(len(structured) <= 3, "Should respect limit of 3")

# Test the keys parameter
def test_fuzzy_search_with_keys_parameter():
    """Test fuzzy.search with multiple keys parameter."""
    items = [
        {"name": "Potion", "desc": "Restores HP", "type": "Medicine"},
        {"name": "Antidote", "desc": "Cures poison", "type": "Medicine"},
        {"name": "Revive", "desc": "Revives fainted Pokemon", "type": "Medicine"},
    ]

    # Search across name and desc fields
    results = fuzzy.search("restore", items, keys = ["name", "desc"])

    testing.is_true(type(results) == "list", "Results should be a list")
    testing.is_true(len(results) > 0, "Should find matching items")

    # Potion should match because desc contains "Restores"
    found_potion = False
    for r in results:
        if r["name"] == "Potion":
            found_potion = True
    testing.is_true(found_potion, "Should find Potion via desc field")

def test_fuzzy_search_with_scores_and_keys():
    """Test fuzzy.search_with_scores with multiple keys."""
    items = data.load_json("pokemon_items.json")

    # Search both name and desc fields
    results = fuzzy.search_with_scores("healing restore", items, keys = ["name", "desc"], limit = 5)

    testing.is_true(type(results) == "list", "Results should be a list")

    for r in results:
        testing.contains(r, "item", "Should have item")
        testing.contains(r, "score", "Should have score")

def test_fuzzy_search_keys_searches_multiple_fields():
    """Test that keys parameter actually searches multiple fields."""
    items = [
        {"name": "Apple", "color": "Red", "taste": "Sweet"},
        {"name": "Banana", "color": "Yellow", "taste": "Sweet"},
        {"name": "Cherry", "color": "Red", "taste": "Tart"},
    ]

    # Search for "red" - should only match items where color is "Red"
    results = fuzzy.search("red", items, keys = ["color"])
    testing.eq(len(results), 2, "Should find 2 red items")

    # Search for "sweet" in taste field
    results = fuzzy.search("sweet", items, keys = ["taste"])
    testing.eq(len(results), 2, "Should find 2 sweet items")
