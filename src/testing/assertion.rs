use anyhow::{anyhow, Result};
use starlark::environment::GlobalsBuilder;
use starlark::values::Value;

/// Assertion error for test failures
#[derive(Debug)]
pub struct AssertionError {
    pub message: String,
}

impl std::fmt::Display for AssertionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for AssertionError {}

/// Register assertion functions for Starlark tests
pub fn register_assertion_functions(builder: &mut GlobalsBuilder) {
    // assert_eq(actual, expected, message="")
    builder.set_function(
        "assert_eq",
        |actual: Value, expected: Value, message: Option<String>| -> Result<()> {
            if actual.equals(expected).map_err(|e| anyhow!("Error comparing values: {}", e))? {
                Ok(())
            } else {
                let msg = message.unwrap_or_else(|| {
                    format!(
                        "Assertion failed: expected {:?}, got {:?}",
                        expected, actual
                    )
                });
                Err(anyhow!(AssertionError { message: msg }))
            }
        },
    );

    // assert_ne(actual, expected, message="")
    builder.set_function(
        "assert_ne",
        |actual: Value, expected: Value, message: Option<String>| -> Result<()> {
            if !actual.equals(expected).map_err(|e| anyhow!("Error comparing values: {}", e))? {
                Ok(())
            } else {
                let msg = message.unwrap_or_else(|| {
                    format!("Assertion failed: expected values to be different, but both are {:?}", actual)
                });
                Err(anyhow!(AssertionError { message: msg }))
            }
        },
    );

    // assert_true(value, message="")
    builder.set_function(
        "assert_true",
        |value: Value, message: Option<String>| -> Result<()> {
            if value.to_bool() {
                Ok(())
            } else {
                let msg = message
                    .unwrap_or_else(|| format!("Assertion failed: expected truthy value, got {:?}", value));
                Err(anyhow!(AssertionError { message: msg }))
            }
        },
    );

    // assert_false(value, message="")
    builder.set_function(
        "assert_false",
        |value: Value, message: Option<String>| -> Result<()> {
            if !value.to_bool() {
                Ok(())
            } else {
                let msg = message
                    .unwrap_or_else(|| format!("Assertion failed: expected falsy value, got {:?}", value));
                Err(anyhow!(AssertionError { message: msg }))
            }
        },
    );

    // assert_in(item, container, message="")
    builder.set_function(
        "assert_in",
        |item: Value, container: Value, message: Option<String>| -> Result<()> {
            let contains = container
                .is_in(item)
                .map_err(|e| anyhow!("Error checking containment: {}", e))?;

            if contains.to_bool() {
                Ok(())
            } else {
                let msg = message.unwrap_or_else(|| {
                    format!("Assertion failed: {:?} not in {:?}", item, container)
                });
                Err(anyhow!(AssertionError { message: msg }))
            }
        },
    );

    // fail(message)
    builder.set_function("fail", |message: String| -> Result<()> {
        Err(anyhow!(AssertionError { message }))
    });
}
