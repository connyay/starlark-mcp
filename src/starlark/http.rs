use allocative::Allocative;
use anyhow::{anyhow, Result};
use derive_more::Display;
use reqwest::blocking::{Client, Response};
use reqwest::header::CONTENT_TYPE;
use serde_json::Value as JsonValue;
use starlark::collections::SmallMap;
use starlark::environment::{GlobalsBuilder, Methods, MethodsBuilder, MethodsStatic};
use starlark::starlark_module;
use starlark::starlark_simple_value;
use starlark::values::starlark_value;
use starlark::values::{
    dict::{Dict, DictRef},
    none::NoneType,
    tuple::TupleRef,
    Heap, NoSerialize, ProvidesStaticType, StarlarkValue, Value, ValueLike,
};
use std::collections::HashMap;
use url::Url;

// HTTP client - we'll use a global client for connection pooling
lazy_static::lazy_static! {
    static ref HTTP_CLIENT: Client = Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .expect("Failed to create HTTP client");
}

/// HTTP module for making HTTP requests
#[derive(Debug, Display, Allocative, ProvidesStaticType, NoSerialize)]
#[display(fmt = "http")]
pub struct HttpModule;

starlark_simple_value!(HttpModule);

#[starlark_value(type = "http")]
impl<'v> StarlarkValue<'v> for HttpModule {
    fn get_methods() -> Option<&'static Methods> {
        static RES: MethodsStatic = MethodsStatic::new();
        RES.methods(http_methods)
    }
}

/// Methods available on the http module
#[starlark_module]
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn http_methods(builder: &mut MethodsBuilder) {
    /// Make an HTTP GET request
    fn get<'v>(
        #[allow(unused_variables)] this: Value<'v>,
        url: &str,
        #[starlark(default = NoneType)] params: Value<'v>,
        #[starlark(default = NoneType)] headers: Value<'v>,
        #[starlark(default = NoneType)] auth: Value<'v>,
        heap: &'v Heap,
    ) -> anyhow::Result<Value<'v>> {
        make_request("GET", url, params, headers, auth, None, heap)
    }

    /// Make an HTTP POST request
    fn post<'v>(
        #[allow(unused_variables)] this: Value<'v>,
        url: &str,
        #[starlark(default = NoneType)] params: Value<'v>,
        #[starlark(default = NoneType)] headers: Value<'v>,
        #[starlark(default = NoneType)] body: Value<'v>,
        #[starlark(default = NoneType)] json_body: Value<'v>,
        #[starlark(default = NoneType)] form_body: Value<'v>,
        #[starlark(default = NoneType)] auth: Value<'v>,
        heap: &'v Heap,
    ) -> anyhow::Result<Value<'v>> {
        make_request_with_body(
            "POST", url, params, headers, auth, body, json_body, form_body, heap,
        )
    }

    /// Make an HTTP PUT request
    fn put<'v>(
        #[allow(unused_variables)] this: Value<'v>,
        url: &str,
        #[starlark(default = NoneType)] params: Value<'v>,
        #[starlark(default = NoneType)] headers: Value<'v>,
        #[starlark(default = NoneType)] body: Value<'v>,
        #[starlark(default = NoneType)] json_body: Value<'v>,
        #[starlark(default = NoneType)] form_body: Value<'v>,
        #[starlark(default = NoneType)] auth: Value<'v>,
        heap: &'v Heap,
    ) -> anyhow::Result<Value<'v>> {
        make_request_with_body(
            "PUT", url, params, headers, auth, body, json_body, form_body, heap,
        )
    }

    /// Make an HTTP PATCH request
    fn patch<'v>(
        #[allow(unused_variables)] this: Value<'v>,
        url: &str,
        #[starlark(default = NoneType)] params: Value<'v>,
        #[starlark(default = NoneType)] headers: Value<'v>,
        #[starlark(default = NoneType)] body: Value<'v>,
        #[starlark(default = NoneType)] json_body: Value<'v>,
        #[starlark(default = NoneType)] form_body: Value<'v>,
        #[starlark(default = NoneType)] auth: Value<'v>,
        heap: &'v Heap,
    ) -> anyhow::Result<Value<'v>> {
        make_request_with_body(
            "PATCH", url, params, headers, auth, body, json_body, form_body, heap,
        )
    }

    /// Make an HTTP DELETE request
    fn delete<'v>(
        #[allow(unused_variables)] this: Value<'v>,
        url: &str,
        #[starlark(default = NoneType)] params: Value<'v>,
        #[starlark(default = NoneType)] headers: Value<'v>,
        #[starlark(default = NoneType)] auth: Value<'v>,
        heap: &'v Heap,
    ) -> anyhow::Result<Value<'v>> {
        make_request("DELETE", url, params, headers, auth, None, heap)
    }

    /// Make an HTTP OPTIONS request
    fn options<'v>(
        #[allow(unused_variables)] this: Value<'v>,
        url: &str,
        #[starlark(default = NoneType)] params: Value<'v>,
        #[starlark(default = NoneType)] headers: Value<'v>,
        #[starlark(default = NoneType)] auth: Value<'v>,
        heap: &'v Heap,
    ) -> anyhow::Result<Value<'v>> {
        make_request("OPTIONS", url, params, headers, auth, None, heap)
    }
}

