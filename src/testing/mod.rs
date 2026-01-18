mod testing;

use anyhow::{Result, anyhow};
use starlark::environment::{FrozenModule, Globals, GlobalsBuilder, LibraryExtension, Module};
use starlark::eval::{Evaluator, FileLoader};
use starlark::syntax::{AstModule, Dialect};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{debug, error, info};

use crate::starlark::mcp_types::mcp_globals;
use crate::starlark::modules::build_globals;

fn build_test_globals() -> Globals {
    GlobalsBuilder::extended_by(&[
        LibraryExtension::StructType,
        LibraryExtension::Json,
        LibraryExtension::Debug,
    ])
    .with(mcp_globals)
    .with(testing::register)
    .with(crate::starlark::math::register)
    .with(crate::starlark::modules::time::register)
    .with(crate::starlark::modules::env::register)
    .with(crate::starlark::modules::exec::register)
    .with(crate::starlark::http::register)
    .with(crate::starlark::postgres::register)
    .with(crate::starlark::sqlite::register)
    .with(crate::starlark::data::register)
    .with(crate::starlark::fuzzy::register)
    .build()
}

struct ModuleLoader {
    modules: HashMap<String, Arc<FrozenModule>>,
}

impl FileLoader for ModuleLoader {
    fn load(&self, path: &str) -> anyhow::Result<FrozenModule> {
        self.modules
            .get(path)
            .map(|m| m.as_ref().clone())
            .ok_or_else(|| anyhow!("Module '{}' not found", path))
    }
}

#[derive(Debug)]
pub struct TestResult {
    pub name: String,
    pub passed: bool,
    pub error: Option<String>,
}

#[derive(Debug)]
pub struct TestSummary {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub results: Vec<TestResult>,
}

impl TestSummary {
    fn new() -> Self {
        Self {
            total: 0,
            passed: 0,
            failed: 0,
            results: Vec::new(),
        }
    }

    fn add_result(&mut self, result: TestResult) {
        self.total += 1;
        if result.passed {
            self.passed += 1;
        } else {
            self.failed += 1;
        }
        self.results.push(result);
    }

    fn print(&self) {
        println!("\n{}", "=".repeat(60));
        println!("Test Summary");
        println!("{}", "=".repeat(60));

        for result in &self.results {
            let status = if result.passed {
                "✓ PASS"
            } else {
                "✗ FAIL"
            };
            println!("{} {}", status, result.name);
            if let Some(error) = &result.error {
                println!("  Error: {}", error);
            }
        }

        println!("{}", "=".repeat(60));
        println!(
            "Total: {} | Passed: {} | Failed: {}",
            self.total, self.passed, self.failed
        );
        println!("{}", "=".repeat(60));
    }
}

/// Discover test files in the given directory
fn discover_test_files(extensions_dir: &str) -> Result<Vec<PathBuf>> {
    let path = Path::new(extensions_dir);
    if !path.exists() {
        return Ok(Vec::new());
    }

    let canonical_dir = std::fs::canonicalize(path)?;

    let test_files = std::fs::read_dir(&canonical_dir)?
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let entry_path = entry.path();
            let canonical_entry = std::fs::canonicalize(&entry_path).ok()?;

            if !canonical_entry.starts_with(&canonical_dir) {
                debug!("Skipping entry outside directory: {:?}", canonical_entry);
                return None;
            }

            let file_name = entry_path.file_name()?.to_str()?;
            if entry_path.is_file() && file_name.ends_with("_test.star") {
                Some(entry_path)
            } else {
                None
            }
        })
        .collect();

    Ok(test_files)
}

/// Load a test file and return the frozen module
fn load_test_file(
    test_path: &Path,
    available_modules: &HashMap<String, Arc<FrozenModule>>,
) -> Result<FrozenModule> {
    let content = std::fs::read_to_string(test_path)?;
    let file_name = test_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");

    let ast = AstModule::parse(file_name, content, &Dialect::Extended)
        .map_err(|e| anyhow!("Failed to parse {}: {}", file_name, e))?;

    let globals = build_test_globals();
    let module = Module::new();

    let loader = ModuleLoader {
        modules: available_modules.clone(),
    };

    // Evaluator must be dropped before freeze() to satisfy borrow checker
    {
        let mut eval = Evaluator::new(&module);
        eval.set_loader(&loader);
        eval.eval_module(ast, &globals)
            .map_err(|e| anyhow!("Failed to evaluate {}: {}", file_name, e))?;
    }

    module.freeze()
}

/// Discover test functions in a frozen module
fn discover_test_functions(module: &FrozenModule) -> Vec<String> {
    let mut test_functions = Vec::new();

    for name in module.names() {
        debug!("Found name in module: {:?}", name);
        // Starlark's module.names() may include quotes in debug repr, so trim them
        let clean_name = name.trim_matches('"');
        if clean_name.starts_with("test_") {
            test_functions.push(clean_name.to_string());
        }
    }

    test_functions
}

