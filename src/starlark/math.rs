use allocative::Allocative;
use derive_more::Display;
use either::Either;
use starlark::environment::{GlobalsBuilder, Methods, MethodsBuilder, MethodsStatic};
use starlark::starlark_module;
use starlark::starlark_simple_value;
use starlark::values::starlark_value;
use starlark::values::{NoSerialize, ProvidesStaticType, StarlarkValue, Value};

#[derive(Debug, Display, Allocative, ProvidesStaticType, NoSerialize)]
#[display(fmt = "math")]
pub struct MathModule;

starlark_simple_value!(MathModule);

#[starlark_value(type = "math")]
impl<'v> StarlarkValue<'v> for MathModule {
    fn get_methods() -> Option<&'static Methods> {
        static RES: MethodsStatic = MethodsStatic::new();
        RES.methods(math_methods)
    }

    fn dir_attr(&self) -> Vec<String> {
        vec![
            "pow".to_owned(),
            "sqrt".to_owned(),
            "ceil".to_owned(),
            "floor".to_owned(),
            "round".to_owned(),
            "abs".to_owned(),
            "pi".to_owned(),
            "e".to_owned(),
        ]
    }
}

#[starlark_module]
fn math_methods(builder: &mut MethodsBuilder) {
    /// Returns x raised to the power y (x^y).
    ///
    /// # Examples
    /// ```python
    /// math.pow(2, 3)    # 8.0
    /// math.pow(10, 2)   # 100.0
    /// math.pow(4, 0.5)  # 2.0 (square root)
    /// ```
    fn pow(
        #[allow(unused_variables)] this: Value,
        x: Either<i32, f64>,
        y: Either<i32, f64>,
    ) -> anyhow::Result<f64> {
        let x_float = match x {
            Either::Left(i) => i as f64,
            Either::Right(f) => f,
        };
        let y_float = match y {
            Either::Left(i) => i as f64,
            Either::Right(f) => f,
        };
        Ok(x_float.powf(y_float))
    }

    /// Returns the square root of x.
    ///
    /// # Examples
    /// ```python
    /// math.sqrt(4)    # 2.0
    /// math.sqrt(9)    # 3.0
    /// math.sqrt(2)    # 1.414...
    /// ```
    fn sqrt(#[allow(unused_variables)] this: Value, x: Either<i32, f64>) -> anyhow::Result<f64> {
        let x_float = match x {
            Either::Left(i) => i as f64,
            Either::Right(f) => f,
        };
        if x_float < 0.0 {
            return Err(anyhow::anyhow!(
                "math domain error: sqrt of negative number"
            ));
        }
        Ok(x_float.sqrt())
    }

    /// Returns the ceiling of x, the smallest integer greater than or equal to x.
    ///
    /// # Examples
    /// ```python
    /// math.ceil(4.2)   # 5
    /// math.ceil(-4.2)  # -4
    /// math.ceil(5)     # 5
    /// ```
    fn ceil(#[allow(unused_variables)] this: Value, x: Either<i32, f64>) -> anyhow::Result<i32> {
        let x_float = match x {
            Either::Left(i) => return Ok(i),
            Either::Right(f) => f,
        };
        Ok(x_float.ceil() as i32)
    }

    /// Returns the floor of x, the largest integer less than or equal to x.
    ///
    /// # Examples
    /// ```python
    /// math.floor(4.8)   # 4
    /// math.floor(-4.2)  # -5
    /// math.floor(5)     # 5
    /// ```
    fn floor(#[allow(unused_variables)] this: Value, x: Either<i32, f64>) -> anyhow::Result<i32> {
        let x_float = match x {
            Either::Left(i) => return Ok(i),
            Either::Right(f) => f,
        };
        Ok(x_float.floor() as i32)
    }

    /// Returns x rounded to the given number of decimal places.
    /// If decimals is not specified, rounds to the nearest integer.
    ///
    /// # Examples
    /// ```python
    /// math.round(3.14159, 2)  # 3.14
    /// math.round(3.5)         # 4.0
    /// math.round(2.718, 1)    # 2.7
    /// ```
    fn round(
        #[allow(unused_variables)] this: Value,
        x: Either<i32, f64>,
        #[starlark(default = 0)] decimals: i32,
    ) -> anyhow::Result<f64> {
        if decimals < 0 {
            return Err(anyhow::anyhow!("decimals must be non-negative"));
        }

        let x_float = match x {
            Either::Left(i) => i as f64,
            Either::Right(f) => f,
        };

        if decimals == 0 {
            Ok(x_float.round())
        } else {
            let multiplier = 10_f64.powi(decimals);
            Ok((x_float * multiplier).round() / multiplier)
        }
    }

    /// Returns the absolute value of x.
    ///
    /// # Examples
    /// ```python
    /// math.abs(-5)    # 5
    /// math.abs(3.14)  # 3.14
    /// math.abs(-2.7)  # 2.7
    /// ```
    fn abs(#[allow(unused_variables)] this: Value, x: Either<i32, f64>) -> anyhow::Result<f64> {
        let x_float = match x {
            Either::Left(i) => i as f64,
            Either::Right(f) => f,
        };
        Ok(x_float.abs())
    }

    /// The mathematical constant π (pi), approximately 3.14159.
    #[starlark(attribute)]
    fn pi(#[allow(unused_variables)] this: Value) -> anyhow::Result<f64> {
        Ok(std::f64::consts::PI)
    }

    /// The mathematical constant e (Euler's number), approximately 2.71828.
    #[starlark(attribute)]
    fn e(#[allow(unused_variables)] this: Value) -> anyhow::Result<f64> {
        Ok(std::f64::consts::E)
    }
}