/// Register the http module in the global namespace
pub fn register(builder: &mut GlobalsBuilder) {
    const HTTP: HttpModule = HttpModule;
    builder.set("http", HTTP);
}

// Helper function for requests without body
fn make_request<'v>(
    method: &str,
    url: &str,
    params: Value<'v>,
    headers: Value<'v>,
    auth: Value<'v>,
    body: Option<String>,
    heap: &'v Heap,
) -> Result<Value<'v>> {
    make_request_with_body(
        method,
        url,
        params,
        headers,
        auth,
        body.map(|b| heap.alloc(b)).unwrap_or(Value::new_none()),
        Value::new_none(),
        Value::new_none(),
        heap,
    )
}

// Main request function with all options
#[allow(clippy::too_many_arguments)]
fn make_request_with_body<'v>(
    method: &str,
    url: &str,
    params: Value<'v>,
    headers: Value<'v>,
    auth: Value<'v>,
    body: Value<'v>,
    json_body: Value<'v>,
    form_body: Value<'v>,
    heap: &'v Heap,
) -> Result<Value<'v>> {
    // Build URL with params
    let mut url = Url::parse(url).map_err(|e| anyhow!("Invalid URL: {}", e))?;

    // Add query parameters
    if !params.is_none() {
        if let Some(dict) = DictRef::from_value(params) {
            let mut query_pairs = url.query_pairs_mut();
            for (key, value) in dict.iter() {
                query_pairs.append_pair(&key.to_str(), &value.to_str());
            }
        } else {
            return Err(anyhow!("params must be a dict, got: {}", params.get_type()));
        }
    }

    // Create request builder
    let mut request = match method {
        "GET" => HTTP_CLIENT.get(url.as_str()),
        "POST" => HTTP_CLIENT.post(url.as_str()),
        "PUT" => HTTP_CLIENT.put(url.as_str()),
        "PATCH" => HTTP_CLIENT.patch(url.as_str()),
        "DELETE" => HTTP_CLIENT.delete(url.as_str()),
        "OPTIONS" => HTTP_CLIENT.request(reqwest::Method::OPTIONS, url.as_str()),
        _ => return Err(anyhow!("Unsupported HTTP method: {}", method)),
    };

    // Add headers
    if !headers.is_none() {
        // Check if headers is a dict
        if let Some(dict) = DictRef::from_value(headers) {
            for (key, value) in dict.iter() {
                request = request.header(key.to_str(), value.to_str());
            }
        } else {
            return Err(anyhow!(
                "headers must be a dict, got: {}",
                headers.get_type()
            ));
        }
    }

    // Add authentication
    if !auth.is_none() {
        let auth_list: Vec<String> = auth
            .iterate(heap)
            .map_err(|e| anyhow!("Failed to iterate auth: {}", e))?
            .map(|v| v.to_str())
            .collect();

        if auth_list.len() == 2 {
            request = request.basic_auth(&auth_list[0], Some(&auth_list[1]));
        } else {
            return Err(anyhow!("auth must be a tuple of (username, password)"));
        }
    }

    // Set body
    if !json_body.is_none() {
        // Convert Starlark value to JSON
        let json_str = starlark_to_json_string(json_body, heap)?;
        request = request
            .header(CONTENT_TYPE, "application/json")
            .body(json_str);
    } else if !form_body.is_none() {
        // Form-encoded body
        if let Some(dict) = DictRef::from_value(form_body) {
            let mut form_data = HashMap::new();
            for (key, value) in dict.iter() {
                form_data.insert(key.to_str(), value.to_str());
            }
            request = request.form(&form_data);
        } else {
            return Err(anyhow!(
                "form_body must be a dict, got: {}",
                form_body.get_type()
            ));
        }
    } else if !body.is_none() {
        // Raw string body
        let body_str = body.to_str();
        request = request.body(body_str);
    }

    // Execute request
    let response = request
        .send()
        .map_err(|e| anyhow!("Request failed: {}", e))?;

    // Convert response to Starlark value
    response_to_starlark(response, heap)
}

