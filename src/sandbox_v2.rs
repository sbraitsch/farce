use anyhow::anyhow;
use axum::Json;
use std::{
    fs::OpenOptions,
    io::Write,
    process::Command,
    sync::{Arc, RwLock},
};

use serde::Serialize;
use wasi_common::{pipe::WritePipe, sync::WasiCtxBuilder};
use wasmtime::{Engine, Linker, Memory, Module, Store};

use crate::{resolve_string, CodeSubmission};

#[derive(Serialize)]
pub struct ExecutionResultV2 {
    stdout: String,
    result: serde_json::Value,
}

pub async fn execute_code(Json(payload): Json<CodeSubmission>) -> Json<ExecutionResultV2> {
    match compile_and_run_wasm(&payload.source_code).await {
        Ok(result) => Json(result),
        Err(err) => Json(ExecutionResultV2 {
            stdout: format!("Error: {}", err),
            result: serde_json::Value::Null,
        }),
    }
}

fn write_file(source_code: &str) -> Result<(), anyhow::Error> {
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open("./template/src/submission.rs")?;

    file.write_all(source_code.as_bytes())?;

    Ok(())
}

async fn compile_and_run_wasm(source_code: &str) -> Result<ExecutionResultV2, anyhow::Error> {
    let _file = write_file(source_code)?;
    let output = Command::new("cargo")
        .args([
            "build",
            "--release",
            "--target",
            "wasm32-wasip1",
            "--target-dir",
            "../target",
        ])
        .current_dir("./template")
        .output()?;

    if !output.status.success() {
        return Err(anyhow!(
            "Compilation failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    run_wasm()
}

fn run_wasm() -> Result<ExecutionResultV2, anyhow::Error> {
    let engine = Engine::default();
    let mut linker = Linker::new(&engine);
    // include necessary imports
    wasi_common::sync::add_to_linker(&mut linker, |s| s)?;

    let stdout_buffer = Arc::new(RwLock::new(Vec::new()));
    let stdout_pipe = WritePipe::from_shared(stdout_buffer.clone());

    let wasi = WasiCtxBuilder::new()
        .inherit_stdio()
        .stdout(Box::new(stdout_pipe))
        .inherit_args()?
        .build();

    let mut store = Store::new(&engine, wasi);

    let module = Module::from_file(&engine, "target/wasm32-wasip1/release/user_code.wasm")?;
    let instance = linker.instantiate(&mut store, &module)?;

    let memory: Memory = instance.get_memory(&mut store, "memory").unwrap();

    let run = instance.get_typed_func::<(), i32>(&mut store, "run")?;

    let ptr = run.call(&mut store, ())? as usize;

    let read = stdout_buffer.read().unwrap();
    let output = String::from_utf8_lossy(&read);

    let result = resolve_string(&memory.data(&store), ptr)?;
    let deserialized: serde_json::Value = serde_json::from_str(&result)?;

    Ok(ExecutionResultV2 {
        stdout: output.to_string(),
        result: deserialized,
    })
}
