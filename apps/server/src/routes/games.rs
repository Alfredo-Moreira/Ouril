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
    online, relay,
    state::AppState,
};

/// Answer with the game, and tell the relay both players' views changed (ADR 0033).
fn changed(state: &AppState, game: OnlineGame) -> Json<OnlineGame> {
    relay::game_changed(state.relay.as_ref(), &game);
    Json(game)
}

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
        .route("/games/{id}/abort", post(abort))
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
    Ok((StatusCode::CREATED, changed(&state, game)))
}

async fn join(
    State(state): State<AppState>,
    auth: AuthUser,
    ApiJson(req): ApiJson<JoinGameRequest>,
) -> AppResult<Json<OnlineGame>> {
    let game = online::join(&state.db, auth.user_id, req.invite_code.trim()).await?;
    Ok(changed(&state, game))
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
    let game = online::accept(&state.db, auth.user_id, game_id(&id)?).await?;
    Ok(changed(&state, game))
}

async fn decline(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> AppResult<Json<OnlineGame>> {
    let game = online::close_invitation(&state.db, auth.user_id, game_id(&id)?, false).await?;
    Ok(changed(&state, game))
}

async fn cancel(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> AppResult<Json<OnlineGame>> {
    let game = online::close_invitation(&state.db, auth.user_id, game_id(&id)?, true).await?;
    Ok(changed(&state, game))
}

async fn play(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
    ApiJson(req): ApiJson<MoveRequest>,
) -> AppResult<Json<OnlineGame>> {
    let game = online::play(&state.db, auth.user_id, game_id(&id)?, req).await?;
    Ok(changed(&state, game))
}

async fn resign(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> AppResult<Json<OnlineGame>> {
    let game = online::resign(&state.db, auth.user_id, game_id(&id)?).await?;
    Ok(changed(&state, game))
}

async fn abort(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> AppResult<Json<OnlineGame>> {
    let game = online::abort_live(&state.db, auth.user_id, game_id(&id)?).await?;
    Ok(changed(&state, game))
}

async fn opponents(State(state): State<AppState>, auth: AuthUser) -> AppResult<Json<OpponentList>> {
    Ok(Json(online::opponents(&state.db, auth.user_id).await?))
}
