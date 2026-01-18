use allocative::Allocative;
use derive_more::Display;
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use starlark::collections::SmallMap;
use starlark::environment::{GlobalsBuilder, Methods, MethodsBuilder, MethodsStatic};
use starlark::starlark_module;
use starlark::starlark_simple_value;
use starlark::values::dict::Dict;
use starlark::values::none::NoneType;
use starlark::values::starlark_value;
use starlark::values::{Heap, NoSerialize, ProvidesStaticType, StarlarkValue, Value};

#[derive(Debug, Display, Allocative, ProvidesStaticType, NoSerialize)]
#[display(fmt = "fuzzy")]
pub struct FuzzyModule;

starlark_simple_value!(FuzzyModule);

#[starlark_value(type = "fuzzy")]
impl<'v> StarlarkValue<'v> for FuzzyModule {
    fn get_methods() -> Option<&'static Methods> {
        static RES: MethodsStatic = MethodsStatic::new();
        RES.methods(fuzzy_methods)
    }

    fn dir_attr(&self) -> Vec<String> {
        vec!["search".to_owned(), "search_with_scores".to_owned()]
    }
}

/// Represents a scored match for sorting
struct ScoredItem<'v> {
    item: Value<'v>,
    score: i64,
}

/// Represents which keys to search in a dict
enum SearchKeys<'a> {
    /// Search all string fields
    All,
    /// Search a single key
    Single(&'a str),
    /// Search multiple keys
    Multiple(Vec<&'a str>),
}

/// Extract the search text from an item based on the keys parameter
fn get_search_text<'v>(item: Value<'v>, keys: &SearchKeys, heap: &'v Heap) -> Option<String> {
    if let Some(s) = item.unpack_str() {
        return Some(s.to_string());
    }

    if item.get_type() != "dict" {
        return None;
    }

    match keys {
        SearchKeys::Single(key) => {
            let key_value = heap.alloc_str(key).to_value();
            item.at(key_value, heap)
                .ok()
                .and_then(|v| v.unpack_str())
                .map(|s| s.to_string())
        }
        SearchKeys::Multiple(key_list) => {
            collect_string_values(item, key_list.iter().copied(), heap)
        }
        SearchKeys::All => {
            let dict_keys: Vec<_> = item
                .iterate(heap)
                .ok()?
                .filter_map(|k| k.unpack_str().map(|s| s.to_string()))
                .collect();
            let key_refs: Vec<&str> = dict_keys.iter().map(|s| s.as_str()).collect();
            collect_string_values(item, key_refs.into_iter(), heap)
        }
    }
}

/// Collect string values from dict fields and join them
fn collect_string_values<'v, 'a>(
    item: Value<'v>,
    keys: impl Iterator<Item = &'a str>,
    heap: &'v Heap,
) -> Option<String> {
    let text_parts: Vec<String> = keys
        .filter_map(|key| {
            let key_value = heap.alloc_str(key).to_value();
            item.at(key_value, heap)
                .ok()
                .and_then(|v| v.unpack_str())
                .map(|s| s.to_string())
        })
        .collect();

    if text_parts.is_empty() {
        None
    } else {
        Some(text_parts.join(" "))
    }
}

/// Perform fuzzy search and return scored results
fn fuzzy_search_internal<'v>(
    query: &str,
    items: Value<'v>,
    keys: &SearchKeys,
    limit: Option<i32>,
    heap: &'v Heap,
) -> anyhow::Result<Vec<ScoredItem<'v>>> {
    let matcher = SkimMatcherV2::default();
    let mut results: Vec<ScoredItem<'v>> = Vec::new();

    let iter = items
        .iterate(heap)
        .map_err(|e| anyhow::anyhow!("fuzzy.search: items must be iterable: {}", e))?;

    for item in iter {
        if let Some(text) = get_search_text(item, keys, heap)
            && let Some(score) = matcher.fuzzy_match(&text, query)
        {
            results.push(ScoredItem { item, score });
        }
    }

    results.sort_by(|a, b| b.score.cmp(&a.score));

    if let Some(limit) = limit
        && limit > 0
    {
        results.truncate(limit as usize);
    }

    Ok(results)
}

/// Helper to insert a key-value pair into a SmallMap with hashing
fn insert_hashed<'v>(
    map: &mut SmallMap<Value<'v>, Value<'v>>,
    heap: &'v Heap,
    key: &str,
    value: Value<'v>,
) {
    let key_value = heap.alloc_str(key).to_value();
    map.insert_hashed(key_value.get_hashed().expect("Failed to hash key"), value);
}

/// Parse the limit parameter into an Option<i32>
fn parse_limit(limit: Value, func_name: &str) -> anyhow::Result<Option<i32>> {
    if limit.is_none() {
        return Ok(None);
    }
    limit
        .unpack_i32()
        .map(Some)
        .ok_or_else(|| anyhow::anyhow!("{}: limit must be an integer", func_name))
}

