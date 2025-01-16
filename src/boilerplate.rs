use anyhow::Result;
use std::fs;

use axum::{extract::Path as UrlPath, http::StatusCode, response::IntoResponse, Json};
use serde_json::json;

pub async fn retrieve(UrlPath(function): UrlPath<String>) -> impl IntoResponse {
    match read_boilerplate(&function) {
        Ok(boilerplate) => Json(json!({
            "boilerplate": boilerplate,
        }))
        .into_response(),
        Err(err) => (
            StatusCode::NOT_FOUND,
            Json(json!({"error": err.to_string()})),
        )
            .into_response(),
    }
}

fn read_boilerplate(function: &str) -> Result<String> {
    let boilerplate_file = format!("templates/{}/src/boilerplate.rs", function);
    Ok(fs::read_to_string(boilerplate_file)?)
}
