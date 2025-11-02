mod assertion;

use anyhow::{anyhow, Result};
use starlark::environment::{GlobalsBuilder, Module};
use starlark::eval::Evaluator;
use starlark::syntax::{AstModule, Dialect};
use starlark::values::{FrozenModule, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{debug, error, info};

use crate::starlark::modules::register_modules;

/// Result of a single test execution
#[derive(Debug)]
pub struct TestResult {
    pub name: String,
    pub passed: bool,
    pub error: Option<String>,
}

/// Summary of all test results
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
            let status = if result.passed { "✓ PASS" } else { "✗ FAIL" };
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

    let mut test_files = Vec::new();

    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                if file_name.ends_with("_test.star") {
                    test_files.push(path);
                }
            }
        }
    }

    Ok(test_files)
}

/// Build globals with assertion functions and standard modules
fn build_test_globals() -> GlobalsBuilder {
    let mut builder = GlobalsBuilder::new();

    // Register assertion functions
    assertion::register_assertion_functions(&mut builder);

    // Register standard modules (json, struct, debug, etc.)
    register_modules(&mut builder);

    builder
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

    // Parse the Starlark code
    let ast = AstModule::parse(file_name, content, &Dialect::Extended)
        .map_err(|e| anyhow!("Failed to parse {}: {}", file_name, e))?;

    // Create module and evaluator
    let globals = build_test_globals().build();
    let module = Module::new();

    let mut eval = Evaluator::new(&module);

    // Make available modules loadable via load() statement
    let modules_for_loader = available_modules.clone();
    eval.set_loader(move |name: &str| {
        if let Some(frozen_module) = modules_for_loader.get(name) {
            Ok(frozen_module.as_ref().clone())
        } else {
            Err(anyhow!("Module '{}' not found", name).into())
        }
    });

    // Evaluate the test file
    eval.eval_module(ast, &globals)
        .map_err(|e| anyhow!("Failed to evaluate {}: {}", file_name, e))?;

    // Freeze the module
    Ok(module.freeze()?)
}

/// Discover test functions in a frozen module
fn discover_test_functions(module: &FrozenModule) -> Vec<String> {
    let mut test_functions = Vec::new();

    for name in module.names() {
        if name.starts_with("test_") {
            test_functions.push(name.to_string());
        }
    }

    test_functions
}

/// Execute a single test function
fn execute_test(module: &FrozenModule, test_name: &str, file_name: &str) -> TestResult {
    let full_name = format!("{}::{}", file_name, test_name);

    debug!("Running test: {}", full_name);

    // Get the test function
    let test_fn = match module.get(test_name) {
        Ok(frozen_ref) => frozen_ref,
        Err(_) => {
            return TestResult {
                name: full_name,
                passed: false,
                error: Some(format!("Test function '{}' not found", test_name)),
            }
        }
    };

    // Create a new module for test execution
    let globals = build_test_globals().build();
    let exec_module = Module::new();
    let mut eval = Evaluator::new(&exec_module);

    // Try to call the test function
    match eval.eval_function(test_fn.value(), &[], &[]) {
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
    }
}

/// Load all non-test extensions as modules that can be imported by tests
async fn load_extension_modules(extensions_dir: &str) -> Result<HashMap<String, Arc<FrozenModule>>> {
    let path = Path::new(extensions_dir);
    if !path.exists() {
        return Ok(HashMap::new());
    }

    let mut modules = HashMap::new();
    let globals = build_test_globals().build();

    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                // Load non-test .star files as modules
                if file_name.ends_with(".star") && !file_name.ends_with("_test.star") {
                    let module_name = file_name.trim_end_matches(".star");
                    let content = std::fs::read_to_string(&path)?;

                    match AstModule::parse(file_name, content, &Dialect::Extended) {
                        Ok(ast) => {
                            let module = Module::new();
                            let mut eval = Evaluator::new(&module);

                            // Evaluate the module
                            if let Err(e) = eval.eval_module(ast, &globals) {
                                error!("Failed to load module {}: {}", module_name, e);
                                continue;
                            }

                            // Freeze and store the module
                            match module.freeze() {
                                Ok(frozen) => {
                                    info!("Loaded module: {}", module_name);
                                    modules.insert(module_name.to_string(), Arc::new(frozen));
                                }
                                Err(e) => {
                                    error!("Failed to freeze module {}: {}", module_name, e);
                                }
                            }
                        }
                        Err(e) => {
                            error!("Failed to parse module {}: {}", module_name, e);
                        }
                    }
                }
            }
        }
    }

    Ok(modules)
}

/// Run all tests in the given directory
pub async fn run_tests(extensions_dir: &str) -> Result<()> {
    println!("Discovering tests in: {}", extensions_dir);

    // Load extension modules first so they can be imported by tests
    let extension_modules = load_extension_modules(extensions_dir).await?;
    info!("Loaded {} extension modules", extension_modules.len());

    // Discover test files
    let test_files = discover_test_files(extensions_dir)?;

    if test_files.is_empty() {
        println!("No test files found (files ending with _test.star)");
        return Ok(());
    }

    println!("Found {} test file(s)", test_files.len());

    let mut summary = TestSummary::new();

    // Run tests from each file
    for test_path in test_files {
        let file_name = test_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        println!("\nRunning tests from: {}", file_name);

        // Load the test file
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

        // Discover test functions
        let test_functions = discover_test_functions(&test_module);

        if test_functions.is_empty() {
            println!("  No test functions found (functions starting with test_)");
            continue;
        }

        println!("  Found {} test(s)", test_functions.len());

        // Execute each test function
        for test_name in test_functions {
            let result = execute_test(&test_module, &test_name, file_name);
            let status = if result.passed { "✓" } else { "✗" };
            println!("    {} {}", status, test_name);
            if let Some(error) = &result.error {
                println!("      Error: {}", error);
            }
            summary.add_result(result);
        }
    }

    // Print summary
    summary.print();

    // Exit with error code if any tests failed
    if summary.failed > 0 {
        std::process::exit(1);
    }

    Ok(())
}