/// Parse the key/keys parameters into a SearchKeys enum
fn parse_search_keys<'a, 'v>(
    key: Value<'v>,
    keys: Value<'v>,
    key_storage: &'a mut Vec<String>,
    func_name: &str,
    heap: &'v Heap,
) -> anyhow::Result<SearchKeys<'a>> {
    if !key.is_none() && !keys.is_none() {
        return Err(anyhow::anyhow!(
            "{}: cannot specify both 'key' and 'keys' parameters",
            func_name
        ));
    }

    if !keys.is_none() {
        let iter = keys
            .iterate(heap)
            .map_err(|_| anyhow::anyhow!("{}: keys must be a list of strings", func_name))?;

        for item in iter {
            if let Some(s) = item.unpack_str() {
                key_storage.push(s.to_string());
            } else {
                return Err(anyhow::anyhow!(
                    "{}: keys must be a list of strings",
                    func_name
                ));
            }
        }

        if key_storage.is_empty() {
            Ok(SearchKeys::All)
        } else {
            let refs: Vec<&str> = key_storage.iter().map(|s| s.as_str()).collect();
            Ok(SearchKeys::Multiple(refs))
        }
    } else if !key.is_none() {
        let key_str = key
            .unpack_str()
            .ok_or_else(|| anyhow::anyhow!("{}: key must be a string", func_name))?;
        key_storage.push(key_str.to_string());
        Ok(SearchKeys::Single(key_storage.last().unwrap().as_str()))
    } else {
        Ok(SearchKeys::All)
    }
}

#[starlark_module]
fn fuzzy_methods(builder: &mut MethodsBuilder) {
    /// Perform fuzzy search on a list of items and return matching items.
    ///
    /// # Arguments
    /// * `query` - The search query string
    /// * `items` - A list of strings or dicts to search through
    /// * `key` - Optional single key to search within dicts
    /// * `keys` - Optional list of keys to search within dicts
    /// * `limit` - Optional maximum number of results to return
    ///
    /// Note: `key` and `keys` are mutually exclusive. If neither is provided, searches all string fields.
    ///
    /// # Returns
    /// A list of matching items, sorted by relevance (best matches first)
    ///
    /// # Examples
    /// ```python
    /// # Search a list of strings
    /// results = fuzzy.search("helo", ["hello", "world", "help"])
    ///
    /// # Search dicts by specific key
    /// items = [{"name": "Potion", "type": "Medicine"}, {"name": "Antidote", "type": "Medicine"}]
    /// results = fuzzy.search("potn", items, key="name")
    ///
    /// # Search dicts by multiple keys
    /// results = fuzzy.search("medicine potion", items, keys=["name", "type"])
    ///
    /// # Search dicts by all string fields
    /// results = fuzzy.search("medicine", items)
    /// ```
    fn search<'v>(
        #[allow(unused_variables)] this: Value<'v>,
        query: &str,
        items: Value<'v>,
        #[starlark(default = NoneType)] key: Value<'v>,
        #[starlark(default = NoneType)] keys: Value<'v>,
        #[starlark(default = NoneType)] limit: Value<'v>,
        heap: &'v Heap,
    ) -> anyhow::Result<Value<'v>> {
        let mut key_storage = Vec::new();
        let search_keys = parse_search_keys(key, keys, &mut key_storage, "fuzzy.search", heap)?;
        let limit_int = parse_limit(limit, "fuzzy.search")?;

        let results = fuzzy_search_internal(query, items, &search_keys, limit_int, heap)?;
        let items: Vec<Value<'v>> = results.into_iter().map(|r| r.item).collect();
        Ok(heap.alloc(items))
    }

    /// Perform fuzzy search and return matching items with their scores.
    ///
    /// # Arguments
    /// * `query` - The search query string
    /// * `items` - A list of strings or dicts to search through
    /// * `key` - Optional single key to search within dicts
    /// * `keys` - Optional list of keys to search within dicts
    /// * `limit` - Optional maximum number of results to return
    ///
    /// Note: `key` and `keys` are mutually exclusive. If neither is provided, searches all string fields.
    ///
    /// # Returns
    /// A list of dicts with "item" and "score" keys, sorted by score (best matches first)
    ///
    /// # Examples
    /// ```python
    /// results = fuzzy.search_with_scores("potn", items, key="name", limit=10)
    /// # Returns: [{"item": {...}, "score": 85}, {"item": {...}, "score": 42}]
    ///
    /// # Search multiple keys
    /// results = fuzzy.search_with_scores("healing", items, keys=["name", "desc"])
    /// ```
    fn search_with_scores<'v>(
        #[allow(unused_variables)] this: Value<'v>,
        query: &str,
        items: Value<'v>,
        #[starlark(default = NoneType)] key: Value<'v>,
        #[starlark(default = NoneType)] keys: Value<'v>,
        #[starlark(default = NoneType)] limit: Value<'v>,
        heap: &'v Heap,
    ) -> anyhow::Result<Value<'v>> {
        let mut key_storage = Vec::new();
        let search_keys = parse_search_keys(
            key,
            keys,
            &mut key_storage,
            "fuzzy.search_with_scores",
            heap,
        )?;
        let limit_int = parse_limit(limit, "fuzzy.search_with_scores")?;

        let results = fuzzy_search_internal(query, items, &search_keys, limit_int, heap)?;

        let scored_items: Vec<Value<'v>> = results
            .into_iter()
            .map(|r| {
                let mut map = SmallMap::new();
                insert_hashed(&mut map, heap, "item", r.item);
                insert_hashed(&mut map, heap, "score", heap.alloc(r.score));
                heap.alloc(Dict::new(map))
            })
            .collect();

        Ok(heap.alloc(scored_items))
    }
}

