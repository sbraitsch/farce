#![allow(dead_code)]
mod sandbox;
mod scaffold;

use axum::{
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use tower_http::cors::CorsLayer;

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum Function {
    Arbitrary,
    Decode,
    Param,
    Prime,
}

#[derive(Deserialize)]
pub struct CodeSubmission {
    user_input: String,
    function: Function,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let app = Router::new()
        .route("/farce/execute", post(sandbox::execute_code))
        .route("/farce/scaffold/{function}", get(scaffold::retrieve))
        .layer(CorsLayer::permissive());
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8081").await.unwrap();
    println!("Server running on port 8081");
    axum::serve(listener, app).await.unwrap();
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
