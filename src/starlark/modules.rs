use allocative::Allocative;
use derive_more::Display;
use starlark::collections::SmallMap;
use starlark::environment::{
    Globals, GlobalsBuilder, LibraryExtension, Methods, MethodsBuilder, MethodsStatic,
};
use starlark::starlark_module;
use starlark::starlark_simple_value;
use starlark::values::starlark_value;
use starlark::values::{
    Heap, NoSerialize, ProvidesStaticType, StarlarkValue, Value, dict::Dict, none::NoneType,
};
use std::process::Command;

use super::data;
use super::fuzzy;
use super::http;
use super::math;
use super::mcp_types::mcp_globals;
use super::postgres;
use super::sqlite;

pub use exec::{clear_exec_whitelist, set_exec_whitelist};

pub use data::{clear_extensions_dir, set_extensions_dir};

pub fn build_globals() -> Globals {
    GlobalsBuilder::extended_by(&[
        LibraryExtension::StructType,
        LibraryExtension::Json,
        LibraryExtension::Debug,
    ])
    .with(mcp_globals)
    .with(math::register)
    .with(time::register)
    .with(env::register)
    .with(exec::register)
    .with(http::register)
    .with(postgres::register)
    .with(sqlite::register)
    .with(data::register)
    .with(fuzzy::register)
    .build()
}

pub(crate) mod time {
    use super::*;

    #[derive(Debug, Display, Allocative, ProvidesStaticType, NoSerialize)]
    #[display(fmt = "time")]
    pub struct TimeModule;

    starlark_simple_value!(TimeModule);

    #[starlark_value(type = "time")]
    impl<'v> StarlarkValue<'v> for TimeModule {
        fn get_methods() -> Option<&'static Methods> {
            static RES: MethodsStatic = MethodsStatic::new();
            RES.methods(time_methods)
        }
    }

    #[starlark_module]
    fn time_methods(builder: &mut MethodsBuilder) {
        fn now(#[allow(unused_variables)] this: Value) -> anyhow::Result<i64> {
            use std::time::{SystemTime, UNIX_EPOCH};
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|e| anyhow::anyhow!("Time error: {}", e))?;
            Ok(now.as_secs() as i64)
        }
    }

    pub fn register(builder: &mut GlobalsBuilder) {
        const TIME: TimeModule = TimeModule;
        builder.set("time", TIME);
    }
}

pub(crate) mod env {
    use super::*;

    #[derive(Debug, Display, Allocative, ProvidesStaticType, NoSerialize)]
    #[display(fmt = "env")]
    pub struct EnvModule;

    starlark_simple_value!(EnvModule);

    #[starlark_value(type = "env")]
    impl<'v> StarlarkValue<'v> for EnvModule {
        fn get_methods() -> Option<&'static Methods> {
            static RES: MethodsStatic = MethodsStatic::new();
            RES.methods(env_methods)
        }
    }

    #[starlark_module]
    fn env_methods(builder: &mut MethodsBuilder) {
        fn get(
            #[allow(unused_variables)] this: Value,
            name: &str,
            #[starlark(default = "")] default: &str,
        ) -> anyhow::Result<String> {
            Ok(std::env::var(name).unwrap_or_else(|_| default.to_string()))
        }
    }

    pub fn register(builder: &mut GlobalsBuilder) {
        const ENV: EnvModule = EnvModule;
        builder.set("env", ENV);
    }
}

pub(crate) mod exec {
    use super::*;
    use std::cell::RefCell;

    thread_local! {
        /// Thread-local storage for the exec whitelist
        /// Set by the tool executor before calling tool handler functions
        static EXEC_WHITELIST: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    }

    /// Set the exec whitelist for the current thread
    pub fn set_exec_whitelist(whitelist: Vec<String>) {
        EXEC_WHITELIST.with(|w| {
            *w.borrow_mut() = whitelist;
        });
    }

    /// Clear the exec whitelist for the current thread
    pub fn clear_exec_whitelist() {
        EXEC_WHITELIST.with(|w| {
            w.borrow_mut().clear();
        });
    }

    /// Get a copy of the current exec whitelist
    fn get_exec_whitelist() -> Vec<String> {
        EXEC_WHITELIST.with(|w| w.borrow().clone())
    }

    #[derive(Debug, Display, Allocative, ProvidesStaticType, NoSerialize)]
    #[display(fmt = "exec")]
    pub struct ExecModule;

    starlark_simple_value!(ExecModule);

    #[starlark_value(type = "exec")]
    impl<'v> StarlarkValue<'v> for ExecModule {
        fn get_methods() -> Option<&'static Methods> {
            static RES: MethodsStatic = MethodsStatic::new();
            RES.methods(exec_methods)
        }
    }

    #[starlark_module]
    fn exec_methods(builder: &mut MethodsBuilder) {
        /// Execute a command and return the result
        /// Returns a dict with keys: stdout, stderr, exit_code, success
        fn run<'v>(
            #[allow(unused_variables)] this: Value<'v>,
            command: String,
            #[starlark(default = NoneType)] args: Value<'v>,
            heap: &'v Heap,
        ) -> anyhow::Result<Value<'v>> {
            let arg_vec = if args.is_none() {
                Vec::new()
            } else {
                let args_list = args
                    .iterate(heap)
                    .map_err(|e| anyhow::anyhow!("Failed to iterate args: {}", e))?;
                let mut vec = Vec::new();
                for arg in args_list {
                    vec.push(arg.to_str());
                }
                vec
            };

            let whitelist = get_exec_whitelist();
            if whitelist.is_empty() {
                return Err(anyhow::anyhow!(
                    "Command '{}' cannot be executed: no exec whitelist configured for this extension. Add allowed_exec=['{}'] to the Extension definition.",
                    command,
                    command
                ));
            }

            if !whitelist.contains(&command) {
                return Err(anyhow::anyhow!(
                    "Command '{}' is not in the allowed exec whitelist. Allowed commands: {:?}",
                    command,
                    whitelist
                ));
            }

            let output = Command::new(&command)
                .args(&arg_vec)
                .output()
                .map_err(|e| anyhow::anyhow!("Failed to execute command '{}': {}", command, e))?;

            let mut map = SmallMap::new();
            let insert = |map: &mut SmallMap<Value<'v>, Value<'v>>, key: &str, value: Value<'v>| {
                let key_value = heap.alloc_str(key).to_value();
                map.insert_hashed(key_value.get_hashed().expect("Failed to hash key"), value);
            };

            insert(
                &mut map,
                "stdout",
                heap.alloc(String::from_utf8_lossy(&output.stdout).to_string()),
            );
            insert(
                &mut map,
                "stderr",
                heap.alloc(String::from_utf8_lossy(&output.stderr).to_string()),
            );
            insert(
                &mut map,
                "exit_code",
                heap.alloc(output.status.code().unwrap_or(-1)),
            );
            insert(&mut map, "success", heap.alloc(output.status.success()));

            Ok(heap.alloc(Dict::new(map)))
        }
    }

    pub fn register(builder: &mut GlobalsBuilder) {
        const EXEC: ExecModule = ExecModule;
        builder.set("exec", EXEC);
    }
}
