//! `/v1/ratings/me` and `/v1/leaderboard`: ratings and leaderboards (ADR 0034, auth required).

use axum::{extract::State, routing::get, Json, Router};
use ouril_protocol::{Leaderboard, LeaderboardScope, MyRatings, PointsBoard};
use serde::Deserialize;

use crate::{
    auth::AuthUser,
    error::{ApiQuery, AppError, AppResult},
    points, ratings,
    state::AppState,
    validate,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/ratings/me", get(mine))
        .route("/leaderboard", get(leaderboard))
        .route("/points", get(points_board))
}

async fn mine(State(state): State<AppState>, auth: AuthUser) -> AppResult<Json<MyRatings>> {
    Ok(Json(ratings::mine(&state.db, auth.user_id).await?))
}

#[derive(Deserialize)]
struct BoardQuery {
    pool: String,
    #[serde(default)]
    scope: Option<LeaderboardScope>,
    #[serde(default)]
    country: Option<String>,
}

async fn leaderboard(
    State(state): State<AppState>,
    auth: AuthUser,
    ApiQuery(q): ApiQuery<BoardQuery>,
) -> AppResult<Json<Leaderboard>> {
    if !ratings::is_valid_pool(&q.pool) {
        return Err(AppError::bad_request("unknown pool"));
    }
    let scope = q.scope.unwrap_or(LeaderboardScope::World);
    let country = match scope {
        LeaderboardScope::World => None,
        // A country board: the one asked for, else the player's own.
        LeaderboardScope::Country => {
            let c = match q.country {
                Some(c) => Some(c),
                None => ratings::country_of(&state.db, auth.user_id).await?,
            };
            let c = c.ok_or_else(|| AppError::bad_request("no country"))?;
            Some(validate::country(&c).map_err(AppError::bad_request)?)
        }
    };
    Ok(Json(
        ratings::leaderboard(&state.db, auth.user_id, &q.pool, scope, country).await?,
    ))
}

#[derive(Deserialize)]
struct PointsQuery {
    #[serde(default)]
    season: Option<String>,
    #[serde(default)]
    scope: Option<LeaderboardScope>,
    #[serde(default)]
    country: Option<String>,
}

async fn points_board(
    State(state): State<AppState>,
    auth: AuthUser,
    ApiQuery(q): ApiQuery<PointsQuery>,
) -> AppResult<Json<PointsBoard>> {
    let scope = q.scope.unwrap_or(LeaderboardScope::World);
    let country = match scope {
        LeaderboardScope::World => None,
        LeaderboardScope::Country => {
            let c = match q.country {
                Some(c) => Some(c),
                None => ratings::country_of(&state.db, auth.user_id).await?,
            };
            let c = c.ok_or_else(|| AppError::bad_request("no country"))?;
            Some(validate::country(&c).map_err(AppError::bad_request)?)
        }
    };
    Ok(Json(
        points::board(&state.db, auth.user_id, q.season, scope, country).await?,
    ))
}
