use allocative::Allocative;
use derive_more::Display;
use starlark::environment::{GlobalsBuilder, Methods, MethodsBuilder, MethodsStatic};
use starlark::starlark_module;
use starlark::starlark_simple_value;
use starlark::values::starlark_value;
use starlark::values::{NoSerialize, ProvidesStaticType, StarlarkValue, Value};

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

#[derive(Debug, Display, Allocative, ProvidesStaticType, NoSerialize)]
#[display(fmt = "testing")]
pub struct TestingModule;

starlark_simple_value!(TestingModule);

#[starlark_value(type = "testing")]
impl<'v> StarlarkValue<'v> for TestingModule {
    fn get_methods() -> Option<&'static Methods> {
        static RES: MethodsStatic = MethodsStatic::new();
        RES.methods(testing_methods)
    }

    fn dir_attr(&self) -> Vec<String> {
        vec![
            "eq".to_owned(),
            "ne".to_owned(),
            "is_true".to_owned(),
            "is_false".to_owned(),
            "contains".to_owned(),
            "fail".to_owned(),
        ]
    }
}

#[starlark_module]
fn testing_methods(builder: &mut MethodsBuilder) {
    /// Assert that two values are equal.
    ///
    /// # Examples
    /// ```python
    /// testing.eq(2, 1 + 1)
    /// testing.eq("hello", "hello")
    /// testing.eq([1, 2], [1, 2], "Lists should be equal")
    /// ```
    fn eq<'v>(
        #[allow(unused_variables)] this: Value<'v>,
        expected: Value<'v>,
        actual: Value<'v>,
        #[starlark(default = "")] message: &str,
    ) -> anyhow::Result<starlark::values::none::NoneType> {
        if actual
            .equals(expected)
            .map_err(|e| anyhow::anyhow!("Error comparing values: {}", e))?
        {
            Ok(starlark::values::none::NoneType)
        } else {
            let msg = if message.is_empty() {
                format!(
                    "Assertion failed: expected {:?}, got {:?}",
                    expected, actual
                )
            } else {
                message.to_string()
            };
            Err(anyhow::anyhow!(AssertionError { message: msg }))
        }
    }

    /// Assert that two values are not equal.
    ///
    /// # Examples
    /// ```python
    /// testing.ne(2, 1)
    /// testing.ne("world", "hello")
    /// testing.ne([3, 4], [1, 2], "Lists should be different")
    /// ```
    fn ne<'v>(
        #[allow(unused_variables)] this: Value<'v>,
        expected: Value<'v>,
        actual: Value<'v>,
        #[starlark(default = "")] message: &str,
    ) -> anyhow::Result<starlark::values::none::NoneType> {
        if !actual
            .equals(expected)
            .map_err(|e| anyhow::anyhow!("Error comparing values: {}", e))?
        {
            Ok(starlark::values::none::NoneType)
        } else {
            let msg = if message.is_empty() {
                format!(
                    "Assertion failed: expected values to be different, but both are {:?}",
                    actual
                )
            } else {
                message.to_string()
            };
            Err(anyhow::anyhow!(AssertionError { message: msg }))
        }
    }

    /// Assert that a value is truthy.
    ///
    /// # Examples
    /// ```python
    /// testing.is_true(True)
    /// testing.is_true(1)
    /// testing.is_true("non-empty")
    /// testing.is_true([1, 2, 3], "List should be truthy")
    /// ```
    fn is_true<'v>(
        #[allow(unused_variables)] this: Value<'v>,
        value: Value<'v>,
        #[starlark(default = "")] message: &str,
    ) -> anyhow::Result<starlark::values::none::NoneType> {
        if value.to_bool() {
            Ok(starlark::values::none::NoneType)
        } else {
            let msg = if message.is_empty() {
                format!("Assertion failed: expected truthy value, got {:?}", value)
            } else {
                message.to_string()
            };
            Err(anyhow::anyhow!(AssertionError { message: msg }))
        }
    }

    /// Assert that a value is falsy.
    ///
    /// # Examples
    /// ```python
    /// testing.is_false(False)
    /// testing.is_false(0)
    /// testing.is_false("")
    /// testing.is_false([], "List should be empty")
    /// ```
    fn is_false<'v>(
        #[allow(unused_variables)] this: Value<'v>,
        value: Value<'v>,
        #[starlark(default = "")] message: &str,
    ) -> anyhow::Result<starlark::values::none::NoneType> {
        if !value.to_bool() {
            Ok(starlark::values::none::NoneType)
        } else {
            let msg = if message.is_empty() {
                format!("Assertion failed: expected falsy value, got {:?}", value)
            } else {
                message.to_string()
            };
            Err(anyhow::anyhow!(AssertionError { message: msg }))
        }
    }

    /// Assert that a container contains an item.
    ///
    /// # Examples
    /// ```python
    /// testing.contains([1, 2, 3], 2)
    /// testing.contains("hello", "ell")
    /// testing.contains({"a": 1, "b": 2}, "a", "Key should exist")
    /// ```
    fn contains<'v>(
        #[allow(unused_variables)] this: Value<'v>,
        container: Value<'v>,
        item: Value<'v>,
        #[starlark(default = "")] message: &str,
    ) -> anyhow::Result<starlark::values::none::NoneType> {
        let contains = container
            .is_in(item)
            .map_err(|e| anyhow::anyhow!("Error checking containment: {}", e))?;

        if contains {
            Ok(starlark::values::none::NoneType)
        } else {
            let msg = if message.is_empty() {
                format!("Assertion failed: {:?} not in {:?}", item, container)
            } else {
                message.to_string()
            };
            Err(anyhow::anyhow!(AssertionError { message: msg }))
        }
    }

    /// Fail unconditionally with a message.
    ///
    /// # Examples
    /// ```python
    /// testing.fail("This should not happen")
    /// ```
    fn fail<'v>(
        #[allow(unused_variables)] this: Value<'v>,
        message: &str,
    ) -> anyhow::Result<starlark::values::none::NoneType> {
        Err(anyhow::anyhow!(AssertionError {
            message: message.to_string()
        }))
    }
}

pub fn register(builder: &mut GlobalsBuilder) {
    const TESTING: TestingModule = TestingModule;
    builder.set("testing", TESTING);
}
