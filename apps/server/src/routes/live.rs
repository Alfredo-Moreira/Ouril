//! `/v1/live/…`: matchmaking for live games and relay tokens (ADR 0033, auth required).

use axum::{extract::State, routing::post, Json, Router};
use ouril_protocol::{
    game_channel, user_channel, LiveToken, LiveTokenRequest, SeekRequest, SeekStatus,
};
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    error::{ApiJson, AppError, AppResult},
    live, relay,
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/live/seek", post(seek).get(status).delete(cancel))
        .route("/live/token", post(token))
}

async fn seek(
    State(state): State<AppState>,
    auth: AuthUser,
    ApiJson(req): ApiJson<SeekRequest>,
) -> AppResult<Json<SeekStatus>> {
    let (status, matched) = live::seek(&state.db, auth.user_id, req).await?;
    if let Some(m) = matched {
        relay::matched(state.relay.as_ref(), m.waiting, m.game);
    }
    Ok(Json(status))
}

async fn status(State(state): State<AppState>, auth: AuthUser) -> AppResult<Json<SeekStatus>> {
    Ok(Json(live::status(&state.db, auth.user_id).await?))
}

async fn cancel(State(state): State<AppState>, auth: AuthUser) -> AppResult<Json<SeekStatus>> {
    Ok(Json(live::cancel(&state.db, auth.user_id).await?))
}

/// A relay token for the player's own channel and, if asked, a game they're playing.
async fn token(
    State(state): State<AppState>,
    auth: AuthUser,
    ApiJson(req): ApiJson<LiveTokenRequest>,
) -> AppResult<Json<LiveToken>> {
    let relay = state
        .relay
        .as_ref()
        .ok_or_else(|| AppError::not_configured("the live relay is not configured"))?;
    let mut channels = vec![user_channel(&auth.user_id.to_string())];
    if let Some(id) = req.game_id {
        // An unknown game, or one you're not in, is a 404 like everywhere else.
        let id = Uuid::parse_str(&id).map_err(|_| AppError::not_found())?;
        let mine: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM online_games
                             WHERE id = $1 AND (south_user = $2 OR north_user = $2))",
        )
        .bind(id)
        .bind(auth.user_id)
        .fetch_one(&state.db)
        .await?;
        if !mine {
            return Err(AppError::not_found());
        }
        channels.push(game_channel(&id.to_string()));
    }
    Ok(Json(relay.token(auth.user_id, channels)?))
}
