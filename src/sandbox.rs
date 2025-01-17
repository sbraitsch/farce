use anyhow::anyhow;
use axum::Json;
use serde_json::json;
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
use wasmtime::{Engine, Linker, Memory, Module, Store, Trap};

use crate::{resolve_string, CodeSubmission, Function};

#[derive(Serialize)]
pub struct ExecutionResult {
    log: Option<String>,
    out: serde_json::Value,
}

pub async fn execute_code(Json(payload): Json<CodeSubmission>) -> Json<ExecutionResult> {
    match compile_and_run_wasm(&payload).await {
        Ok(result) => Json(result),
        Err(err) => {
            if let Some(oof) = err.downcast_ref::<Trap>() {
                if matches!(oof, Trap::OutOfFuel) {
                    return Json(ExecutionResult {
                        log: Some(String::from(
                            "Instruction maximum exceeded. Aborted execution to avoid DOS.",
                        )),
                        out: serde_json::Value::Null,
                    });
                }
            }
            Json(ExecutionResult {
                log: Some(format!("Error: {}", err)),
                out: serde_json::Value::Null,
            })
        }
    }
}

fn write_file(path: &Path, source_code: &str) -> Result<(), anyhow::Error> {
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(path)?;

    write!(file, "{}", source_code)?;

    Ok(())
}

fn customize_cargo(path: &Path, unique_name: &str) -> Result<(), anyhow::Error> {
    let mut file = OpenOptions::new().append(true).open(path)?;

    writeln!(file, "name = \"{}\"", unique_name)?;
    Ok(())
}

fn copy_template(src: &Path, dst: &Path) -> Result<(), anyhow::Error> {
    if !dst.exists() {
        fs::create_dir_all(dst)?;
    }

    for entry in fs::read_dir(src)? {
        let entry = entry?;
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

async fn compile_and_run_wasm(payload: &CodeSubmission) -> Result<ExecutionResult, anyhow::Error> {
    let temp_dir = tempdir()?;

    let template = format!(
        "templates/{}",
        json!(&payload.function)
            .as_str()
            .ok_or(anyhow!("Error deserializing function enum"))?
    );
    let src_dir = Path::new(&template);
    let dst_dir = temp_dir.path();
    copy_template(src_dir, dst_dir)?;

    if !matches!(payload.function, Function::Param) {
        write_file(&dst_dir.join("src/boilerplate.rs"), &payload.source_code)?
    }

    let unique_name = format!(
        "user{}",
        dst_dir
            .file_name()
            .ok_or(anyhow!("Error reading generated temp name."))?
            .to_str()
            .ok_or(anyhow!("Error converting temp name to string"))?
    )
    .replace(".", "_");
    customize_cargo(&dst_dir.join("cargo.toml"), &unique_name)?;

    let target_dir = env::current_dir()?.join("target");

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

    run_wasm(&unique_name, &payload)
}

fn run_wasm(file_name: &str, payload: &CodeSubmission) -> Result<ExecutionResult, anyhow::Error> {
    let engine = Engine::new(wasmtime::Config::new().consume_fuel(true))?;
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
    store.set_fuel(500_000)?;

    let module = Module::from_file(
        &engine,
        format!("target/wasm32-wasip1/release/{file_name}.wasm"),
    )?;
    let instance = linker.instantiate(&mut store, &module)?;


    let memory: Memory = instance.get_memory(&mut store, "memory").unwrap();

    let ptr;

    if let Function::Param = payload.function {
        let run = instance.get_typed_func::<(i32, i32), i32>(&mut store, "run")?;
        let param = payload.param.clone().ok_or(anyhow!(
                    "Param function called without passing a parameter."
                ))?;

        let offset = 0;
        let length = param.len();
        memory.write(
            &mut store,
            offset,
            param.as_bytes(),
        )?;

        ptr = run.call(&mut store, (offset as i32, length as i32))? as usize;
    } else {
        let run = instance.get_typed_func::<(), i32>(&mut store, "run")?;
        ptr = run.call(&mut store, ())? as usize;
    }


    let read = stdout_buffer.read().unwrap();
    let output = String::from_utf8_lossy(&read);

    let stdout = if output.is_empty() {
        Some(output.to_string())
    } else {
        None
    };

    let result = resolve_string(memory.data(&store), ptr)?;
    let deserialized: serde_json::Value = serde_json::from_str(&result)?;

    fs::remove_file(format!("target/wasm32-wasip1/release/{file_name}.wasm"))?;
    fs::remove_file(format!("target/wasm32-wasip1/release/{file_name}.d"))?;

    Ok(ExecutionResult {
        log: stdout,
        out: deserialized,
    })
}
