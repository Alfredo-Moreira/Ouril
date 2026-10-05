//! `/v1/sync/*`: offline-first sync (auth required). See docs/architecture/data-and-sync.md.

use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use ouril_protocol::{
    SyncPull, SyncPullQuery, SyncPush, SyncPushResult, MAX_MUTATIONS_PER_PUSH, PULL_DEFAULT_LIMIT,
    PULL_MAX_LIMIT,
};

use crate::{
    auth::AuthUser,
    error::{ApiJson, ApiQuery, AppError, AppResult},
    state::AppState,
    sync,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/sync/push", post(push))
        .route("/sync/pull", get(pull))
}

/// Apply mutations in request order, one transaction each. Results come back in the same
/// order. A server error stops the batch (500); mutations applied before it stay applied
/// and are answered `applied` again when the client retries.
async fn push(
    State(state): State<AppState>,
    auth: AuthUser,
    ApiJson(body): ApiJson<SyncPush>,
) -> AppResult<Json<SyncPushResult>> {
    if body.mutations.len() > MAX_MUTATIONS_PER_PUSH {
        return Err(AppError::payload_too_large(format!(
            "at most {MAX_MUTATIONS_PER_PUSH} mutations per request"
        )));
    }
    let mut results = Vec::with_capacity(body.mutations.len());
    for m in &body.mutations {
        results.push(sync::apply_mutation(&state.db, auth.user_id, m).await?);
    }
    let rejected = results
        .iter()
        .filter(|r| r.status == ouril_protocol::MutationStatus::Rejected)
        .count();
    tracing::info!(total = results.len(), rejected, "sync push");
    Ok(Json(SyncPushResult { results }))
}

async fn pull(
    State(state): State<AppState>,
    auth: AuthUser,
    ApiQuery(q): ApiQuery<SyncPullQuery>,
) -> AppResult<Json<SyncPull>> {
    let cursor = q.cursor.unwrap_or(0);
    let cursor = i64::try_from(cursor).map_err(|_| AppError::bad_request("cursor is too large"))?;
    let limit = q.limit.unwrap_or(PULL_DEFAULT_LIMIT);
    if !(1..=PULL_MAX_LIMIT).contains(&limit) {
        return Err(AppError::bad_request(format!(
            "limit must be between 1 and {PULL_MAX_LIMIT}"
        )));
    }
    let (changes, has_more) = sync::pull(&state.db, auth.user_id, cursor, i64::from(limit)).await?;
    let new_cursor = changes
        .last()
        .map(|c| c.change_seq)
        .unwrap_or(cursor as u64);
    Ok(Json(SyncPull {
        changes,
        cursor: new_cursor,
        has_more,
    }))
}