/// Execute a single test function
fn execute_test(
    module: &FrozenModule,
    test_name: &str,
    file_name: &str,
    extensions_dir: &str,
) -> TestResult {
    let full_name = format!("{}::{}", file_name, test_name);

    debug!("Running test: {}", full_name);

    let test_fn = match module.get(test_name) {
        Ok(frozen_ref) => frozen_ref,
        Err(_) => {
            return TestResult {
                name: full_name,
                passed: false,
                error: Some(format!("Test function '{}' not found", test_name)),
            };
        }
    };

    crate::starlark::modules::set_extensions_dir(extensions_dir.to_string());

    let exec_module = Module::new();
    let mut eval = Evaluator::new(&exec_module);

    let result = match eval.eval_function(test_fn.value(), &[], &[]) {
        Ok(_) => TestResult {
            name: full_name,
            passed: true,
            error: None,
        },
        Err(e) => TestResult {
            name: full_name,
            passed: false,
            error: Some(format!("{}", e)),
        },
    };

    crate::starlark::modules::clear_extensions_dir();

    result
}

/// Load all non-test extensions as modules that can be imported by tests
fn load_extension_modules(extensions_dir: &str) -> Result<HashMap<String, Arc<FrozenModule>>> {
    let path = Path::new(extensions_dir);
    if !path.exists() {
        return Ok(HashMap::new());
    }

    let canonical_dir = std::fs::canonicalize(path)?;
    let mut modules = HashMap::new();
    let globals = build_globals();

    crate::starlark::modules::set_extensions_dir(canonical_dir.to_string_lossy().to_string());

    let extension_files: Vec<_> = std::fs::read_dir(&canonical_dir)?
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let entry_path = entry.path();
            let canonical_entry = std::fs::canonicalize(&entry_path).ok()?;

            if !canonical_entry.starts_with(&canonical_dir) {
                debug!("Skipping entry outside directory: {:?}", canonical_entry);
                return None;
            }

            let file_name = entry_path.file_name()?.to_str()?.to_string();
            if entry_path.is_file()
                && file_name.ends_with(".star")
                && !file_name.ends_with("_test.star")
            {
                Some((entry_path, file_name))
            } else {
                None
            }
        })
        .collect();

    for (entry_path, file_name) in extension_files {
        let module_name = file_name.trim_end_matches(".star");

        if let Err(e) =
            load_single_module(&entry_path, &file_name, module_name, &globals, &mut modules)
        {
            error!("Failed to load module {}: {}", module_name, e);
        }
    }

    crate::starlark::modules::clear_extensions_dir();

    Ok(modules)
}

/// Load a single extension module
fn load_single_module(
    entry_path: &Path,
    file_name: &str,
    module_name: &str,
    globals: &Globals,
    modules: &mut HashMap<String, Arc<FrozenModule>>,
) -> Result<()> {
    let content = std::fs::read_to_string(entry_path)?;
    let ast = AstModule::parse(file_name, content, &Dialect::Extended)
        .map_err(|e| anyhow!("Parse error: {}", e))?;

    let module = Module::new();
    {
        let mut eval = Evaluator::new(&module);
        eval.eval_module(ast, globals)
            .map_err(|e| anyhow!("Eval error: {}", e))?;
    }

    let frozen = module
        .freeze()
        .map_err(|e| anyhow!("Freeze error: {}", e))?;
    info!("Loaded module: {}", module_name);
    modules.insert(module_name.to_string(), Arc::new(frozen));
    Ok(())
}

/// Run all tests in the given directory
pub async fn run_tests(extensions_dir: &str) -> Result<()> {
    println!("Discovering tests in: {}", extensions_dir);

    let extension_modules = load_extension_modules(extensions_dir)?;
    info!("Loaded {} extension modules", extension_modules.len());

    let test_files = discover_test_files(extensions_dir)?;

    if test_files.is_empty() {
        println!("No test files found (files ending with _test.star)");
        return Ok(());
    }

    println!("Found {} test file(s)", test_files.len());

    let mut summary = TestSummary::new();

    for test_path in test_files {
        let file_name = test_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        println!("\nRunning tests from: {}", file_name);

        let test_module = match load_test_file(&test_path, &extension_modules) {
            Ok(module) => module,
            Err(e) => {
                error!("Failed to load test file {}: {}", file_name, e);
                summary.add_result(TestResult {
                    name: format!("{} (load error)", file_name),
                    passed: false,
                    error: Some(format!("{}", e)),
                });
                continue;
            }
        };

        let test_functions = discover_test_functions(&test_module);

        if test_functions.is_empty() {
            println!("  No test functions found (functions starting with test_)");
            continue;
        }

        println!("  Found {} test(s)", test_functions.len());

        for test_name in test_functions {
            let result = execute_test(&test_module, &test_name, file_name, extensions_dir);
            let status = if result.passed { "✓" } else { "✗" };
            println!("    {} {}", status, test_name);
            if let Some(error) = &result.error {
                println!("      Error: {}", error);
            }
            summary.add_result(result);
        }
    }

    summary.print();

    if summary.failed > 0 {
        return Err(anyhow!(
            "Tests failed: {} of {} tests",
            summary.failed,
            summary.total
        ));
    }

    Ok(())
}
