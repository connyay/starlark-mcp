use allocative::Allocative;
use anyhow::{anyhow, Result};
use derive_more::Display;
use rusqlite::{Connection, Row};
use starlark::collections::SmallMap;
use starlark::environment::{GlobalsBuilder, Methods, MethodsBuilder, MethodsStatic};
use starlark::starlark_module;
use starlark::starlark_simple_value;
use starlark::values::starlark_value;
use starlark::values::{
    dict::Dict, none::NoneType, Heap, NoSerialize, ProvidesStaticType, StarlarkValue, Value,
};

/// SQLite module for database operations
#[derive(Debug, Display, Allocative, ProvidesStaticType, NoSerialize)]
#[display(fmt = "sqlite")]
pub struct SqliteModule;

starlark_simple_value!(SqliteModule);

#[starlark_value(type = "sqlite")]
impl<'v> StarlarkValue<'v> for SqliteModule {
    fn get_methods() -> Option<&'static Methods> {
        static RES: MethodsStatic = MethodsStatic::new();
        RES.methods(sqlite_methods)
    }
}

/// Methods available on the sqlite module
#[starlark_module]
#[allow(clippy::type_complexity)]
fn sqlite_methods(builder: &mut MethodsBuilder) {
    /// Execute a SELECT query and return results as list of dicts
    fn query<'v>(
        #[allow(unused_variables)] this: Value<'v>,
        db_path: &str,
        query: &str,
        #[starlark(default = NoneType)] params: Value<'v>,
        heap: &'v Heap,
    ) -> anyhow::Result<Value<'v>> {
        execute_query(db_path, query, params, heap)
    }

    /// Execute INSERT/UPDATE/DELETE and return affected rows
    fn execute<'v>(
        #[allow(unused_variables)] this: Value<'v>,
        db_path: &str,
        statement: &str,
        #[starlark(default = NoneType)] params: Value<'v>,
        heap: &'v Heap,
    ) -> anyhow::Result<i32> {
        execute_statement(db_path, statement, params, heap)
    }

    /// List all tables in the database
    fn list_tables<'v>(
        #[allow(unused_variables)] this: Value<'v>,
        db_path: &str,
        heap: &'v Heap,
    ) -> anyhow::Result<Value<'v>> {
        let query = "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name";
        execute_query(db_path, query, Value::new_none(), heap)
    }

    /// Get table schema information
    fn describe_table<'v>(
        #[allow(unused_variables)] this: Value<'v>,
        db_path: &str,
        table_name: &str,
        heap: &'v Heap,
    ) -> anyhow::Result<Value<'v>> {
        // Use PRAGMA table_info which returns: cid, name, type, notnull, dflt_value, pk
        let query = format!("PRAGMA table_info({})", table_name);
        execute_query(db_path, &query, Value::new_none(), heap)
    }
}

/// Register the sqlite module in the global namespace
pub fn register(builder: &mut GlobalsBuilder) {
    const SQLITE: SqliteModule = SqliteModule;
    builder.set("sqlite", SQLITE);
}

// Helper function to execute a query and return results
fn execute_query<'v>(
    db_path: &str,
    query: &str,
    params: Value<'v>,
    heap: &'v Heap,
) -> Result<Value<'v>> {
    // Convert Starlark parameters to SQLite parameters
    let sqlite_params = convert_params_to_sqlite(params, heap)?;

    // Clone values for thread
    let db_path = db_path.to_string();
    let query_str = query.to_string();

    // Run SQLite operations in a separate thread
    let rows = std::thread::spawn(move || {
        // Open database connection (read-only for safety)
        let conn =
            Connection::open_with_flags(&db_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
                .map_err(|e| anyhow!("Failed to open SQLite database: {}", e))?;

        // Execute query
        let mut stmt = conn
            .prepare(&query_str)
            .map_err(|e| anyhow!("Failed to prepare query: {}", e))?;

        // Get column names
        let column_names: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();

        // Execute with parameters and collect rows
        let mut result_rows = Vec::new();
        let mut rows = if sqlite_params.is_empty() {
            stmt.query([])
                .map_err(|e| anyhow!("Query execution failed: {}", e))?
        } else {
            // Convert params to rusqlite::types::ToSql trait objects
            let sql_params: Vec<Box<dyn rusqlite::types::ToSql>> =
                sqlite_params.iter().map(|p| p.to_sql()).collect();
            let param_refs: Vec<&dyn rusqlite::types::ToSql> =
                sql_params.iter().map(|p| p.as_ref()).collect();

            stmt.query(&param_refs[..])
                .map_err(|e| anyhow!("Query execution failed: {}", e))?
        };

        // Collect all rows using next()
        while let Some(row) = rows
            .next()
            .map_err(|e| anyhow!("Failed to fetch row: {}", e))?
        {
            result_rows.push(row_to_values(row, &column_names)?);
        }

        Ok::<Vec<RowData>, anyhow::Error>(result_rows)
    })
    .join()
    .map_err(|e| anyhow!("Thread panicked: {:?}", e))??;

    // Convert rows to Starlark list of dicts
    rows_to_starlark(&rows, heap)
}

