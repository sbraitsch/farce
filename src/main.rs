use std::{
    fs::OpenOptions,
    io::Write,
    process::Command,
    sync::{Arc, RwLock},
};

use anyhow::anyhow;
use axum::{response::Json as JsonResponse, routing::post, Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use wasi_common::{pipe::WritePipe, sync::WasiCtxBuilder};
use wasmtime::{Engine, Linker, Memory, Module, Store};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let app = Router::new().route("/execute", post(execute_code));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("Server running on port 3000");
    axum::serve(listener, app).await.unwrap();
}

#[derive(Deserialize)]
struct CodeSubmission {
    source_code: String,
}

#[derive(Serialize)]
struct ExecutionResult {
    stdout: String,
    result: Value,
}

async fn execute_code(Json(payload): Json<CodeSubmission>) -> JsonResponse<ExecutionResult> {
    match compile_and_run_wasm(&payload.source_code).await {
        Ok(result) => JsonResponse(result),
        Err(err) => JsonResponse(ExecutionResult {
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

async fn compile_and_run_wasm(source_code: &str) -> Result<ExecutionResult, anyhow::Error> {
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

fn run_wasm() -> Result<ExecutionResult, anyhow::Error> {
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

    let comp_res = resolve_string(&memory.data(&store), ptr)?;
    let deserialized: Value = serde_json::from_str(&comp_res)?;

    Ok(ExecutionResult {
        stdout: output.to_string(),
        result: deserialized,
    })
}

fn resolve_string(memory: &[u8], ptr: usize) -> Result<String, anyhow::Error> {
    let s = &memory[ptr..ptr + 8];
    let (p, l) = s.split_at(4);
    let (str_ptr, length) = (
        i32::from_ne_bytes(p.try_into()?) as usize,
        i32::from_ne_bytes(l.try_into()?) as usize,
    );
    let string_bytes = &memory[str_ptr..str_ptr + length];

    Ok(String::from_utf8(string_bytes.to_vec())?)
}
