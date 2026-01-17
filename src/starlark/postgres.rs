use allocative::Allocative;
use anyhow::{Result, anyhow};
use chrono::NaiveDateTime;
use derive_more::Display;
use postgres::types::Type;
use postgres::{Client, NoTls, Row};
use serde_json::Value as JsonValue;
use starlark::collections::SmallMap;
use starlark::environment::{GlobalsBuilder, Methods, MethodsBuilder, MethodsStatic};
use starlark::starlark_module;
use starlark::starlark_simple_value;
use starlark::values::starlark_value;
use starlark::values::{
    Heap, NoSerialize, ProvidesStaticType, StarlarkValue, Value, dict::Dict, none::NoneType,
};

/// PostgreSQL module for database operations
#[derive(Debug, Display, Allocative, ProvidesStaticType, NoSerialize)]
#[display(fmt = "postgres")]
pub struct PostgresModule;

starlark_simple_value!(PostgresModule);

#[starlark_value(type = "postgres")]
impl<'v> StarlarkValue<'v> for PostgresModule {
    fn get_methods() -> Option<&'static Methods> {
        static RES: MethodsStatic = MethodsStatic::new();
        RES.methods(postgres_methods)
    }
}

/// Methods available on the postgres module
#[starlark_module]
#[allow(clippy::type_complexity)]
fn postgres_methods(builder: &mut MethodsBuilder) {
    /// Execute a SELECT query and return results as list of dicts
    fn query<'v>(
        #[allow(unused_variables)] this: Value<'v>,
        connection_string: &str,
        query: &str,
        #[starlark(default = NoneType)] params: Value<'v>,
        heap: &'v Heap,
    ) -> anyhow::Result<Value<'v>> {
        execute_query(connection_string, query, params, heap)
    }

    /// Execute INSERT/UPDATE/DELETE and return affected rows
    fn execute<'v>(
        #[allow(unused_variables)] this: Value<'v>,
        connection_string: &str,
        statement: &str,
        #[starlark(default = NoneType)] params: Value<'v>,
        heap: &'v Heap,
    ) -> anyhow::Result<i32> {
        execute_statement(connection_string, statement, params, heap)
    }

    /// List all tables in the database
    fn list_tables<'v>(
        #[allow(unused_variables)] this: Value<'v>,
        connection_string: &str,
        #[starlark(default = "public")] schema: &str,
        heap: &'v Heap,
    ) -> anyhow::Result<Value<'v>> {
        let query = "SELECT tablename FROM pg_tables WHERE schemaname = $1 ORDER BY tablename";
        execute_query(
            connection_string,
            query,
            heap.alloc(vec![heap.alloc(schema)]),
            heap,
        )
    }

    /// Get table schema information
    fn describe_table<'v>(
        #[allow(unused_variables)] this: Value<'v>,
        connection_string: &str,
        table_name: &str,
        #[starlark(default = "public")] schema: &str,
        heap: &'v Heap,
    ) -> anyhow::Result<Value<'v>> {
        let query = "
            SELECT
                column_name,
                data_type,
                is_nullable,
                column_default,
                character_maximum_length
            FROM information_schema.columns
            WHERE table_schema = $1 AND table_name = $2
            ORDER BY ordinal_position
        ";
        let params = vec![heap.alloc(schema), heap.alloc(table_name)];
        execute_query(connection_string, query, heap.alloc(params), heap)
    }
}

/// Register the postgres module in the global namespace
pub fn register(builder: &mut GlobalsBuilder) {
    const POSTGRES: PostgresModule = PostgresModule;
    builder.set("postgres", POSTGRES);
}

// Helper function to execute a query and return results
fn execute_query<'v>(
    connection_string: &str,
    query: &str,
    params: Value<'v>,
    heap: &'v Heap,
) -> Result<Value<'v>> {
    // Parse connection string (obfuscate password in errors)
    let safe_conn_str = obfuscate_password(connection_string);

    // Convert Starlark parameters to PostgreSQL parameters
    let pg_params = convert_params_to_postgres(params, heap)?;

    // Clone values for thread
    let conn_str = connection_string.to_string();
    let query_str = query.to_string();

    // Run PostgreSQL operations in a separate thread to avoid runtime conflicts
    let rows = std::thread::spawn(move || {
        // Connect to database
        let mut client = Client::connect(&conn_str, NoTls)
            .map_err(|e| anyhow!("Failed to connect to PostgreSQL: {}", e))?;

        // Execute query
        let rows = if pg_params.is_empty() {
            client
                .query(&query_str, &[])
                .map_err(|e| anyhow!("Query execution failed: {}", e))?
        } else {
            // Convert params to ToSql trait objects
            let sql_params: Vec<Box<dyn postgres::types::ToSql + Sync>> =
                pg_params.iter().map(|p| p.to_sql()).collect();
            let param_refs: Vec<&(dyn postgres::types::ToSql + Sync)> =
                sql_params.iter().map(|p| p.as_ref()).collect();

            client
                .query(&query_str, &param_refs[..])
                .map_err(|e| anyhow!("Query execution failed: {}", e))?
        };

        Ok::<Vec<Row>, anyhow::Error>(rows)
    })
    .join()
    .map_err(|e| anyhow!("Thread panicked: {:?}", e))?
    .map_err(|e| anyhow!("PostgreSQL operation failed ({}): {}", safe_conn_str, e))?;

    // Convert rows to Starlark list of dicts
    rows_to_starlark(&rows, heap)
}