pub fn register(builder: &mut GlobalsBuilder) {
    const MATH: MathModule = MathModule;
    builder.set("math", MATH);
}

#[cfg(test)]
mod tests {
    use super::*;
    use starlark::environment::GlobalsBuilder;
    use starlark::eval::Evaluator;
    use starlark::syntax::{AstModule, Dialect};

    fn eval_math(code: &str) -> Result<String, starlark::Error> {
        let globals = GlobalsBuilder::new().with(register).build();
        let module = starlark::environment::Module::new();
        let ast = AstModule::parse("test.star", code.to_owned(), &Dialect::Standard)?;
        let mut eval = Evaluator::new(&module);
        let result = eval.eval_module(ast, &globals)?;
        // Convert to string while the Module and heap are still in scope so we don't
        // return a Value<'static> that contains non-Send/Sync internals.
        Ok(result.to_string())
    }

    #[test]
    fn test_pow() {
        assert_eq!(eval_math("math.pow(2, 3)").unwrap(), "8.0");
        assert_eq!(eval_math("math.pow(10, 2)").unwrap(), "100.0");
        assert_eq!(eval_math("math.pow(4, 0.5)").unwrap(), "2.0");
    }

    #[test]
    fn test_sqrt() {
        assert_eq!(eval_math("math.sqrt(4)").unwrap(), "2.0");
        assert_eq!(eval_math("math.sqrt(9)").unwrap(), "3.0");
        assert!(eval_math("math.sqrt(-1)").is_err());
    }

    #[test]
    fn test_ceil() {
        assert_eq!(eval_math("math.ceil(4.2)").unwrap(), "5");
        assert_eq!(eval_math("math.ceil(-4.2)").unwrap(), "-4");
        assert_eq!(eval_math("math.ceil(5)").unwrap(), "5");
    }

    #[test]
    fn test_floor() {
        assert_eq!(eval_math("math.floor(4.8)").unwrap(), "4");
        assert_eq!(eval_math("math.floor(-4.2)").unwrap(), "-5");
        assert_eq!(eval_math("math.floor(5)").unwrap(), "5");
    }

    #[test]
    fn test_round() {
        let result = eval_math("math.round(3.14159, 2)").unwrap();
        assert!(result.starts_with("3.14"));

        assert_eq!(eval_math("math.round(3.5)").unwrap(), "4.0");

        let result = eval_math("math.round(2.718, 1)").unwrap();
        assert!(result.starts_with("2.7"));
    }

    #[test]
    fn test_abs() {
        assert_eq!(eval_math("math.abs(-5)").unwrap(), "5.0");
        let result = eval_math("math.abs(3.14)").unwrap();
        assert!(result.starts_with("3.14"));
    }

    #[test]
    fn test_constants() {
        let pi_result = eval_math("math.pi").unwrap();
        assert!(pi_result.starts_with("3.14"));

        let e_result = eval_math("math.e").unwrap();
        assert!(e_result.starts_with("2.71"));
    }
}
