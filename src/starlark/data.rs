use allocative::Allocative;
use derive_more::Display;
use starlark::environment::{GlobalsBuilder, Methods, MethodsBuilder, MethodsStatic};
use starlark::starlark_module;
use starlark::starlark_simple_value;
use starlark::values::dict::AllocDict;
use starlark::values::starlark_value;
use starlark::values::{Heap, NoSerialize, ProvidesStaticType, StarlarkValue, Value};
use std::cell::RefCell;
use std::path::Path;

thread_local! {
    /// Thread-local storage for the extensions directory path
    /// Set by the tool executor before calling tool handler functions
    static EXTENSIONS_DIR: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// Set the extensions directory for the current thread
pub fn set_extensions_dir(dir: String) {
    EXTENSIONS_DIR.with(|d| {
        *d.borrow_mut() = Some(dir);
    });
}

/// Clear the extensions directory for the current thread
pub fn clear_extensions_dir() {
    EXTENSIONS_DIR.with(|d| {
        *d.borrow_mut() = None;
    });
}

/// Get the current extensions directory
fn get_extensions_dir() -> Option<String> {
    EXTENSIONS_DIR.with(|d| d.borrow().clone())
}

#[derive(Debug, Display, Allocative, ProvidesStaticType, NoSerialize)]
#[display(fmt = "data")]
pub struct DataModule;

starlark_simple_value!(DataModule);

#[starlark_value(type = "data")]
impl<'v> StarlarkValue<'v> for DataModule {
    fn get_methods() -> Option<&'static Methods> {
        static RES: MethodsStatic = MethodsStatic::new();
        RES.methods(data_methods)
    }

    fn dir_attr(&self) -> Vec<String> {
        vec!["load_json".to_owned()]
    }
}

#[starlark_module]
fn data_methods(builder: &mut MethodsBuilder) {
    /// Load a JSON file from the extensions directory and return its contents as a Starlark value.
    ///
    /// # Arguments
    /// * `path` - Path to the JSON file, relative to the extensions directory
    ///
    /// # Examples
    /// ```python
    /// items = data.load_json("data.json")
    /// ```
    ///
    /// # Security
    /// Only paths within the extensions directory are allowed. Path traversal (e.g., "../") is rejected.
    fn load_json<'v>(
        #[allow(unused_variables)] this: Value<'v>,
        path: &str,
        heap: &'v Heap,
    ) -> anyhow::Result<Value<'v>> {
        let extensions_dir = get_extensions_dir().ok_or_else(|| {
            anyhow::anyhow!("data.load_json: extensions directory not configured")
        })?;

        if path.contains("..") {
            return Err(anyhow::anyhow!(
                "data.load_json: path traversal not allowed: {}",
                path
            ));
        }

        let ext_path = Path::new(&extensions_dir);
        let full_path = ext_path.join(path);

        let canonical_ext = ext_path.canonicalize().map_err(|e| {
            anyhow::anyhow!(
                "data.load_json: failed to resolve extensions directory: {}",
                e
            )
        })?;

        let canonical_file = full_path.canonicalize().map_err(|e| {
            anyhow::anyhow!(
                "data.load_json: failed to resolve file path '{}': {}",
                path,
                e
            )
        })?;

        if !canonical_file.starts_with(&canonical_ext) {
            return Err(anyhow::anyhow!(
                "data.load_json: path must be within extensions directory"
            ));
        }

        let content = std::fs::read_to_string(&canonical_file).map_err(|e| {
            anyhow::anyhow!("data.load_json: failed to read file '{}': {}", path, e)
        })?;

        let json_value: serde_json::Value = serde_json::from_str(&content).map_err(|e| {
            anyhow::anyhow!("data.load_json: failed to parse JSON in '{}': {}", path, e)
        })?;

        json_to_starlark_value(json_value, heap)
    }
}

/// Convert a serde_json::Value to a Starlark Value
fn json_to_starlark_value<'v>(
    json: serde_json::Value,
    heap: &'v Heap,
) -> anyhow::Result<Value<'v>> {
    match json {
        serde_json::Value::Null => Ok(Value::new_none()),
        serde_json::Value::Bool(b) => Ok(Value::new_bool(b)),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(heap.alloc(i))
            } else if let Some(f) = n.as_f64() {
                Ok(heap.alloc(f))
            } else {
                Err(anyhow::anyhow!("Invalid number"))
            }
        }
        serde_json::Value::String(s) => Ok(heap.alloc(s)),
        serde_json::Value::Array(arr) => {
            let values: anyhow::Result<Vec<_>> = arr
                .into_iter()
                .map(|v| json_to_starlark_value(v, heap))
                .collect();
            Ok(heap.alloc(values?))
        }
        serde_json::Value::Object(obj) => {
            let mut dict_items = Vec::new();
            for (k, v) in obj {
                let key = heap.alloc(k);
                let value = json_to_starlark_value(v, heap)?;
                dict_items.push((key, value));
            }
            Ok(heap.alloc(AllocDict(dict_items)))
        }
    }
}

pub fn register(builder: &mut GlobalsBuilder) {
    const DATA: DataModule = DataModule;
    builder.set("data", DATA);
}

#[cfg(test)]
mod tests {
    use super::*;
    use starlark::environment::GlobalsBuilder;
    use starlark::eval::Evaluator;
    use starlark::syntax::{AstModule, Dialect};
    use std::io::Write;
    use tempfile::TempDir;

    fn setup_test_env() -> TempDir {
        let temp_dir = TempDir::new().unwrap();

        let json_content = r#"[
            {"name": "Item 1", "value": 100},
            {"name": "Item 2", "value": 200}
        ]"#;

        let json_path = temp_dir.path().join("test_items.json");
        let mut file = std::fs::File::create(&json_path).unwrap();
        file.write_all(json_content.as_bytes()).unwrap();

        temp_dir
    }

    fn eval_with_data(code: &str, extensions_dir: &str) -> Result<String, starlark::Error> {
        set_extensions_dir(extensions_dir.to_string());

        let globals = GlobalsBuilder::new().with(register).build();
        let module = starlark::environment::Module::new();
        let ast = AstModule::parse("test.star", code.to_owned(), &Dialect::Standard)?;
        let mut eval = Evaluator::new(&module);
        let result = eval.eval_module(ast, &globals)?;

        clear_extensions_dir();

        Ok(result.to_string())
    }

    #[test]
    fn test_load_json_basic() {
        let temp_dir = setup_test_env();
        let result = eval_with_data(
            "data.load_json(\"test_items.json\")",
            temp_dir.path().to_str().unwrap(),
        )
        .unwrap();

        assert!(result.contains("Item 1"));
        assert!(result.contains("Item 2"));
    }

    #[test]
    fn test_load_json_path_traversal_rejected() {
        let temp_dir = setup_test_env();
        let result = eval_with_data(
            "data.load_json(\"../etc/passwd\")",
            temp_dir.path().to_str().unwrap(),
        );

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("path traversal not allowed"));
    }

    #[test]
    fn test_load_json_file_not_found() {
        let temp_dir = setup_test_env();
        let result = eval_with_data(
            "data.load_json(\"nonexistent.json\")",
            temp_dir.path().to_str().unwrap(),
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_dir_attr() {
        let module = DataModule;
        let attrs = module.dir_attr();
        assert!(attrs.contains(&"load_json".to_owned()));
    }
}
