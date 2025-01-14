#![allow(dead_code)]
mod sandbox_v1;
mod sandbox_v2;

use axum::{routing::post, Router};
use tower_http::cors::CorsLayer;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct CodeSubmission {
    source_code: String,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let app = Router::new()
        .route("/v1/execute", post(sandbox_v1::execute_code))
        .route("/v2/execute", post(sandbox_v2::execute_code))
        .layer(CorsLayer::permissive());
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("Server running on port 3000");
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
