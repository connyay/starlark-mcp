# PostgreSQL MCP Server Extension
# Provides database query tools via PostgreSQL

# Configuration helper
def get_postgres_config():
    """Get PostgreSQL configuration from environment or defaults"""

    # Check for single connection string first
    conn_str = env.get("POSTGRES_CONNECTION_STRING", "")
    if conn_str:
        return conn_str

    # Build connection string from components
    host = env.get("POSTGRES_HOST", "localhost")
    port = env.get("POSTGRES_PORT", "5432")
    user = env.get("POSTGRES_USER", "postgres")
    password = env.get("POSTGRES_PASSWORD", "")
    database = env.get("POSTGRES_DATABASE", "postgres")

    # Build PostgreSQL connection string
    if password:
        return "postgresql://{}:{}@{}:{}/{}".format(user, password, host, port, database)
    else:
        return "postgresql://{}@{}:{}/{}".format(user, host, port, database)

# Tool implementations
def list_databases(params):
    """List all databases in the PostgreSQL server"""
    conn_str = get_postgres_config()

    if not conn_str:
        return error_response("PostgreSQL connection not configured. Set POSTGRES_CONNECTION_STRING or individual POSTGRES_* environment variables.")

    query = """
        SELECT
            datname as name,
            pg_size_pretty(pg_database_size(datname)) as size,
            datcollate as collation,
            datconnlimit as connection_limit
        FROM pg_database
        WHERE datistemplate = false
        ORDER BY datname
    """

    result = postgres.query(conn_str, query, [])

    output = "Found {} database(s):\n\n".format(len(result))
    for db in result:
        output += "📊 {} ({})\n".format(db["name"], db["size"])
        output += "   Collation: {}\n".format(db["collation"])
        limit = db.get("connection_limit", -1)
        if limit > 0:
            output += "   Connection limit: {}\n".format(limit)
        output += "\n"

    return {"content": [{"type": "text", "text": output}]}

def list_tables(params):
    """List all tables in the database"""
    conn_str = get_postgres_config()

    if not conn_str:
        return error_response("PostgreSQL connection not configured.")

    schema = params.get("schema", "public")

    tables = postgres.list_tables(conn_str, schema)

    output = "Found {} table(s) in schema '{}':\n\n".format(len(tables), schema)
    for table in tables:
        output += "📋 {}\n".format(table["tablename"])

    return {"content": [{"type": "text", "text": output}]}

def describe_table(params):
    """Get detailed information about a table"""
    conn_str = get_postgres_config()

    if not conn_str:
        return error_response("PostgreSQL connection not configured.")

    table_name = params.get("table_name", "")
    schema = params.get("schema", "public")

    if not table_name:
        return error_response("table_name parameter is required")

    columns = postgres.describe_table(conn_str, table_name, schema)

    if not columns:
        return error_response("Table '{}' not found in schema '{}'".format(table_name, schema))

    output = "Table: {}.{}\n".format(schema, table_name)
    output += "=" * 50 + "\n\n"
    output += "Columns:\n"

    for col in columns:
        nullable = "NULL" if col["is_nullable"] == "YES" else "NOT NULL"
        col_type = col["data_type"]

        # Add length info for varchar/char
        if col.get("character_maximum_length"):
            col_type += "({})".format(col["character_maximum_length"])

        output += "  - {} {} {}\n".format(
            col["column_name"],
            col_type,
            nullable,
        )

        if col.get("column_default"):
            output += "    Default: {}\n".format(col["column_default"])

    return {"content": [{"type": "text", "text": output}]}

def query_table(params):
    """Query a table with optional filters"""
    conn_str = get_postgres_config()

    if not conn_str:
        return error_response("PostgreSQL connection not configured.")

    table_name = params.get("table_name", "")
    schema = params.get("schema", "public")
    limit = params.get("limit", "100")
    where_clause = params.get("where", "")
    order_by = params.get("order_by", "")

    if not table_name:
        return error_response("table_name parameter is required")

    # Build query
    query = "SELECT * FROM {}.{}".format(schema, table_name)
    query_params = []

    if where_clause:
        query += " WHERE {}".format(where_clause)

    if order_by:
        query += " ORDER BY {}".format(order_by)

    query += " LIMIT {}".format(limit)

    rows = postgres.query(conn_str, query, query_params)

    if not rows:
        output = "No rows found in {}.{}\n".format(schema, table_name)
    else:
        output = "Query returned {} row(s) from {}.{}:\n\n".format(
            len(rows),
            schema,
            table_name,
        )

        # Format as simple table
        if len(rows) > 0:
            # Get column names
            columns = list(rows[0].keys())

            # Show first few rows in detail
            for i, row in enumerate(rows[:10]):
                output += "Row {}:\n".format(i + 1)
                for col in columns:
                    value = row.get(col)
                    if value == None:
                        value = "NULL"
                    output += "  {}: {}\n".format(col, value)
                output += "\n"

            if len(rows) > 10:
                output += "... and {} more rows\n".format(len(rows) - 10)

    return {"content": [{"type": "text", "text": output}]}

