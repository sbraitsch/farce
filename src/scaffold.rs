use anyhow::{anyhow, Result};
use std::fs;

use axum::{extract::Path as UrlPath, http::StatusCode, response::IntoResponse, Json};
use serde_json::json;

use crate::Function;

pub async fn retrieve(UrlPath(function): UrlPath<Function>) -> impl IntoResponse {
    match read_scaffold(&function) {
        Ok(boilerplate) => Json(json!({
            "scaffold": boilerplate,
        }))
        .into_response(),
        Err(err) => (
            StatusCode::NOT_FOUND,
            Json(json!({"error": err.to_string()})),
        )
            .into_response(),
    }
}

fn read_scaffold(function: &Function) -> Result<String> {
    let scaffold = format!(
        "templates/{}/src/scaffold.rs",
        json!(&function)
            .as_str()
            .ok_or(anyhow!("Error deserializing function enum"))?
    );
    Ok(fs::read_to_string(scaffold)?)
}
