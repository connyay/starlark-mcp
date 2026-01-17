use anyhow::anyhow;
use starlark::environment::GlobalsBuilder;
use starlark::starlark_module;
use starlark::values::dict::AllocDict;
use starlark::values::{Heap, Value, none::NoneType};

use crate::mcp::{Tool, ToolInputSchema};

// Extension type - represents a loaded Starlark extension
#[derive(Debug, Clone)]
pub struct StarlarkExtension {
    pub name: String,
    pub version: String,
    pub description: String,
    pub tools: Vec<StarlarkTool>,
    pub allowed_exec: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct StarlarkTool {
    pub name: String,
    pub description: String,
    pub handler_name: String,
    pub parameters: Vec<StarlarkToolParameter>,
}

#[derive(Debug, Clone)]
pub struct StarlarkToolParameter {
    pub name: String,
    pub param_type: String,
    pub required: bool,
    pub default: Option<String>,
    pub description: String,
}

// MCP globals for Starlark
#[starlark_module]
#[allow(clippy::type_complexity)]
pub fn mcp_globals(builder: &mut GlobalsBuilder) {
    fn Extension<'v>(
        name: String,
        version: String,
        description: String,
        tools: Value<'v>,
        #[starlark(default = NoneType)] allowed_exec: Value<'v>,
        heap: &'v Heap,
    ) -> anyhow::Result<Value<'v>> {
        // Create a dict to return using the allocator
        let dict_items = vec![
            (heap.alloc("name"), heap.alloc(name)),
            (heap.alloc("version"), heap.alloc(version)),
            (heap.alloc("description"), heap.alloc(description)),
            (heap.alloc("tools"), tools),
            (heap.alloc("allowed_exec"), allowed_exec),
        ];

        Ok(heap.alloc(AllocDict(dict_items)))
    }

    fn Tool<'v>(
        name: String,
        description: String,
        #[starlark(default = NoneType)] parameters: Value<'v>,
        handler: Value<'v>,
        heap: &'v Heap,
    ) -> anyhow::Result<Value<'v>> {
        // Create a dict to return using the allocator
        let dict_items = vec![
            (heap.alloc("name"), heap.alloc(name)),
            (heap.alloc("description"), heap.alloc(description)),
            (heap.alloc("parameters"), parameters),
            (heap.alloc("handler"), handler),
        ];

        Ok(heap.alloc(AllocDict(dict_items)))
    }

    fn ToolParameter<'v>(
        name: String,
        param_type: String, // Will be passed with keyword "type" from Starlark
        required: bool,
        #[starlark(default = NoneType)] default: Value<'v>,
        description: String,
        heap: &'v Heap,
    ) -> anyhow::Result<Value<'v>> {
        // Create a dict to return using the allocator
        let dict_items = vec![
            (heap.alloc("name"), heap.alloc(name)),
            (heap.alloc("type"), heap.alloc(param_type)),
            (heap.alloc("required"), heap.alloc(required)),
            (heap.alloc("default"), default),
            (heap.alloc("description"), heap.alloc(description)),
        ];

        Ok(heap.alloc(AllocDict(dict_items)))
    }
}

