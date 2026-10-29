//! `POST /v1/internal/jobs/{name}`: scheduled jobs, called by `ouril-jobs` on the homelab with
//! `Authorization: Bearer <JOBS_TOKEN>` (ADR 0032). Without `JOBS_TOKEN` the route is a 404.

use std::time::Instant;

use axum::{
    extract::{Path, State},
    http::{header, HeaderMap},
    routing::post,
    Json, Router,
};
use serde_json::{json, Value};

use crate::{
    error::{AppError, AppResult},
    jobs,
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new().route("/internal/jobs/{name}", post(run))
}

/// Constant-time comparison, so timing reveals nothing about the token.
fn same_secret(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

async fn run(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(name): Path<String>,
) -> AppResult<Json<Value>> {
    let Some(token) = state.config.jobs_token.as_deref() else {
        return Err(AppError::not_found());
    };
    let presented = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .unwrap_or_default();
    if !same_secret(presented.as_bytes(), token.as_bytes()) {
        return Err(AppError::unauthorized("bad job token"));
    }
    let started = Instant::now();
    let rows = jobs::run(&state.db, &name).await?;
    Ok(Json(json!({
        "job": name,
        "rows": rows,
        "took_ms": started.elapsed().as_millis() as u64,
    })))
}