// Helper function to execute a statement and return affected rows
fn execute_statement<'v>(
    connection_string: &str,
    statement: &str,
    params: Value<'v>,
    heap: &'v Heap,
) -> Result<i32> {
    // Parse connection string (obfuscate password in errors)
    let safe_conn_str = obfuscate_password(connection_string);

    // Convert Starlark parameters to PostgreSQL parameters
    let pg_params = convert_params_to_postgres(params, heap)?;

    // Clone values for thread
    let conn_str = connection_string.to_string();
    let stmt_str = statement.to_string();

    // Run PostgreSQL operations in a separate thread to avoid runtime conflicts
    let affected_rows = std::thread::spawn(move || {
        // Connect to database
        let mut client = Client::connect(&conn_str, NoTls)
            .map_err(|e| anyhow!("Failed to connect to PostgreSQL: {}", e))?;

        // Execute statement
        let affected_rows = if pg_params.is_empty() {
            client
                .execute(&stmt_str, &[])
                .map_err(|e| anyhow!("Statement execution failed: {}", e))?
        } else {
            // Convert params to ToSql trait objects
            let sql_params: Vec<Box<dyn postgres::types::ToSql + Sync>> =
                pg_params.iter().map(|p| p.to_sql()).collect();
            let param_refs: Vec<&(dyn postgres::types::ToSql + Sync)> =
                sql_params.iter().map(|p| p.as_ref()).collect();

            client
                .execute(&stmt_str, &param_refs[..])
                .map_err(|e| anyhow!("Statement execution failed: {}", e))?
        };

        Ok::<u64, anyhow::Error>(affected_rows)
    })
    .join()
    .map_err(|e| anyhow!("Thread panicked: {:?}", e))?
    .map_err(|e| anyhow!("PostgreSQL operation failed ({}): {}", safe_conn_str, e))?;

    Ok(affected_rows as i32)
}

// Convert Starlark parameters to PostgreSQL parameters
fn convert_params_to_postgres<'v>(params: Value<'v>, heap: &'v Heap) -> Result<Vec<PostgresParam>> {
    let mut pg_params = Vec::new();

    if !params.is_none() {
        // Iterate over the list of parameters
        for param in params
            .iterate(heap)
            .map_err(|e| anyhow!("Failed to iterate parameters: {}", e))?
        {
            let pg_param = starlark_to_postgres_param(param)?;
            pg_params.push(pg_param);
        }
    }

    Ok(pg_params)
}

// Parameter type that can be sent across threads
#[derive(Debug, Clone)]
enum PostgresParam {
    Null,
    Bool(bool),
    Int(i32),
    String(String),
}

impl PostgresParam {
    fn to_sql(&self) -> Box<dyn postgres::types::ToSql + Sync> {
        match self {
            PostgresParam::Null => Box::new(None::<String>),
            PostgresParam::Bool(b) => Box::new(*b),
            PostgresParam::Int(i) => Box::new(*i),
            PostgresParam::String(s) => Box::new(s.clone()),
        }
    }
}

// Convert a single Starlark value to a thread-safe PostgreSQL parameter
fn starlark_to_postgres_param(value: Value) -> Result<PostgresParam> {
    if value.is_none() {
        Ok(PostgresParam::Null)
    } else if let Some(b) = value.unpack_bool() {
        Ok(PostgresParam::Bool(b))
    } else if let Some(i) = value.unpack_i32() {
        Ok(PostgresParam::Int(i))
    } else if let Some(s) = value.unpack_str() {
        Ok(PostgresParam::String(s.to_string()))
    } else {
        // Try to convert as string fallback
        Ok(PostgresParam::String(value.to_str()))
    }
}

// Convert PostgreSQL rows to Starlark list of dicts
fn rows_to_starlark<'v>(rows: &[Row], heap: &'v Heap) -> Result<Value<'v>> {
    let mut result = Vec::new();

    for row in rows {
        let mut row_map = SmallMap::new();

        for (idx, column) in row.columns().iter().enumerate() {
            let col_name = column.name();
            let value = postgres_value_to_starlark(row, idx, heap)?;

            row_map.insert_hashed(
                heap.alloc_str(col_name)
                    .to_value()
                    .get_hashed()
                    .map_err(|e| anyhow!("Failed to hash column name: {}", e))?,
                value,
            );
        }

        result.push(heap.alloc(Dict::new(row_map)));
    }

    Ok(heap.alloc(result))
}

