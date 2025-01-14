use anyhow::anyhow;
use axum::Json;
use std::{
    env,
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
    process::Command,
    sync::{Arc, RwLock},
};
use tempfile::tempdir;

use serde::Serialize;
use wasi_common::{pipe::WritePipe, sync::WasiCtxBuilder};
use wasmtime::{Engine, Linker, Memory, Module, Store};

use crate::{resolve_string, CodeSubmission};

#[derive(Serialize)]
pub struct ExecutionResultV2 {
    stdout: Option<String>,
    result: serde_json::Value,
}

pub async fn execute_code(Json(payload): Json<CodeSubmission>) -> Json<ExecutionResultV2> {
    match compile_and_run_wasm(&payload.source_code).await {
        Ok(result) => Json(result),
        Err(err) => Json(ExecutionResultV2 {
            stdout: Some(format!("Error: {}", err)),
            result: serde_json::Value::Null,
        }),
    }
}

fn write_file(path: &Path, source_code: &str) -> Result<(), anyhow::Error> {
    println!("Writing submitted code to {path:?}");
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(path)?;

    let import = "use serde::Serialize;";
    writeln!(file, "{}", import)?;
    write!(file, "{}", source_code)?;

    Ok(())
}

fn copy_template(src: &Path, dst: &Path) -> Result<(), anyhow::Error> {
    println!("Begin copying template dir");
    if !dst.exists() {
        fs::create_dir_all(dst)?;
    }

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        println!("Copying {:?}", entry.path());
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_template(&src_path, &dst_path)?;
        } else {
            fs::copy(src_path, dst_path)?;
        }
    }
    Ok(())
}

async fn compile_and_run_wasm(source_code: &str) -> Result<ExecutionResultV2, anyhow::Error> {
    let temp_dir = tempdir()?;
    let src_dir = Path::new("template");
    let dst_dir = temp_dir.path();
    copy_template(src_dir, dst_dir)?;

    write_file(&dst_dir.join("src/submission.rs"), source_code)?;

    let target_dir = env::current_dir()?.join("target");
    println!("Compiling to {target_dir:?}");
    println!("Temp dir is {dst_dir:?}");
    let output = Command::new("cargo")
        .args([
            "build",
            "--release",
            "--target",
            "wasm32-wasip1",
            "--target-dir",
            target_dir
                .to_str()
                .ok_or(anyhow!("Failed to convert target directory to str."))?,
        ])
        .current_dir(dst_dir)
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

    println!("Building wasm module from file.");
    let module = Module::from_file(&engine, "target/wasm32-wasip1/release/user_code.wasm")?;
    let instance = linker.instantiate(&mut store, &module)?;

    let memory: Memory = instance.get_memory(&mut store, "memory").unwrap();

    let run = instance.get_typed_func::<(), i32>(&mut store, "run")?;

    let ptr = run.call(&mut store, ())? as usize;

    let read = stdout_buffer.read().unwrap();
    let output = String::from_utf8_lossy(&read);

    let stdout = if output.len() > 0 { Some(output.to_string()) } else { None };
    
    let result = resolve_string(&memory.data(&store), ptr)?;
    let deserialized: serde_json::Value = serde_json::from_str(&result)?;

    Ok(ExecutionResultV2 {
        stdout,
        result: deserialized,
    })
}
