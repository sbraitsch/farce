use anyhow::anyhow;
use axum::Json;
use serde::Serialize;
use std::{fs::File, io::Write, process::Command};

use wasi_common::sync::WasiCtxBuilder;
use wasmtime::{Engine, Linker, Memory, Module, Store};

use crate::{resolve_string, CodeSubmission};

const WRAPPER: &str = r#"
#[no_mangle]
pub extern "C" fn run() -> *const i32 {
    let message = format!("{:?}", execute());
    let ptr = message.as_ptr() as i32;
    let length = message.len() as i32;
    std::mem::forget(message);
    let res = Box::new([ptr, length]);
    Box::into_raw(res) as *const i32
}

"#;

#[derive(Serialize)]
pub struct ExecutionResultV1 {
    result: String,
}

pub async fn execute_code(Json(payload): Json<CodeSubmission>) -> Json<ExecutionResultV1> {
    match compile_and_run_wasm(&payload.source_code).await {
        Ok(result) => Json(result),
        Err(err) => Json(ExecutionResultV1 {
            result: format!("Error: {}", err),
        }),
    }
}

fn write_file(source_code: &str) -> Result<(), anyhow::Error> {
    let mut file = File::create("user_code.rs")?;
    write!(file, "{}", WRAPPER)?;
    write!(file, "{}", source_code)?;
    Ok(())
}

pub async fn compile_and_run_wasm(source_code: &str) -> Result<ExecutionResultV1, anyhow::Error> {
    let _file = write_file(source_code)?;
    let output = Command::new("rustc")
        .args([
            "--target",
            "wasm32-wasip1",
            "--crate-type=cdylib",
            "user_code.rs",
            "-o",
            "user_code.wasm",
        ])
        .output()?;

    if !output.status.success() {
        return Err(anyhow!(
            "Compilation failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    run_wasm()
}

fn run_wasm() -> Result<ExecutionResultV1, anyhow::Error> {
    let engine = Engine::default();
    let mut linker = Linker::new(&engine);
    // include necessary imports
    wasi_common::sync::add_to_linker(&mut linker, |s| s)?;

    let wasi = WasiCtxBuilder::new()
        .inherit_stdio()
        .inherit_args()?
        .build();

    let mut store = Store::new(&engine, wasi);

    let module = Module::from_file(&engine, "user_code.wasm")?;
    let instance = linker.instantiate(&mut store, &module)?;

    let memory: Memory = instance.get_memory(&mut store, "memory").unwrap();

    let run = instance.get_typed_func::<(), i32>(&mut store, "run")?;

    let ptr = run.call(&mut store, ())? as usize;

    let result = resolve_string(&memory.data(&store), ptr)?;

    Ok(ExecutionResultV1 { result })
}
