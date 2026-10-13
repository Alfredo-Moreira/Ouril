//! `/v1/games` and `/v1/opponents`: games against other players (ADR 0031, auth required).

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use ouril_protocol::{
    CreateGameRequest, GameList, JoinGameRequest, MoveRequest, OnlineGame, OpponentList,
};
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    error::{ApiJson, AppError, AppResult},
    online,
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/games", get(list).post(create))
        .route("/games/join", post(join))
        .route("/games/{id}", get(get_game))
        .route("/games/{id}/accept", post(accept))
        .route("/games/{id}/decline", post(decline))
        .route("/games/{id}/cancel", post(cancel))
        .route("/games/{id}/moves", post(play))
        .route("/games/{id}/resign", post(resign))
        .route("/opponents", get(opponents))
}

fn game_id(id: &str) -> AppResult<Uuid> {
    // An unknown or malformed ID is a 404, like a game you aren't part of.
    Uuid::parse_str(id).map_err(|_| AppError::not_found())
}

async fn list(State(state): State<AppState>, auth: AuthUser) -> AppResult<Json<GameList>> {
    Ok(Json(online::list(&state.db, auth.user_id).await?))
}

async fn create(
    State(state): State<AppState>,
    auth: AuthUser,
    ApiJson(req): ApiJson<CreateGameRequest>,
) -> AppResult<(StatusCode, Json<OnlineGame>)> {
    let game = online::create(&state.db, auth.user_id, req).await?;
    Ok((StatusCode::CREATED, Json(game)))
}

async fn join(
    State(state): State<AppState>,
    auth: AuthUser,
    ApiJson(req): ApiJson<JoinGameRequest>,
) -> AppResult<Json<OnlineGame>> {
    Ok(Json(
        online::join(&state.db, auth.user_id, req.invite_code.trim()).await?,
    ))
}

async fn get_game(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> AppResult<Json<OnlineGame>> {
    Ok(Json(
        online::get(&state.db, auth.user_id, game_id(&id)?).await?,
    ))
}

async fn accept(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> AppResult<Json<OnlineGame>> {
    Ok(Json(
        online::accept(&state.db, auth.user_id, game_id(&id)?).await?,
    ))
}

async fn decline(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> AppResult<Json<OnlineGame>> {
    Ok(Json(
        online::close_invitation(&state.db, auth.user_id, game_id(&id)?, false).await?,
    ))
}

async fn cancel(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> AppResult<Json<OnlineGame>> {
    Ok(Json(
        online::close_invitation(&state.db, auth.user_id, game_id(&id)?, true).await?,
    ))
}

async fn play(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
    ApiJson(req): ApiJson<MoveRequest>,
) -> AppResult<Json<OnlineGame>> {
    Ok(Json(
        online::play(&state.db, auth.user_id, game_id(&id)?, req).await?,
    ))
}

async fn resign(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> AppResult<Json<OnlineGame>> {
    Ok(Json(
        online::resign(&state.db, auth.user_id, game_id(&id)?).await?,
    ))
}

async fn opponents(State(state): State<AppState>, auth: AuthUser) -> AppResult<Json<OpponentList>> {
    Ok(Json(online::opponents(&state.db, auth.user_id).await?))
}