// Helper function to execute a statement and return affected rows
fn execute_statement<'v>(
    db_path: &str,
    statement: &str,
    params: Value<'v>,
    heap: &'v Heap,
) -> Result<i32> {
    // Convert Starlark parameters to SQLite parameters
    let sqlite_params = convert_params_to_sqlite(params, heap)?;

    // Clone values for thread
    let db_path = db_path.to_string();
    let stmt_str = statement.to_string();

    // Run SQLite operations in a separate thread
    let affected_rows = std::thread::spawn(move || {
        // Open database connection (writable)
        let conn = Connection::open(&db_path)
            .map_err(|e| anyhow!("Failed to open SQLite database: {}", e))?;

        // Execute statement
        let affected_rows = if sqlite_params.is_empty() {
            conn.execute(&stmt_str, [])
                .map_err(|e| anyhow!("Statement execution failed: {}", e))?
        } else {
            // Convert params to rusqlite::types::ToSql trait objects
            let sql_params: Vec<Box<dyn rusqlite::types::ToSql>> =
                sqlite_params.iter().map(|p| p.to_sql()).collect();
            let param_refs: Vec<&dyn rusqlite::types::ToSql> =
                sql_params.iter().map(|p| p.as_ref()).collect();

            conn.execute(&stmt_str, &param_refs[..])
                .map_err(|e| anyhow!("Statement execution failed: {}", e))?
        };

        Ok::<usize, anyhow::Error>(affected_rows)
    })
    .join()
    .map_err(|e| anyhow!("Thread panicked: {:?}", e))??;

    Ok(affected_rows as i32)
}

// Convert Starlark parameters to SQLite parameters
fn convert_params_to_sqlite<'v>(params: Value<'v>, heap: &'v Heap) -> Result<Vec<SqliteParam>> {
    let mut sqlite_params = Vec::new();

    if !params.is_none() {
        // Iterate over the list of parameters
        for param in params
            .iterate(heap)
            .map_err(|e| anyhow!("Failed to iterate parameters: {}", e))?
        {
            let sqlite_param = starlark_to_sqlite_param(param)?;
            sqlite_params.push(sqlite_param);
        }
    }

    Ok(sqlite_params)
}

// Parameter type that can be sent across threads
#[derive(Debug, Clone)]
enum SqliteParam {
    Null,
    Bool(bool),
    Int(i64),
    #[allow(dead_code)]
    Real(f64),
    Text(String),
}

impl SqliteParam {
    fn to_sql(&self) -> Box<dyn rusqlite::types::ToSql> {
        match self {
            SqliteParam::Null => Box::new(None::<String>),
            SqliteParam::Bool(b) => Box::new(*b),
            SqliteParam::Int(i) => Box::new(*i),
            SqliteParam::Real(f) => Box::new(*f),
            SqliteParam::Text(s) => Box::new(s.clone()),
        }
    }
}

// Convert a single Starlark value to a thread-safe SQLite parameter
fn starlark_to_sqlite_param(value: Value) -> Result<SqliteParam> {
    if value.is_none() {
        Ok(SqliteParam::Null)
    } else if let Some(b) = value.unpack_bool() {
        Ok(SqliteParam::Bool(b))
    } else if let Some(i) = value.unpack_i32() {
        Ok(SqliteParam::Int(i as i64))
    } else if let Some(s) = value.unpack_str() {
        Ok(SqliteParam::Text(s.to_string()))
    } else {
        // Try to convert as string fallback
        Ok(SqliteParam::Text(value.to_str()))
    }
}

// Row data that can be sent across threads
#[derive(Debug, Clone)]
struct RowData {
    columns: Vec<(String, ColumnValue)>,
}

#[derive(Debug, Clone)]
enum ColumnValue {
    Null,
    Integer(i64),
    Real(f64),
    Text(String),
    #[allow(dead_code)]
    Blob(Vec<u8>),
}

// Convert a SQLite row to thread-safe row data
fn row_to_values(row: &Row, column_names: &[String]) -> Result<RowData> {
    let mut columns = Vec::new();

    for (idx, col_name) in column_names.iter().enumerate() {
        let value = match row.get_ref(idx) {
            Ok(rusqlite::types::ValueRef::Null) => ColumnValue::Null,
            Ok(rusqlite::types::ValueRef::Integer(i)) => ColumnValue::Integer(i),
            Ok(rusqlite::types::ValueRef::Real(f)) => ColumnValue::Real(f),
            Ok(rusqlite::types::ValueRef::Text(t)) => {
                ColumnValue::Text(String::from_utf8_lossy(t).to_string())
            }
            Ok(rusqlite::types::ValueRef::Blob(b)) => ColumnValue::Blob(b.to_vec()),
            Err(e) => return Err(anyhow!("Failed to get column {} value: {}", col_name, e)),
        };

        columns.push((col_name.clone(), value));
    }

    Ok(RowData { columns })
}

// Convert SQLite rows to Starlark list of dicts
fn rows_to_starlark<'v>(rows: &[RowData], heap: &'v Heap) -> Result<Value<'v>> {
    let mut result = Vec::new();

    for row_data in rows {
        let mut row_map = SmallMap::new();

        for (col_name, value) in &row_data.columns {
            let starlark_value = match value {
                ColumnValue::Null => Value::new_none(),
                ColumnValue::Integer(i) => {
                    // Starlark uses i32, so clamp large values
                    if *i >= i32::MIN as i64 && *i <= i32::MAX as i64 {
                        heap.alloc(*i as i32)
                    } else {
                        // Convert to string for very large integers
                        heap.alloc_str(&i.to_string()).to_value()
                    }
                }
                ColumnValue::Real(f) => heap.alloc(*f),
                ColumnValue::Text(s) => heap.alloc_str(s).to_value(),
                ColumnValue::Blob(_) => {
                    // Represent blobs as a placeholder string
                    heap.alloc_str("<blob>").to_value()
                }
            };

            row_map.insert_hashed(
                heap.alloc_str(col_name)
                    .to_value()
                    .get_hashed()
                    .map_err(|e| anyhow!("Failed to hash column name: {}", e))?,
                starlark_value,
            );
        }

        result.push(heap.alloc(Dict::new(row_map)));
    }

    Ok(heap.alloc(result))
}
