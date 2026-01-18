pokemon_items = data.load_json("pokemon_items.json")

def fuzzy_handler(params):
    """Handler that performs fuzzy search on Pokemon items."""
    query = params.get("query", "")
    limit = params.get("limit", 10)

    if not query:
        return {
            "content": [{"type": "text", "text": "Please provide a search query"}],
            "isError": True,
        }

    results = fuzzy.search_with_scores(query, pokemon_items, key = "name", limit = limit)

    if not results:
        message = "No items found matching: " + query
        return {
            "content": [{"type": "text", "text": message}],
            "structuredContent": [],
        }

    result_items = []
    for r in results:
        result_items.append({
            "name": r["item"]["name"],
            "description": r["item"]["desc"],
            "type": r["item"]["type"],
            "score": r["score"],
        })

    message = "Found " + str(len(result_items)) + " items matching: " + query

    return {
        "content": [{"type": "text", "text": message}],
        "structuredContent": result_items,
    }

def describe_extension():
    """Describes the fuzzy extension."""
    return Extension(
        name = "fuzzy_extension",
        version = "1.0.0",
        description = "An extension that provides fuzzy search capabilities for Pokemon items.",
        tools = [
            Tool(
                name = "fuzzy_search",
                description = "Search for Pokemon items by name using fuzzy matching. Returns items sorted by relevance.",
                handler = fuzzy_handler,
                parameters = [
                    ToolParameter(
                        name = "query",
                        param_type = "string",
                        required = True,
                        description = "The search query to match against item names.",
                    ),
                    ToolParameter(
                        name = "limit",
                        param_type = "number",
                        required = False,
                        description = "Maximum number of results to return (default: 10).",
                    ),
                ],
            ),
        ],
    )