def execute_sql(params):
    """Execute arbitrary SQL query (unrestricted mode)"""
    conn_str = get_postgres_config()

    if not conn_str:
        return error_response("PostgreSQL connection not configured.")

    sql = params.get("sql", "")
    query_params = params.get("params", [])

    if not sql:
        return error_response("sql parameter is required")

    # Determine if it's a query or statement
    sql_upper = sql.strip().upper()
    is_query = sql_upper.startswith(("SELECT", "WITH", "SHOW", "EXPLAIN"))

    if is_query:
        rows = postgres.query(conn_str, sql, query_params)

        if not rows:
            output = "Query returned no rows\n"
        else:
            output = "Query returned {} row(s):\n\n".format(len(rows))

            # Format results
            for i, row in enumerate(rows[:50]):  # Limit to first 50 rows
                output += "Row {}:\n".format(i + 1)
                for key, value in row.items():
                    if value == None:
                        value = "NULL"
                    output += "  {}: {}\n".format(key, value)
                output += "\n"

            if len(rows) > 50:
                output += "... and {} more rows\n".format(len(rows) - 50)
    else:
        # Execute as statement
        affected_rows = postgres.execute(conn_str, sql, query_params)
        output = "✅ Statement executed successfully\n"
        output += "Affected rows: {}\n".format(affected_rows)

    return {"content": [{"type": "text", "text": output}]}

def get_table_stats(params):
    """Get statistics about tables in the database"""
    conn_str = get_postgres_config()

    if not conn_str:
        return error_response("PostgreSQL connection not configured.")

    query = """
        SELECT
            n.nspname as schemaname,
            c.relname as tablename,
            pg_size_pretty(pg_total_relation_size(c.oid)) as total_size,
            pg_total_relation_size(c.oid) as size_bytes,
            c.reltuples::bigint as estimated_rows
        FROM pg_class c
        JOIN pg_namespace n ON n.oid = c.relnamespace
        WHERE c.relkind = 'r'
          AND n.nspname NOT IN ('pg_catalog', 'information_schema')
          AND n.nspname NOT LIKE 'pg_toast%'
        ORDER BY pg_total_relation_size(c.oid) DESC
        LIMIT 20
    """

    rows = postgres.query(conn_str, query, [])

    output = "Top {} largest tables:\n\n".format(len(rows))
    for row in rows:
        output += "📊 {}.{} ({})\n".format(
            row["schemaname"],
            row["tablename"],
            row["total_size"],
        )

        estimated_rows = row.get("estimated_rows", 0)
        if estimated_rows and estimated_rows > 0:
            output += "   Estimated rows: {:,}\n".format(estimated_rows)

        output += "\n"

    return {"content": [{"type": "text", "text": output}]}

# Helper functions
def error_response(message):
    """Create an error response"""
    return {
        "content": [{"type": "text", "text": "Error: " + message}],
        "isError": True,
    }

# Extension definition
def describe_extension():
    """Define the PostgreSQL MCP extension"""
    return Extension(
        name = "postgres",
        version = "1.0.0",
        description = "PostgreSQL database integration",
        tools = [
            Tool(
                name = "postgres_list_databases",
                description = "List all databases in the PostgreSQL server",
                parameters = [],
                handler = list_databases,
            ),
            Tool(
                name = "postgres_list_tables",
                description = "List all tables in a database schema",
                parameters = [
                    ToolParameter(
                        name = "schema",
                        param_type = "string",
                        required = False,
                        default = "public",
                        description = "Schema name",
                    ),
                ],
                handler = list_tables,
            ),
            Tool(
                name = "postgres_describe_table",
                description = "Get detailed information about a table",
                parameters = [
                    ToolParameter(
                        name = "table_name",
                        param_type = "string",
                        required = True,
                        description = "Name of the table",
                    ),
                    ToolParameter(
                        name = "schema",
                        param_type = "string",
                        required = False,
                        default = "public",
                        description = "Schema name",
                    ),
                ],
                handler = describe_table,
            ),
            Tool(
                name = "postgres_query_table",
                description = "Query a table with optional filters",
                parameters = [
                    ToolParameter(
                        name = "table_name",
                        param_type = "string",
                        required = True,
                        description = "Name of the table to query",
                    ),
                    ToolParameter(
                        name = "schema",
                        param_type = "string",
                        required = False,
                        default = "public",
                        description = "Schema name",
                    ),
                    ToolParameter(
                        name = "where",
                        param_type = "string",
                        required = False,
                        description = "WHERE clause (without WHERE keyword)",
                    ),
                    ToolParameter(
                        name = "order_by",
                        param_type = "string",
                        required = False,
                        description = "ORDER BY clause (without ORDER BY keyword)",
                    ),
                    ToolParameter(
                        name = "limit",
                        param_type = "string",
                        required = False,
                        default = "100",
                        description = "Maximum number of rows to return",
                    ),
                ],
                handler = query_table,
            ),
            Tool(
                name = "postgres_execute_sql",
                description = "Execute arbitrary SQL query or statement",
                parameters = [
                    ToolParameter(
                        name = "sql",
                        param_type = "string",
                        required = True,
                        description = "SQL query or statement to execute",
                    ),
                    ToolParameter(
                        name = "params",
                        param_type = "array",
                        required = False,
                        description = "Query parameters for parameterized queries",
                    ),
                ],
                handler = execute_sql,
            ),
            Tool(
                name = "postgres_table_stats",
                description = "Get statistics about tables (size, row count, vacuum info)",
                parameters = [],
                handler = get_table_stats,
            ),
        ],
    )
