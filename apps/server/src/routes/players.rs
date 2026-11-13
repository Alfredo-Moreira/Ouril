//! `GET /v1/players/{id}`: a player's profile, by ID or `@handle` (auth required).

use axum::{
    extract::{Path, State},
    routing::get,
    Json, Router,
};
use ouril_protocol::PlayerProfile;

use crate::{auth::AuthUser, error::AppResult, profiles, state::AppState};

pub fn router() -> Router<AppState> {
    Router::new().route("/players/{key}", get(profile))
}

async fn profile(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(key): Path<String>,
) -> AppResult<Json<PlayerProfile>> {
    Ok(Json(
        profiles::profile(&state.db, auth.user_id, key.trim()).await?,
    ))
}