pub fn register(builder: &mut GlobalsBuilder) {
    const FUZZY: FuzzyModule = FuzzyModule;
    builder.set("fuzzy", FUZZY);
}

#[cfg(test)]
mod tests {
    use super::*;
    use starlark::environment::GlobalsBuilder;
    use starlark::eval::Evaluator;
    use starlark::syntax::{AstModule, Dialect};

    fn eval_fuzzy(code: &str) -> Result<String, starlark::Error> {
        let globals = GlobalsBuilder::new().with(register).build();
        let module = starlark::environment::Module::new();
        let ast = AstModule::parse("test.star", code.to_owned(), &Dialect::Standard)?;
        let mut eval = Evaluator::new(&module);
        let result = eval.eval_module(ast, &globals)?;
        Ok(result.to_string())
    }

    #[test]
    fn test_search_strings() {
        let result =
            eval_fuzzy(r#"fuzzy.search("hello", ["hello", "world", "help", "helicopter"])"#)
                .unwrap();
        // Should match "hello" and possibly others, but not "world"
        assert!(result.contains("hello"));
        assert!(!result.contains("world"));
    }

    #[test]
    fn test_search_with_limit() {
        let result =
            eval_fuzzy(r#"fuzzy.search("hel", ["hello", "help", "helicopter"], limit=2)"#).unwrap();
        // Should only return 2 results
        let count = result.matches("hel").count();
        assert!(count <= 2);
    }

    #[test]
    fn test_search_dicts_with_key() {
        let result = eval_fuzzy(
            r#"fuzzy.search("potn", [{"name": "Potion", "type": "Medicine"}, {"name": "Antidote", "type": "Medicine"}], key="name")"#,
        )
        .unwrap();
        assert!(result.contains("Potion"));
    }

    #[test]
    fn test_search_dicts_all_fields() {
        let result = eval_fuzzy(
            r#"fuzzy.search("medicine", [{"name": "Potion", "type": "Medicine"}, {"name": "Pokeball", "type": "Ball"}])"#,
        )
        .unwrap();
        assert!(result.contains("Potion"));
        assert!(!result.contains("Pokeball"));
    }

    #[test]
    fn test_search_with_scores() {
        let result =
            eval_fuzzy(r#"fuzzy.search_with_scores("hello", ["hello", "helo", "world"])"#).unwrap();
        assert!(result.contains("item"));
        assert!(result.contains("score"));
    }

    #[test]
    fn test_empty_results() {
        let result = eval_fuzzy(r#"fuzzy.search("xyz", ["hello", "world"])"#).unwrap();
        assert_eq!(result, "[]");
    }

    #[test]
    fn test_search_dicts_with_keys() {
        // Test searching multiple keys - "heals" matches the desc field "Heals HP"
        let result = eval_fuzzy(
            r#"fuzzy.search("heals", [{"name": "Potion", "desc": "Heals HP", "type": "Medicine"}, {"name": "Antidote", "desc": "Cures poison", "type": "Medicine"}], keys=["name", "desc"])"#,
        )
        .unwrap();
        assert!(result.contains("Potion"));
        assert!(!result.contains("Antidote"));
    }

    #[test]
    fn test_search_with_scores_and_keys() {
        let result = eval_fuzzy(
            r#"fuzzy.search_with_scores("heal", [{"name": "Potion", "desc": "Heals HP"}, {"name": "Antidote", "desc": "Cures poison"}], keys=["name", "desc"])"#,
        )
        .unwrap();
        assert!(result.contains("item"));
        assert!(result.contains("score"));
        assert!(result.contains("Potion"));
    }

    #[test]
    fn test_key_and_keys_mutually_exclusive() {
        let result =
            eval_fuzzy(r#"fuzzy.search("test", [{"name": "Test"}], key="name", keys=["name"])"#);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("cannot specify both"));
    }

    #[test]
    fn test_dir_attr() {
        let module = FuzzyModule;
        let attrs = module.dir_attr();
        assert!(attrs.contains(&"search".to_owned()));
        assert!(attrs.contains(&"search_with_scores".to_owned()));
    }
}
