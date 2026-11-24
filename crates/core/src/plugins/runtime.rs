use wasmi::{
    Caller, Config, Engine, Extern, Linker, Module, Store, StoreLimits, StoreLimitsBuilder,
};

use crate::error::{AppError, AppResult};

const FUEL_LIMIT: u64 = 10_000_000;
const MEMORY_LIMIT: usize = 64 * 1024 * 1024;
const MAX_INPUT_BYTES: usize = 512 * 1024;
const MAX_OUTPUT_BYTES: usize = 256 * 1024;
const MAX_LOG_BYTES: usize = 64 * 1024;
const MAX_SINGLE_LOG_BYTES: usize = 8 * 1024;

pub struct RuntimeOutput {
    pub json: String,
    pub logs: Vec<String>,
    pub fuel_used: u64,
}

struct HostState {
    limits: StoreLimits,
    logs: Vec<String>,
    log_bytes: usize,
}

pub fn validate_wasm(bytes: &[u8]) -> AppResult<()> {
    let engine = Engine::default();
    Module::new(&engine, bytes)
        .map(|_| ())
        .map_err(|error| AppError::Plugin(format!("Invalid WebAssembly module: {error}")))
}

pub fn run_wasm(wasm: &[u8], input: &str) -> AppResult<RuntimeOutput> {
    if input.len() > MAX_INPUT_BYTES {
        return Err(AppError::Plugin("Plugin input is too large.".into()));
    }
    let mut config = Config::default();
    config.consume_fuel(true);
    let engine = Engine::new(&config);
    let limits = StoreLimitsBuilder::new()
        .memory_size(MEMORY_LIMIT)
        .table_elements(1024)
        .memories(1)
        .tables(1)
        .instances(1)
        .build();
    let mut store = Store::new(
        &engine,
        HostState {
            limits,
            logs: Vec::new(),
            log_bytes: 0,
        },
    );
    store.limiter(|state| &mut state.limits);
    store
        .set_fuel(FUEL_LIMIT)
        .map_err(|error| AppError::Plugin(format!("Unable to meter plugin: {error}")))?;

    let mut linker = Linker::new(&engine);
    linker
        .func_wrap(
            "env",
            "host_log",
            |mut caller: Caller<HostState>, ptr: i32, len: i32| {
                if ptr < 0 || len < 0 {
                    return;
                }
                let len = len as usize;
                if len > MAX_SINGLE_LOG_BYTES || caller.data().log_bytes + len > MAX_LOG_BYTES {
                    return;
                }
                let Some(memory) = caller.get_export("memory").and_then(Extern::into_memory) else {
                    return;
                };
                let mut buffer = vec![0u8; len];
                if memory.read(&caller, ptr as usize, &mut buffer).is_err() {
                    return;
                }
                if let Ok(text) = String::from_utf8(buffer) {
                    let state = caller.data_mut();
                    state.log_bytes += text.len();
                    state.logs.push(text);
                }
            },
        )
        .map_err(|error| AppError::Plugin(format!("Unable to link plugin host: {error}")))?;

    let module = Module::new(&engine, wasm)
        .map_err(|error| AppError::Plugin(format!("Invalid WebAssembly module: {error}")))?;
    let instance = linker
        .instantiate_and_start(&mut store, &module)
        .map_err(|error| AppError::Plugin(format!("Plugin failed to start: {error}")))?;
    let memory = instance
        .get_memory(&store, "memory")
        .ok_or_else(|| AppError::Plugin("Plugin does not export memory.".into()))?;
    let alloc = instance
        .get_typed_func::<i32, i32>(&store, "alloc")
        .map_err(|_| AppError::Plugin("Plugin does not export alloc.".into()))?;
    let run = instance
        .get_typed_func::<(i32, i32), i64>(&store, "run")
        .map_err(|_| AppError::Plugin("Plugin does not export run.".into()))?;

    let input_ptr = alloc
        .call(&mut store, input.len() as i32)
        .map_err(|error| AppError::Plugin(format!("Plugin allocation failed: {error}")))?;
    if input_ptr < 0 {
        return Err(AppError::Plugin(
            "Plugin returned an invalid input pointer.".into(),
        ));
    }
    memory
        .write(&mut store, input_ptr as usize, input.as_bytes())
        .map_err(|_| AppError::Plugin("Plugin input write failed.".into()))?;
    let packed = run
        .call(&mut store, (input_ptr, input.len() as i32))
        .map_err(|error| AppError::Plugin(format!("Plugin execution failed: {error}")))?;
    let output_ptr = ((packed >> 32) as u32) as usize;
    let output_len = (packed as u32) as usize;
    if output_len > MAX_OUTPUT_BYTES {
        return Err(AppError::Plugin("Plugin output is too large.".into()));
    }
    let mut output = vec![0u8; output_len];
    memory
        .read(&store, output_ptr, &mut output)
        .map_err(|_| AppError::Plugin("Plugin output read failed.".into()))?;
    let json = String::from_utf8(output)
        .map_err(|_| AppError::Plugin("Plugin output is not valid UTF-8.".into()))?;
    let fuel_used = FUEL_LIMIT.saturating_sub(store.get_fuel().unwrap_or(0));
    Ok(RuntimeOutput {
        json,
        logs: store.data().logs.clone(),
        fuel_used,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wasm_with_output(json: &str) -> Vec<u8> {
        let escaped = json
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n");
        let packed = ((1024_i64) << 32) | (json.len() as i64);
        wat::parse_str(format!(
            r#"(module
                (import "env" "host_log" (func $log (param i32 i32)))
                (memory (export "memory") 2)
                (global $next (mut i32) (i32.const 2048))
                (data (i32.const 1024) "{escaped}")
                (func (export "alloc") (param $len i32) (result i32)
                    (local $p i32)
                    global.get $next
                    local.set $p
                    global.get $next
                    local.get $len
                    i32.add
                    global.set $next
                    local.get $p)
                (func (export "run") (param $ptr i32) (param $len i32) (result i64)
                    i32.const 1024
                    i32.const {len}
                    call $log
                    i64.const {packed}))"#,
            len = json.len()
        ))
        .expect("valid wat")
    }

    #[test]
    fn runs_plugin_and_captures_logs() {
        let output = r#"{"summary":"ok","testCases":[],"findings":[],"notes":[]}"#;
        let wasm = wasm_with_output(output);
        let result = run_wasm(&wasm, r#"{"schema":"testtracker.plugin.input.v1"}"#).expect("run");
        assert_eq!(result.json, output);
        assert_eq!(result.logs, vec![output.to_string()]);
        assert!(result.fuel_used > 0);
    }

    #[test]
    fn rejects_infinite_plugin() {
        let wasm = wat::parse_str(
            r#"(module
                (memory (export "memory") 1)
                (func (export "alloc") (param i32) (result i32) i32.const 1024)
                (func (export "run") (param i32 i32) (result i64)
                    (loop $loop (br $loop))
                    i64.const 0))"#,
        )
        .expect("wasm");
        assert!(run_wasm(&wasm, "{}").is_err());
    }

    #[test]
    fn rejects_missing_exports() {
        let wasm = wat::parse_str(r#"(module (memory (export "memory") 1))"#).expect("wasm");
        assert!(run_wasm(&wasm, "{}").is_err());
    }

    #[test]
    fn rejects_oversized_input() {
        let wasm = wasm_with_output("{}");
        let input = "x".repeat(MAX_INPUT_BYTES + 1);
        assert!(run_wasm(&wasm, &input).is_err());
    }

    #[test]
    fn validates_wasm_bytes() {
        assert!(validate_wasm(&wasm_with_output("{}")).is_ok());
        assert!(validate_wasm(b"not wasm").is_err());
    }
}