pub fn extract_extension_from_value<'v>(
    value: Value<'v>,
    heap: &'v Heap,
) -> anyhow::Result<StarlarkExtension> {
    // Get dict items via indexing
    let name_val = value
        .at(heap.alloc("name"), heap)
        .map_err(|e| anyhow!("Extension error getting 'name': {}", e))?;
    let name = name_val
        .unpack_str()
        .ok_or_else(|| anyhow!("Extension 'name' must be a string"))?
        .to_string();

    let version_val = value
        .at(heap.alloc("version"), heap)
        .map_err(|e| anyhow!("Extension error getting 'version': {}", e))?;
    let version = version_val
        .unpack_str()
        .ok_or_else(|| anyhow!("Extension 'version' must be a string"))?
        .to_string();

    let description_val = value
        .at(heap.alloc("description"), heap)
        .map_err(|e| anyhow!("Extension error getting 'description': {}", e))?;
    let description = description_val
        .unpack_str()
        .ok_or_else(|| anyhow!("Extension 'description' must be a string"))?
        .to_string();

    let tools_value = value
        .at(heap.alloc("tools"), heap)
        .map_err(|e| anyhow!("Extension error getting 'tools': {}", e))?;

    let mut tools = Vec::new();
    for tool_value in tools_value
        .iterate(heap)
        .map_err(|e| anyhow!("Tools iterate error: {}", e))?
    {
        let tool_name_val = tool_value
            .at(heap.alloc("name"), heap)
            .map_err(|e| anyhow!("Tool error getting 'name': {}", e))?;
        let tool_name = tool_name_val
            .unpack_str()
            .ok_or_else(|| anyhow!("Tool 'name' must be a string"))?
            .to_string();

        let tool_desc_val = tool_value
            .at(heap.alloc("description"), heap)
            .map_err(|e| anyhow!("Tool error getting 'description': {}", e))?;
        let tool_description = tool_desc_val
            .unpack_str()
            .ok_or_else(|| anyhow!("Tool 'description' must be a string"))?
            .to_string();

        let handler = tool_value
            .at(heap.alloc("handler"), heap)
            .map_err(|e| anyhow!("Tool error getting 'handler': {}", e))?;

        let handler_name = handler.to_string();

        // Extract parameters if present
        let mut parameters = Vec::new();
        if let Ok(params_value) = tool_value.at(heap.alloc("parameters"), heap)
            && !params_value.is_none()
        {
            for param_value in params_value
                .iterate(heap)
                .map_err(|e| anyhow!("Parameters iterate error: {}", e))?
            {
                let param_name = param_value
                    .at(heap.alloc("name"), heap)
                    .map_err(|e| anyhow!("Parameter 'name' error: {}", e))?
                    .unpack_str()
                    .ok_or_else(|| anyhow!("Parameter 'name' must be a string"))?
                    .to_string();

                let param_type = param_value
                    .at(heap.alloc("type"), heap)
                    .map_err(|e| anyhow!("Parameter 'type' error: {}", e))?
                    .unpack_str()
                    .ok_or_else(|| anyhow!("Parameter 'type' must be a string"))?
                    .to_string();

                let required = param_value
                    .at(heap.alloc("required"), heap)
                    .map_err(|e| anyhow!("Parameter 'required' error: {}", e))?
                    .unpack_bool()
                    .ok_or_else(|| anyhow!("Parameter 'required' must be a boolean"))?;

                let default = if let Ok(default_val) = param_value.at(heap.alloc("default"), heap) {
                    if !default_val.is_none() {
                        Some(default_val.to_str())
                    } else {
                        None
                    }
                } else {
                    None
                };

                let description = param_value
                    .at(heap.alloc("description"), heap)
                    .map_err(|e| anyhow!("Parameter 'description' error: {}", e))?
                    .unpack_str()
                    .unwrap_or("")
                    .to_string();

                parameters.push(StarlarkToolParameter {
                    name: param_name,
                    param_type,
                    required,
                    default,
                    description,
                });
            }
        }

        tools.push(StarlarkTool {
            name: tool_name,
            description: tool_description,
            handler_name,
            parameters,
        });
    }

    // Extract allowed_exec if present
    let allowed_exec = if let Ok(allowed_exec_value) = value.at(heap.alloc("allowed_exec"), heap) {
        if !allowed_exec_value.is_none() {
            let mut exec_list = Vec::new();
            for cmd in allowed_exec_value
                .iterate(heap)
                .map_err(|e| anyhow!("Failed to iterate allowed_exec: {}", e))?
            {
                exec_list.push(cmd.to_str().to_string());
            }
            exec_list
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };

    Ok(StarlarkExtension {
        name,
        version,
        description,
        tools,
        allowed_exec,
    })
}

impl StarlarkExtension {
    pub fn to_mcp_tools(&self) -> Vec<Tool> {
        self.tools
            .iter()
            .map(|t| {
                // Build properties map for JSON schema
                let mut properties = serde_json::Map::new();
                let mut required = Vec::new();

                for param in &t.parameters {
                    let mut prop = serde_json::Map::new();

                    // Map Starlark types to JSON Schema types
                    let json_type = match param.param_type.as_str() {
                        "string" => "string",
                        "integer" | "int" => "integer",
                        "float" | "number" => "number",
                        "boolean" | "bool" => "boolean",
                        _ => "string", // Default to string
                    };
                    prop.insert(
                        "type".to_string(),
                        serde_json::Value::String(json_type.to_string()),
                    );

                    if !param.description.is_empty() {
                        prop.insert(
                            "description".to_string(),
                            serde_json::Value::String(param.description.clone()),
                        );
                    }

                    if let Some(ref default_val) = param.default {
                        // Try to parse the default value appropriately
                        let default = match param.param_type.as_str() {
                            "integer" | "int" => default_val
                                .parse::<i64>()
                                .map(|n| serde_json::Value::Number(serde_json::Number::from(n)))
                                .unwrap_or_else(|_| serde_json::Value::String(default_val.clone())),
                            "boolean" | "bool" => default_val
                                .parse::<bool>()
                                .map(serde_json::Value::Bool)
                                .unwrap_or_else(|_| serde_json::Value::String(default_val.clone())),
                            _ => serde_json::Value::String(default_val.clone()),
                        };
                        prop.insert("default".to_string(), default);
                    }

                    properties.insert(param.name.clone(), serde_json::Value::Object(prop));

                    if param.required {
                        required.push(param.name.clone());
                    }
                }

                Tool {
                    name: t.name.clone(),
                    description: t.description.clone(),
                    input_schema: ToolInputSchema {
                        schema_type: "object".to_string(),
                        properties: properties.into_iter().collect(),
                        required,
                    },
                }
            })
            .collect()
    }
}
