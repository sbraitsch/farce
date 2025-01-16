#![allow(dead_code)]
mod sandbox;
mod boilerplate;

use axum::{routing::{ post, get }, Router};
use tower_http::cors::CorsLayer;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct CodeSubmission {
    source_code: String,
    function: String,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let app = Router::new()
        .route("/farce/execute", post(sandbox::execute_code))
        .route("/farce/boilerplate/{function}", get(boilerplate::retrieve))
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