// Convert a PostgreSQL value to a Starlark value
fn postgres_value_to_starlark<'v>(row: &Row, idx: usize, heap: &'v Heap) -> Result<Value<'v>> {
    let column = &row.columns()[idx];
    let column_type = column.type_();

    match *column_type {
        Type::BOOL => {
            let val: bool = row
                .try_get(idx)
                .map_err(|e| anyhow!("Failed to get BOOL at column {}: {}", idx, e))?;
            Ok(heap.alloc(val))
        }
        Type::INT2 => {
            let val: i16 = row
                .try_get(idx)
                .map_err(|e| anyhow!("Failed to get INT2 at column {}: {}", idx, e))?;
            Ok(heap.alloc(val as i32))
        }
        Type::INT4 => match row.try_get::<_, Option<i32>>(idx) {
            Ok(Some(val)) => Ok(heap.alloc(val)),
            Ok(None) => Ok(Value::new_none()),
            Err(e) => Err(anyhow!("Failed to get INT4 at column {}: {}", idx, e)),
        },
        Type::INT8 => {
            let val: i64 = row
                .try_get(idx)
                .map_err(|e| anyhow!("Failed to get INT8 at column {}: {}", idx, e))?;
            // Note: Starlark doesn't have i64, so we may lose precision for very large values
            Ok(heap.alloc(val as i32))
        }
        Type::FLOAT4 => {
            let val: f32 = row
                .try_get(idx)
                .map_err(|e| anyhow!("Failed to get FLOAT4 at column {}: {}", idx, e))?;
            Ok(heap.alloc(val as f64))
        }
        Type::FLOAT8 => {
            let val: f64 = row
                .try_get(idx)
                .map_err(|e| anyhow!("Failed to get FLOAT8 at column {}: {}", idx, e))?;
            Ok(heap.alloc(val))
        }
        Type::TEXT | Type::VARCHAR | Type::CHAR | Type::BPCHAR => {
            match row.try_get::<_, Option<String>>(idx) {
                Ok(Some(val)) => Ok(heap.alloc_str(&val).to_value()),
                Ok(None) => Ok(Value::new_none()),
                Err(e) => Err(anyhow!("Failed to get TEXT at column {}: {}", idx, e)),
            }
        }
        Type::TIMESTAMP | Type::TIMESTAMPTZ => {
            let val: NaiveDateTime = row
                .try_get(idx)
                .map_err(|e| anyhow!("Failed to get TIMESTAMP at column {}: {}", idx, e))?;
            // Convert to Unix timestamp (seconds since epoch)
            Ok(heap.alloc(val.and_utc().timestamp() as i32))
        }
        Type::JSON | Type::JSONB => {
            let val: JsonValue = row
                .try_get(idx)
                .map_err(|e| anyhow!("Failed to get JSON at column {}: {}", idx, e))?;
            json_to_starlark(&val, heap)
        }
        _ => {
            // Fallback: try to get as string
            match row.try_get::<_, Option<String>>(idx) {
                Ok(Some(s)) => Ok(heap.alloc_str(&s).to_value()),
                Ok(None) => Ok(Value::new_none()),
                Err(_) => {
                    // If we can't get it as string, return None
                    Ok(Value::new_none())
                }
            }
        }
    }
}

// Convert JSON value to Starlark value (from http.rs)
fn json_to_starlark<'v>(json: &JsonValue, heap: &'v Heap) -> Result<Value<'v>> {
    match json {
        JsonValue::Null => Ok(Value::new_none()),
        JsonValue::Bool(b) => Ok(heap.alloc(*b)),
        JsonValue::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(heap.alloc(i as i32))
            } else if let Some(f) = n.as_f64() {
                Ok(heap.alloc(f))
            } else {
                Ok(heap.alloc_str(&n.to_string()).to_value())
            }
        }
        JsonValue::String(s) => Ok(heap.alloc_str(s).to_value()),
        JsonValue::Array(arr) => {
            let items: Result<Vec<Value>> = arr.iter().map(|v| json_to_starlark(v, heap)).collect();
            Ok(heap.alloc(items?))
        }
        JsonValue::Object(obj) => {
            let mut map = SmallMap::new();
            for (key, value) in obj {
                let starlark_value = json_to_starlark(value, heap)?;
                map.insert_hashed(
                    heap.alloc_str(key)
                        .to_value()
                        .get_hashed()
                        .map_err(|e| anyhow!("Failed to hash JSON key: {}", e))?,
                    starlark_value,
                );
            }
            Ok(heap.alloc(Dict::new(map)))
        }
    }
}

// Obfuscate password in connection string for error messages
fn obfuscate_password(conn_str: &str) -> String {
    // Pattern 1: postgresql://user:password@host
    let re1 = regex::Regex::new(r"(postgresql://[^:]+:)[^@]+(@.+)")
        .unwrap_or_else(|_| regex::Regex::new("").unwrap());
    let result = re1.replace(conn_str, "${1}****${2}");

    // Pattern 2: password=value in DSN
    let re2 =
        regex::Regex::new(r"(password=)[^;\s]+").unwrap_or_else(|_| regex::Regex::new("").unwrap());
    let result = re2.replace(&result, "${1}****");

    result.to_string()
}
