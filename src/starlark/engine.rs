use anyhow::{Result, anyhow};
use starlark::environment::{FrozenModule, Globals, Module};
use starlark::eval::Evaluator;
use starlark::syntax::{AstModule, Dialect};
use starlark::values::{Value, dict::AllocDict};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

use super::mcp_types::{StarlarkExtension, extract_extension_from_value};
use super::modules::build_globals;
use crate::mcp::ToolResult;

pub struct StarlarkEngine {
    globals: Globals,
    extensions: Arc<RwLock<HashMap<String, LoadedExtension>>>,
}

struct LoadedExtension {
    extension: StarlarkExtension,
    module: FrozenModule,
}

impl Default for StarlarkEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl StarlarkEngine {
    pub fn new() -> Self {
        Self {
            globals: build_globals(),
            extensions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn load_extension(&self, name: &str, content: &str) -> Result<StarlarkExtension> {
        info!("Loading extension: {}", name);

        let ast = AstModule::parse(name, content.to_owned(), &Dialect::Standard)
            .map_err(|e| anyhow!("Parse error: {}", e))?;

        // Evaluator must be dropped before any .await to prevent non-Send errors
        let (extension, frozen_module) = {
            let module = Module::new();
            let mut eval = Evaluator::new(&module);

            let _result = eval
                .eval_module(ast, &self.globals)
                .map_err(|e| anyhow!("Eval error: {}", e))?;
            let describe_fn = module
                .get("describe_extension")
                .ok_or_else(|| anyhow!("Extension must define describe_extension()"))?;

            let extension_value = eval
                .eval_function(describe_fn, &[], &[])
                .map_err(|e| anyhow!("Function call error: {}", e))?;

            let extension = extract_extension_from_value(extension_value, module.heap())?;

            drop(eval);
            let frozen_module = module
                .freeze()
                .map_err(|e| anyhow!("Freeze error: {}", e))?;

            (extension, frozen_module)
        };

        let mut extensions = self.extensions.write().await;
        extensions.insert(
            extension.name.clone(),
            LoadedExtension {
                extension: extension.clone(),
                module: frozen_module,
            },
        );

        info!(
            "Loaded extension '{}' with {} tools",
            extension.name,
            extension.tools.len()
        );

        Ok(extension)
    }

    pub async fn get_extension(&self, name: &str) -> Option<StarlarkExtension> {
        let extensions = self.extensions.read().await;
        extensions.get(name).map(|e| e.extension.clone())
    }

    pub async fn get_all_extensions(&self) -> Vec<StarlarkExtension> {
        let extensions = self.extensions.read().await;
        extensions.values().map(|e| e.extension.clone()).collect()
    }

    pub async fn remove_extension(&self, name: &str) -> Option<StarlarkExtension> {
        info!("Removing extension: {}", name);
        let mut extensions = self.extensions.write().await;
        extensions.remove(name).map(|e| e.extension)
    }
}

#[derive(Clone)]
pub struct ToolExecutor {
    engine: Arc<StarlarkEngine>,
}

impl Default for ToolExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolExecutor {
    pub fn new() -> Self {
        Self {
            engine: Arc::new(StarlarkEngine::new()),
        }
    }

    pub fn engine(&self) -> Arc<StarlarkEngine> {
        self.engine.clone()
    }

    pub async fn execute_tool(
        &self,
        tool_name: &str,
        arguments: serde_json::Value,
    ) -> Result<ToolResult> {
        debug!("Executing tool: {}", tool_name);

        // Find the extension and tool
        let extensions = self.engine.extensions.read().await;

        let (extension_name, tool) = extensions
            .iter()
            .find_map(|(ext_name, loaded_ext)| {
                loaded_ext
                    .extension
                    .tools
                    .iter()
                    .find(|t| t.name == tool_name)
                    .map(|t| (ext_name.clone(), t.clone()))
            })
            .ok_or_else(|| anyhow!("Tool not found: {}", tool_name))?;

        let loaded_ext = extensions
            .get(&extension_name)
            .ok_or_else(|| anyhow!("Extension not found: {}", extension_name))?;

        super::modules::set_exec_whitelist(loaded_ext.extension.allowed_exec.clone());

        let module = Module::new();

        // Extract just the function name (remove module prefix if present)
        let function_name = tool
            .handler_name
            .split('.')
            .next_back()
            .unwrap_or(&tool.handler_name);

        let handler_frozen = loaded_ext
            .module
            .get(function_name)
            .map_err(|e| anyhow!("Handler lookup error for '{}': {}", function_name, e))?;

        let mut eval = Evaluator::new(&module);

        let heap = module.heap();
        let params_dict = json_to_starlark_value(arguments, heap)?;
        let handler = handler_frozen.value();
        let result_value = eval
            .eval_function(handler, &[params_dict], &[])
            .map_err(|e| {
                super::modules::clear_exec_whitelist();
                anyhow!("Handler execution error: {}", e)
            })?;

        super::modules::clear_exec_whitelist();

        let result_json = starlark_value_to_json(result_value, heap)?;
        let tool_result: ToolResult = serde_json::from_value(result_json)?;

        Ok(tool_result)
    }
}

fn json_to_starlark_value<'v>(
    json: serde_json::Value,
    heap: &'v starlark::values::Heap,
) -> Result<Value<'v>> {
    match json {
        serde_json::Value::Null => Ok(Value::new_none()),
        serde_json::Value::Bool(b) => Ok(Value::new_bool(b)),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(heap.alloc(i))
            } else if let Some(f) = n.as_f64() {
                Ok(heap.alloc(f))
            } else {
                Err(anyhow!("Invalid number"))
            }
        }
        serde_json::Value::String(s) => Ok(heap.alloc(s)),
        serde_json::Value::Array(arr) => {
            let values: Result<Vec<_>> = arr
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

fn starlark_value_to_json<'v>(
    value: Value<'v>,
    heap: &'v starlark::values::Heap,
) -> Result<serde_json::Value> {
    if value.is_none() {
        Ok(serde_json::Value::Null)
    } else if let Some(b) = value.unpack_bool() {
        Ok(serde_json::Value::Bool(b))
    } else if let Some(i) = value.unpack_i32() {
        Ok(serde_json::Value::Number(i.into()))
    } else if value.get_type() == "float" {
        // Handle float type - parse from string representation
        let float_str = value.to_string();
        let f: f64 = float_str
            .parse()
            .map_err(|e| anyhow!("Failed to parse float '{}': {}", float_str, e))?;
        Ok(serde_json::Number::from_f64(f)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null))
    } else if let Some(s) = value.unpack_str() {
        Ok(serde_json::Value::String(s.to_string()))
    } else {
        // Check type to see if it's a dict
        let type_name = value.get_type();
        if type_name == "dict" {
            // Try to iterate keys and get values
            let mut map = serde_json::Map::new();

            // Get all keys by iterating the dict
            for key in value
                .iterate(heap)
                .map_err(|e| anyhow!("Dict iterate error: {}", e))?
            {
                let key_str = key
                    .unpack_str()
                    .ok_or_else(|| anyhow!("Dict keys must be strings, got: {}", key))?;

                let val = value
                    .at(key, heap)
                    .map_err(|e| anyhow!("Error getting dict value: {}", e))?;

                map.insert(key_str.to_string(), starlark_value_to_json(val, heap)?);
            }
            return Ok(serde_json::Value::Object(map));
        }
        if let Ok(iter) = value.iterate(heap) {
            let mut arr = Vec::new();
            for item in iter {
                arr.push(starlark_value_to_json(item, heap)?);
            }
            return Ok(serde_json::Value::Array(arr));
        }

        Err(anyhow!("Unsupported Starlark type: {}", value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use starlark::environment::Module;

    fn eval_to_json(code: &str) -> Result<serde_json::Value> {
        let module = Module::new();
        let globals = Globals::standard();
        let mut eval = Evaluator::new(&module);

        let ast = AstModule::parse("test", code.to_owned(), &Dialect::Standard)
            .map_err(|e| anyhow!("Parse error: {}", e))?;

        let value = eval
            .eval_module(ast, &globals)
            .map_err(|e| anyhow!("Eval error: {}", e))?;

        starlark_value_to_json(value, module.heap())
    }

    #[test]
    fn test_starlark_value_to_json_none() {
        let result = eval_to_json("None").unwrap();
        assert_eq!(result, json!(null));
    }

    #[test]
    fn test_starlark_value_to_json_bool_true() {
        let result = eval_to_json("True").unwrap();
        assert_eq!(result, json!(true));
    }

    #[test]
    fn test_starlark_value_to_json_bool_false() {
        let result = eval_to_json("False").unwrap();
        assert_eq!(result, json!(false));
    }

    #[test]
    fn test_starlark_value_to_json_integer() {
        let result = eval_to_json("42").unwrap();
        assert_eq!(result, json!(42));
    }

    #[test]
    fn test_starlark_value_to_json_negative_integer() {
        let result = eval_to_json("-123").unwrap();
        assert_eq!(result, json!(-123));
    }

    #[test]
    fn test_starlark_value_to_json_float() {
        let result = eval_to_json("3.14").unwrap();
        assert_eq!(result, json!(3.14));
    }

    #[test]
    fn test_starlark_value_to_json_negative_float() {
        let result = eval_to_json("-2.5").unwrap();
        assert_eq!(result, json!(-2.5));
    }

    #[test]
    fn test_starlark_value_to_json_float_from_division() {
        let result = eval_to_json("10 / 4").unwrap();
        assert_eq!(result, json!(2.5));
    }

    #[test]
    fn test_starlark_value_to_json_string() {
        let result = eval_to_json("\"hello world\"").unwrap();
        assert_eq!(result, json!("hello world"));
    }

    #[test]
    fn test_starlark_value_to_json_empty_string() {
        let result = eval_to_json("\"\"").unwrap();
        assert_eq!(result, json!(""));
    }

    #[test]
    fn test_starlark_value_to_json_list() {
        let result = eval_to_json("[1, 2, 3]").unwrap();
        assert_eq!(result, json!([1, 2, 3]));
    }

    #[test]
    fn test_starlark_value_to_json_empty_list() {
        let result = eval_to_json("[]").unwrap();
        assert_eq!(result, json!([]));
    }

    #[test]
    fn test_starlark_value_to_json_mixed_list() {
        let result = eval_to_json("[1, \"two\", True, None]").unwrap();
        assert_eq!(result, json!([1, "two", true, null]));
    }

    #[test]
    fn test_starlark_value_to_json_dict() {
        let result = eval_to_json("{\"a\": 1, \"b\": 2}").unwrap();
        assert_eq!(result, json!({"a": 1, "b": 2}));
    }

    #[test]
    fn test_starlark_value_to_json_empty_dict() {
        let result = eval_to_json("{}").unwrap();
        assert_eq!(result, json!({}));
    }

    #[test]
    fn test_starlark_value_to_json_nested_dict() {
        let result = eval_to_json("{\"outer\": {\"inner\": 42}}").unwrap();
        assert_eq!(result, json!({"outer": {"inner": 42}}));
    }

    #[test]
    fn test_starlark_value_to_json_dict_with_list() {
        let result = eval_to_json("{\"items\": [1, 2, 3]}").unwrap();
        assert_eq!(result, json!({"items": [1, 2, 3]}));
    }

    #[test]
    fn test_starlark_value_to_json_list_with_dict() {
        let result = eval_to_json("[{\"a\": 1}, {\"b\": 2}]").unwrap();
        assert_eq!(result, json!([{"a": 1}, {"b": 2}]));
    }

    #[test]
    fn test_starlark_value_to_json_complex_nested() {
        let result = eval_to_json(
            r#"{
                "name": "test",
                "count": 42,
                "enabled": True,
                "data": [1, 2.5, "three"],
                "nested": {"x": 10, "y": 20}
            }"#,
        )
        .unwrap();

        assert_eq!(result["name"], json!("test"));
        assert_eq!(result["count"], json!(42));
        assert_eq!(result["enabled"], json!(true));
        assert_eq!(result["data"], json!([1, 2.5, "three"]));
        assert_eq!(result["nested"], json!({"x": 10, "y": 20}));
    }

    #[test]
    fn test_starlark_value_to_json_float_precision() {
        // Test that float precision is maintained
        let result = eval_to_json("15.8").unwrap();
        assert_eq!(result, json!(15.8));

        let result = eval_to_json("0.1 + 0.2").unwrap();
        // Note: floating point arithmetic means this won't be exactly 0.3
        assert!(result.as_f64().unwrap() > 0.29 && result.as_f64().unwrap() < 0.31);
    }
}
