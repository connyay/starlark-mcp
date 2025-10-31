# SQLite Explorer Extension for Starlark MCP
# Provides safe, read-only SQLite database exploration

def list_tables(params):
    """List all tables in the SQLite database"""
    db_path = params.get("db_path", "")

    if not db_path:
        return error_response("db_path parameter is required")

    tables = sqlite.list_tables(db_path)

    if not tables:
        return {
            "content": [{"type": "text", "text": "No tables found in database"}],
        }

    table_list = "Found {} tables:\n".format(len(tables))
    for table in tables:
        table_list += "  - {}\n".format(table["name"])

    return {
        "content": [{"type": "text", "text": table_list}],
    }

def describe_table(params):
    """Get the schema information for a specific table"""
    db_path = params.get("db_path", "")
    table_name = params.get("table", "")

    if not db_path:
        return error_response("db_path parameter is required")

    if not table_name:
        return error_response("table parameter is required")

    # Sanitize table name (basic check)
    if not is_valid_identifier(table_name):
        return error_response("Invalid table name: " + table_name)

    columns = sqlite.describe_table(db_path, table_name)

    if not columns:
        return error_response("Table '{}' not found".format(table_name))

    # Format schema information
    schema = "Table: {}\n".format(table_name)
    schema += "=" * (len(schema) - 1) + "\n\n"
    schema += "Columns:\n"

    for col in columns:
        schema += "  {} {} {}".format(
            col.get("name", ""),
            col.get("type", "TEXT"),
            "(PRIMARY KEY)" if col.get("pk", 0) > 0 else "",
        )
        if col.get("notnull", 0):
            schema += " NOT NULL"
        if col.get("dflt_value", None):
            schema += " DEFAULT {}".format(col.get("dflt_value"))
        schema += "\n"

    # Get row count using native query
    count_result = sqlite.query(db_path, "SELECT COUNT(*) as count FROM " + table_name, [])
    if count_result and len(count_result) > 0:
        row_count = count_result[0].get("count", 0)
        schema += "\nRow count: {}\n".format(row_count)

    return {
        "content": [{"type": "text", "text": schema}],
    }

def query(params):
    """Execute a SELECT query on the database"""
    db_path = params.get("db_path", "")
    sql = params.get("query", "")
    limit = params.get("limit", 100)

    if not db_path:
        return error_response("db_path parameter is required")

    if not sql:
        return error_response("query parameter is required")

    # Security: Only allow SELECT queries
    sql_upper = sql.strip().upper()
    if not sql_upper.startswith("SELECT"):
        return error_response("Only SELECT queries are allowed for safety")

    # Check for dangerous keywords
    dangerous = ["INSERT", "UPDATE", "DELETE", "DROP", "CREATE", "ALTER", "ATTACH", "DETACH"]
    for word in dangerous:
        if word in sql_upper:
            return error_response("Query contains unsafe keyword: " + word)

    # Add LIMIT if not present and limit is specified
    if limit > 0 and "LIMIT" not in sql_upper:
        sql = sql + " LIMIT {}".format(limit)

    # Execute query using native sqlite module
    rows = sqlite.query(db_path, sql, [])

    if not rows:
        return {
            "content": [{"type": "text", "text": "Query returned no results"}],
        }

    # Format results
    output = "Query returned {} row(s):\n\n".format(len(rows))
    output += json.encode(rows)

    return {
        "content": [{"type": "text", "text": output}],
    }

def analyze_database(params):
    """Analyze database statistics and provide insights"""
    db_path = params.get("db_path", "")

    if not db_path:
        return error_response("db_path parameter is required")

    analysis = "Database Analysis\n"
    analysis += "=" * 50 + "\n\n"

    # Get database size using exec.run()
    size_result = exec.run("ls", ["-lh", db_path])
    if size_result["success"]:
        parts = size_result["stdout"].strip().split()
        if len(parts) >= 5:
            analysis += "Database size: {}\n".format(parts[4])

    # Get list of tables using native module
    tables = sqlite.list_tables(db_path)
    analysis += "Number of tables: {}\n\n".format(len(tables))

    # Analyze each table
    analysis += "Table Statistics:\n"
    analysis += "-" * 30 + "\n"

    total_rows = 0
    for table in tables:
        # Get row count using native query
        table_name = table.get("name", "")
        count_result = sqlite.query(db_path, "SELECT COUNT(*) as count FROM " + table_name, [])
        if count_result and len(count_result) > 0:
            count = count_result[0].get("count", 0)
            total_rows += count
            analysis += "  {}: {} rows\n".format(table_name, count)

    analysis += "\nTotal rows across all tables: {}\n".format(total_rows)

    # Check for indexes using native query
    indexes = sqlite.query(db_path, "SELECT name FROM sqlite_master WHERE type='index'", [])
    analysis += "\nNumber of indexes: {}\n".format(len(indexes))

    # Database integrity check using native query
    integrity_result = sqlite.query(db_path, "PRAGMA integrity_check", [])
    if integrity_result and len(integrity_result) > 0:
        integrity = integrity_result[0].get("integrity_check", "")
        if integrity == "ok":
            analysis += "\nDatabase integrity: ✓ OK\n"
        else:
            analysis += "\nDatabase integrity: ⚠ Issues found\n"

    return {
        "content": [{"type": "text", "text": analysis}],
    }