// Convert response to Starlark dict
fn response_to_starlark<'v>(response: Response, heap: &'v Heap) -> Result<Value<'v>> {
    let status = response.status().as_u16() as i32;
    let url = response.url().to_string();

    // Convert headers
    let mut headers_map = SmallMap::new();
    for (key, value) in response.headers() {
        let key_str = key.as_str();
        let value_str = value.to_str().unwrap_or("");
        headers_map.insert_hashed(
            heap.alloc_str(key_str)
                .to_value()
                .get_hashed()
                .map_err(|e| anyhow!("Failed to hash header key: {}", e))?,
            heap.alloc_str(value_str).to_value(),
        );
    }
    let headers_dict = heap.alloc(Dict::new(headers_map));

    // Get body as text
    let body_text = response
        .text()
        .map_err(|e| anyhow!("Failed to read response body: {}", e))?;

    // Build response dict
    let mut result = SmallMap::new();

    result.insert_hashed(
        heap.alloc_str("url")
            .to_value()
            .get_hashed()
            .map_err(|e| anyhow!("Failed to hash key: {}", e))?,
        heap.alloc_str(&url).to_value(),
    );

    result.insert_hashed(
        heap.alloc_str("status_code")
            .to_value()
            .get_hashed()
            .map_err(|e| anyhow!("Failed to hash key: {}", e))?,
        heap.alloc(status),
    );

    result.insert_hashed(
        heap.alloc_str("headers")
            .to_value()
            .get_hashed()
            .map_err(|e| anyhow!("Failed to hash key: {}", e))?,
        headers_dict.to_value(),
    );

    // Add body as string
    result.insert_hashed(
        heap.alloc_str("body")
            .to_value()
            .get_hashed()
            .map_err(|e| anyhow!("Failed to hash key: {}", e))?,
        heap.alloc_str(&body_text).to_value(),
    );

    // Try to parse as JSON and add json field
    if let Ok(json_value) = serde_json::from_str::<JsonValue>(&body_text) {
        let starlark_json = json_to_starlark(&json_value, heap)?;
        result.insert_hashed(
            heap.alloc_str("json")
                .to_value()
                .get_hashed()
                .map_err(|e| anyhow!("Failed to hash key: {}", e))?,
            starlark_json,
        );
    }

    Ok(heap.alloc(Dict::new(result)))
}

// Convert JSON value to Starlark value
fn json_to_starlark<'v>(json: &JsonValue, heap: &'v Heap) -> Result<Value<'v>> {
    match json {
        JsonValue::Null => Ok(Value::new_none()),
        JsonValue::Bool(b) => Ok(heap.alloc(*b)),
        JsonValue::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(heap.alloc(i))
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
                        .map_err(|e| anyhow!("Failed to hash key: {}", e))?,
                    starlark_value,
                );
            }
            Ok(heap.alloc(Dict::new(map)))
        }
    }
}

// Convert Starlark value to JSON string
fn starlark_to_json_string<'v>(value: Value<'v>, heap: &'v Heap) -> Result<String> {
    let json_value = starlark_to_json(value, heap)?;
    serde_json::to_string(&json_value).map_err(|e| anyhow!("Failed to serialize to JSON: {}", e))
}

// Convert Starlark value to serde_json::Value
fn starlark_to_json<'v>(value: Value<'v>, heap: &'v Heap) -> Result<JsonValue> {
    if value.is_none() {
        Ok(JsonValue::Null)
    } else if let Some(b) = value.unpack_bool() {
        Ok(JsonValue::Bool(b))
    } else if let Some(i) = value.unpack_i32() {
        Ok(JsonValue::Number((i as i64).into()))
    } else if let Some(s) = value.unpack_str() {
        Ok(JsonValue::String(s.to_string()))
    } else if let Ok(len) = value.length() {
        if len > 0 {
            // Try to determine if it's a dict by checking its type
            if value.get_type() == "dict" {
                // It's a dict-like object
                let mut obj = serde_json::Map::new();
                for item in value
                    .iterate(heap)
                    .map_err(|e| anyhow!("Failed to iterate dict: {}", e))?
                {
                    let (key, val) = extract_dict_item(item, heap)?;
                    obj.insert(key, starlark_to_json(val, heap)?);
                }
                Ok(JsonValue::Object(obj))
            } else {
                // It's a list
                let mut arr = Vec::new();
                for item in value
                    .iterate(heap)
                    .map_err(|e| anyhow!("Failed to iterate list: {}", e))?
                {
                    arr.push(starlark_to_json(item, heap)?);
                }
                Ok(JsonValue::Array(arr))
            }
        } else {
            // Empty collection, default to empty list
            Ok(JsonValue::Array(Vec::new()))
        }
    } else {
        // Default to string representation
        Ok(JsonValue::String(value.to_str()))
    }
}

fn extract_dict_item<'v>(item: Value<'v>, _heap: &'v Heap) -> Result<(String, Value<'v>)> {
    // For dict iteration, we get (key, value) tuples
    if let Some(tuple) = TupleRef::from_value(item) {
        if tuple.len() == 2 {
            return Ok((tuple.content()[0].to_str(), tuple.content()[1]));
        }
    }

    Err(anyhow!(
        "Expected (key, value) tuple from dict iteration, got: {}",
        item.get_type()
    ))
}