def get_sample_data(params):
    """Get sample rows from a table"""
    db_path = params.get("db_path", "")
    table_name = params.get("table", "")
    count = params.get("count", 5)

    if not db_path:
        return error_response("db_path parameter is required")

    if not table_name:
        return error_response("table parameter is required")

    # Sanitize table name
    if not is_valid_identifier(table_name):
        return error_response("Invalid table name: " + table_name)

    # Get sample rows using native query
    query = "SELECT * FROM " + table_name + " LIMIT " + str(count)
    rows = sqlite.query(db_path, query, [])

    if not rows:
        return {
            "content": [{"type": "text", "text": "Table '{}' has no data".format(table_name)}],
        }

    # Format results
    output = "Sample data from '{}' (first {} rows):\n\n".format(table_name, len(rows))
    output += json.encode(rows)

    return {
        "content": [{"type": "text", "text": output}],
    }

# Helper functions
def is_valid_identifier(name):
    """Check if a name is a valid SQL identifier"""
    if not name:
        return False

    # Basic check - only alphanumeric and underscores
    # In Starlark, we check each character using elems() to split the string
    for i in range(len(name)):
        char = name[i]
        if not (char.isalnum() or char == "_"):
            return False

    # Must not start with a number
    if name[0].isdigit():
        return False

    return True

def error_response(message):
    """Create an error response"""
    return {
        "content": [{"type": "text", "text": "Error: " + message}],
        "isError": True,
    }

# Extension definition
def describe_extension():
    """Define the SQLite Explorer extension"""
    return Extension(
        name = "sqlite_explorer",
        version = "1.0.0",
        description = "Safe, read-only SQLite database exploration tools",
        allowed_exec = ["ls"],
        tools = [
            Tool(
                name = "sqlite_list_tables",
                description = "List all tables in a SQLite database",
                parameters = [
                    ToolParameter(
                        name = "db_path",
                        param_type = "string",
                        required = True,
                        description = "Path to the SQLite database file",
                    ),
                ],
                handler = list_tables,
            ),
            Tool(
                name = "sqlite_describe_table",
                description = "Get schema information for a specific table",
                parameters = [
                    ToolParameter(
                        name = "db_path",
                        param_type = "string",
                        required = True,
                        description = "Path to the SQLite database file",
                    ),
                    ToolParameter(
                        name = "table",
                        param_type = "string",
                        required = True,
                        description = "Name of the table to describe",
                    ),
                ],
                handler = describe_table,
            ),
            Tool(
                name = "sqlite_query",
                description = "Execute a SELECT query on the database (read-only)",
                parameters = [
                    ToolParameter(
                        name = "db_path",
                        param_type = "string",
                        required = True,
                        description = "Path to the SQLite database file",
                    ),
                    ToolParameter(
                        name = "query",
                        param_type = "string",
                        required = True,
                        description = "SELECT query to execute",
                    ),
                    ToolParameter(
                        name = "limit",
                        param_type = "integer",
                        required = False,
                        default = "100",
                        description = "Maximum number of rows to return (default: 100)",
                    ),
                ],
                handler = query,
            ),
            Tool(
                name = "sqlite_analyze",
                description = "Analyze database statistics and provide insights",
                parameters = [
                    ToolParameter(
                        name = "db_path",
                        param_type = "string",
                        required = True,
                        description = "Path to the SQLite database file",
                    ),
                ],
                handler = analyze_database,
            ),
            Tool(
                name = "sqlite_sample_data",
                description = "Get sample rows from a table",
                parameters = [
                    ToolParameter(
                        name = "db_path",
                        param_type = "string",
                        required = True,
                        description = "Path to the SQLite database file",
                    ),
                    ToolParameter(
                        name = "table",
                        param_type = "string",
                        required = True,
                        description = "Name of the table to sample",
                    ),
                    ToolParameter(
                        name = "count",
                        param_type = "integer",
                        required = False,
                        default = "5",
                        description = "Number of sample rows to return (default: 5)",
                    ),
                ],
                handler = get_sample_data,
            ),
        ],
    )
