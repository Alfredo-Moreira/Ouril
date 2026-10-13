//! Games against other players, played turn by turn through the server (ADR 0031).
//!
//! - The server owns the game: it stores the moves, replays them with the engine, and accepts
//!   a move only from the player to move, at the expected `ply`, when the engine says it's
//!   legal. Clients replay the same moves with the same engine to draw the board.
//! - The inviter plays South; who moves first is a coin toss. Invitations are by handle, by
//!   past opponent, or by an invite link anyone signed in can accept.
//! - Deadlines need no background job: a game is settled whenever it's read or written. An
//!   active game whose turn deadline passed is lost on time by the player to move; an
//!   invitation nobody accepted within `INVITE_TTL_DAYS` expires.
//! - A finished game is copied into both players' `games` history as a format-3 record
//!   (`mode: "vs_player"`), which sync pull delivers to their devices, and counted in
//!   `user_opponents`.

use ouril_engine::{
    apply_move, replay, variant, variant_version, GameResult, GameState, MoveEvent, Player, Status,
    VariantConfig,
};
use ouril_protocol::{
    CreateGameRequest, GameList, MoveRequest, OnlineGame, OnlineGameResult, OnlineGameStatus,
    OpponentList, OpponentSummary, PlayerSummary, Seat, INVITE_TTL_DAYS, MAX_OPEN_INVITES,
    TURN_DEADLINE_DAYS,
};
use ouril_sync::{
    GameMode, GameRecord, GameRecordResult, RecordEndReason, RecordOpponent, VariantRef,
    GAME_RECORD_FORMAT_VS_PLAYER,
};
use sqlx::{PgConnection, PgPool};
use time::{format_description::well_known::Rfc3339, Duration, OffsetDateTime};
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    opponents,
    ratings::{self, FinishedGame},
    sync::insert_game_record,
};

/// The variant of a new game when the request names none.
pub const DEFAULT_VARIANT: &str = "cv.standard";
/// Finished, declined, cancelled and expired games stay in a player's list this long.
const RECENT_DAYS: i64 = 30;
/// Most games returned by `GET /v1/games`.
const LIST_LIMIT: i64 = 50;

/// The `online_games` columns [`Row`] reads (a macro, so queries stay static strings).
macro_rules! columns {
    () => {
        "id, variant_id, variant_version, first_player, south_user, north_user,
    invited_user, invite_code, status, moves, turn_deadline, result_outcome, result_reason,
    store_south, store_north, created_at, updated_at, started_at, mode, rated, rating_applied"
    };
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct Row {
    id: Uuid,
    variant_id: String,
    variant_version: i32,
    first_player: String,
    south_user: Uuid,
    north_user: Option<Uuid>,
    invited_user: Option<Uuid>,
    invite_code: Option<String>,
    status: String,
    moves: Vec<i32>,
    turn_deadline: Option<OffsetDateTime>,
    result_outcome: Option<String>,
    result_reason: Option<String>,
    store_south: Option<i32>,
    store_north: Option<i32>,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
    started_at: Option<OffsetDateTime>,
    mode: String,
    rated: bool,
    rating_applied: bool,
}

impl Row {
    /// The seat `me` plays (or will play, as the invited player), if any.
    fn seat_of(&self, me: Uuid) -> Option<Seat> {
        if self.south_user == me {
            Some(Seat::South)
        } else if self.north_user == Some(me)
            || (self.north_user.is_none() && self.invited_user == Some(me))
        {
            Some(Seat::North)
        } else {
            None
        }
    }

    fn user_at(&self, seat: Seat) -> Option<Uuid> {
        match seat {
            Seat::South => Some(self.south_user),
            Seat::North => self.north_user.or(self.invited_user),
        }
    }

    fn moves(&self) -> Vec<u8> {
        self.moves.iter().map(|&m| m as u8).collect()
    }

    fn first_player(&self) -> Player {
        if self.first_player == "north" {
            Player::North
        } else {
            Player::South
        }
    }

    fn variant(&self) -> AppResult<&'static VariantConfig> {
        let version = u16::try_from(self.variant_version).map_err(|_| AppError::internal())?;
        variant_version(&self.variant_id, version).ok_or_else(AppError::internal)
    }

    /// The current position, from the stored moves.
    fn position(&self) -> AppResult<GameState> {
        let replayed =
            replay(self.variant()?, self.first_player(), &self.moves()).map_err(|e| {
                tracing::error!(game = %self.id, error = ?e, "stored moves don't replay");
                AppError::internal()
            })?;
        Ok(replayed.final_state().clone())
    }
}

fn player(seat: Seat) -> Player {
    match seat {
        Seat::South => Player::South,
        Seat::North => Player::North,
    }
}

fn other(seat: Seat) -> Seat {
    match seat {
        Seat::South => Seat::North,
        Seat::North => Seat::South,
    }
}

fn winner(seat: Seat) -> GameResult {
    match seat {
        Seat::South => GameResult::SouthWins,
        Seat::North => GameResult::NorthWins,
    }
}

fn rfc3339(t: OffsetDateTime) -> AppResult<String> {
    t.format(&Rfc3339).map_err(|_| AppError::internal())
}

/// The serde string of a unit enum (`GameResult::SouthWins` -> `"south_wins"`).
fn enum_str<T: serde::Serialize>(v: &T) -> String {
    serde_json::to_value(v)
        .ok()
        .and_then(|v| v.as_str().map(str::to_owned))
        .unwrap_or_default()
}

fn status_of(s: &str) -> OnlineGameStatus {
    match s {
        "active" => OnlineGameStatus::Active,
        "finished" => OnlineGameStatus::Finished,
        "declined" => OnlineGameStatus::Declined,
        "cancelled" => OnlineGameStatus::Cancelled,
        "expired" => OnlineGameStatus::Expired,
        _ => OnlineGameStatus::Invited,
    }
}

/// 32 random hex characters for an invite link.
fn invite_code() -> String {
    let mut buf = [0u8; 16];
    getrandom::fill(&mut buf).expect("OS random number generator unavailable");
    buf.iter().map(|b| format!("{b:02x}")).collect()
}

fn coin_toss() -> Player {
    let mut b = [0u8; 1];
    getrandom::fill(&mut b).expect("OS random number generator unavailable");
    if b[0] & 1 == 0 {
        Player::South
    } else {
        Player::North
    }
}

async fn summary(conn: &mut PgConnection, user_id: Uuid) -> AppResult<PlayerSummary> {
    let (display_name, handle): (String, Option<String>) =
        sqlx::query_as("SELECT display_name, handle FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_one(&mut *conn)
            .await?;
    Ok(PlayerSummary {
        user_id: user_id.to_string(),
        display_name,
        handle,
    })
}

/// The game as `me` sees it.
async fn view(conn: &mut PgConnection, row: &Row, me: Uuid) -> AppResult<OnlineGame> {
    let you = row.seat_of(me).ok_or_else(AppError::not_found)?;
    let opponent = match row.user_at(other(you)) {
        Some(id) => Some(summary(conn, id).await?),
        None => None,
    };
    let your_turn = row.status == "active" && row.position()?.to_move == player(you);
    let result = match (&row.result_outcome, &row.result_reason) {
        (Some(outcome), Some(reason)) => Some(OnlineGameResult {
            outcome: outcome.clone(),
            reason: reason.clone(),
            stores: [
                row.store_south.unwrap_or(0) as u8,
                row.store_north.unwrap_or(0) as u8,
            ],
        }),
        _ => None,
    };
    let invited_by_you = row.south_user == me;
    let rating_changes = if row.rating_applied {
        ratings::changes_for(conn, row.id, me).await?
    } else {
        Vec::new()
    };
    Ok(OnlineGame {
        id: row.id.to_string(),
        variant_id: row.variant_id.clone(),
        variant_version: row.variant_version as u16,
        status: status_of(&row.status),
        first_player: if row.first_player() == Player::North {
            Seat::North
        } else {
            Seat::South
        },
        you,
        opponent,
        moves: row.moves(),
        your_turn,
        turn_deadline: row.turn_deadline.map(rfc3339).transpose()?,
        result,
        invite_code: row
            .invite_code
            .clone()
            .filter(|_| invited_by_you && row.status == "invited"),
        invited_by_you,
        rated: row.rated,
        rating_changes,
        created_at: rfc3339(row.created_at)?,
        updated_at: rfc3339(row.updated_at)?,
    })
}

async fn lock(conn: &mut PgConnection, id: Uuid) -> AppResult<Option<Row>> {
    Ok(sqlx::query_as(concat!(
        "SELECT ",
        columns!(),
        " FROM online_games WHERE id = $1 FOR UPDATE"
    ))
    .bind(id)
    .fetch_optional(&mut *conn)
    .await?)
}

/// Lock a game `me` takes part in (404 otherwise, so game IDs reveal nothing), settled.
async fn lock_mine(conn: &mut PgConnection, id: Uuid, me: Uuid) -> AppResult<(Row, Seat)> {
    let mut row = lock(conn, id).await?.ok_or_else(AppError::not_found)?;
    settle(conn, &mut row, OffsetDateTime::now_utc()).await?;
    let seat = row.seat_of(me).ok_or_else(AppError::not_found)?;
    Ok((row, seat))
}

async fn reload(conn: &mut PgConnection, id: Uuid) -> AppResult<Row> {
    Ok(sqlx::query_as(concat!(
        "SELECT ",
        columns!(),
        " FROM online_games WHERE id = $1"
    ))
    .bind(id)
    .fetch_one(&mut *conn)
    .await?)
}

/// Apply what time has decided: an expired invitation, or a turn deadline that passed (the
/// player to move loses on time). The row must be locked.
async fn settle(conn: &mut PgConnection, row: &mut Row, now: OffsetDateTime) -> AppResult<()> {
    if row.status == "invited" && row.created_at + Duration::days(INVITE_TTL_DAYS) < now {
        sqlx::query(
            "UPDATE online_games SET status = 'expired', invite_code = NULL, updated_at = now()
             WHERE id = $1",
        )
        .bind(row.id)
        .execute(&mut *conn)
        .await?;
        *row = reload(conn, row.id).await?;
    } else if row.status == "active" && row.turn_deadline.is_some_and(|d| d < now) {
        let pos = row.position()?;
        let loser = if pos.to_move == Player::South {
            Seat::South
        } else {
            Seat::North
        };
        let ended = row.turn_deadline.expect("checked");
        finish(
            conn,
            row,
            winner(other(loser)),
            RecordEndReason::Timeout,
            pos.stores,
            ended,
            true,
        )
        .await?;
    }
    Ok(())
}

/// End a game: store the result, copy it into both players' history (skipping a deleted
/// account), count it in `user_opponents`, and, with `rate` for a rated game, update both
/// players' ratings (ADR 0034). The row must be locked.
async fn finish(
    conn: &mut PgConnection,
    row: &mut Row,
    outcome: GameResult,
    reason: RecordEndReason,
    stores: [u8; 2],
    ended: OffsetDateTime,
    rate: bool,
) -> AppResult<()> {
    sqlx::query(
        "UPDATE online_games
            SET status = 'finished', result_outcome = $2, result_reason = $3, store_south = $4,
                store_north = $5, ended_at = $6, turn_deadline = NULL, updated_at = now()
          WHERE id = $1",
    )
    .bind(row.id)
    .bind(enum_str(&outcome))
    .bind(enum_str(&reason))
    .bind(i32::from(stores[0]))
    .bind(i32::from(stores[1]))
    .bind(ended)
    .execute(&mut *conn)
    .await?;
    *row = reload(conn, row.id).await?;

    let north = row.north_user.ok_or_else(AppError::internal)?;
    let started = row.started_at.unwrap_or(row.created_at);
    for (me, seat, them) in [
        (row.south_user, Seat::South, north),
        (north, Seat::North, row.south_user),
    ] {
        let deleted: bool =
            sqlx::query_scalar("SELECT deleted_at IS NOT NULL FROM users WHERE id = $1")
                .bind(me)
                .fetch_one(&mut *conn)
                .await?;
        if deleted {
            continue;
        }
        let opponent = summary(conn, them).await?;
        let record = GameRecord {
            format: GAME_RECORD_FORMAT_VS_PLAYER,
            id: Uuid::now_v7().to_string(),
            variant: VariantRef {
                id: row.variant_id.clone(),
                version: row.variant_version as u16,
            },
            first_player: row.first_player(),
            moves: row.moves(),
            core_version: env!("CARGO_PKG_VERSION").to_string(),
            result: GameRecordResult {
                outcome,
                stores,
                reason,
            },
            started_at: rfc3339(started)?,
            ended_at: rfc3339(ended)?,
            mode: GameMode::VsPlayer,
            ai_level: None,
            human_player: Some(player(seat)),
            opponent: Some(RecordOpponent {
                user_id: opponent.user_id,
                display_name: opponent.display_name,
                handle: opponent.handle,
            }),
        };
        insert_game_record(conn, me, &record).await?;
    }
    opponents::record_game_between(&mut *conn, row.south_user, north, ended).await?;
    if rate && row.rated {
        ratings::rate_finished_game(
            conn,
            &FinishedGame {
                id: row.id,
                variant_id: row.variant_id.clone(),
                mode: row.mode.clone(),
                south: row.south_user,
                north,
                outcome,
                plies: row.moves.len(),
                ended,
            },
        )
        .await?;
        *row = reload(conn, row.id).await?;
    }
    Ok(())
}

/// Find an account to invite, by handle or ID (404 if unknown or deleted).
async fn find_opponent(
    conn: &mut PgConnection,
    req: &CreateGameRequest,
) -> AppResult<Option<Uuid>> {
    let id: Option<Uuid> = if let Some(handle) = &req.opponent_handle {
        let handle = handle.trim().trim_start_matches('@');
        sqlx::query_scalar(
            "SELECT id FROM users WHERE lower(handle) = lower($1) AND deleted_at IS NULL",
        )
        .bind(handle)
        .fetch_optional(&mut *conn)
        .await?
    } else if let Some(id) = &req.opponent_id {
        let id = Uuid::parse_str(id).map_err(|_| AppError::bad_request("invalid opponent_id"))?;
        sqlx::query_scalar("SELECT id FROM users WHERE id = $1 AND deleted_at IS NULL")
            .bind(id)
            .fetch_optional(&mut *conn)
            .await?
    } else {
        return Ok(None);
    };
    id.map(Some).ok_or_else(AppError::not_found)
}

/// `POST /v1/games`: invite a player, or create an invite link.
pub async fn create(db: &PgPool, me: Uuid, req: CreateGameRequest) -> AppResult<OnlineGame> {
    let variant_id = req.variant_id.as_deref().unwrap_or(DEFAULT_VARIANT);
    let v = variant(variant_id).ok_or_else(|| AppError::bad_request("unknown variant"))?;
    let mut tx = db.begin().await?;
    let opponent = find_opponent(&mut tx, &req).await?;
    if opponent == Some(me) {
        return Err(AppError::bad_request("you can't invite yourself"));
    }
    let open: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM online_games WHERE south_user = $1 AND status = 'invited'",
    )
    .bind(me)
    .fetch_one(&mut *tx)
    .await?;
    if open >= MAX_OPEN_INVITES {
        return Err(AppError::conflict("too many open invitations"));
    }
    let id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO online_games (id, variant_id, variant_version, first_player, south_user,
                                   invited_user, invite_code, rated)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
    )
    .bind(id)
    .bind(&v.id)
    .bind(i32::from(v.version))
    .bind(enum_str(&coin_toss()))
    .bind(me)
    .bind(opponent)
    .bind(opponent.is_none().then(invite_code))
    .bind(req.rated.unwrap_or(true))
    .execute(&mut *tx)
    .await?;
    let row = reload(&mut tx, id).await?;
    let game = view(&mut tx, &row, me).await?;
    tx.commit().await?;
    Ok(game)
}

/// `GET /v1/games`: open and active games, plus the last `RECENT_DAYS` of the rest.
pub async fn list(db: &PgPool, me: Uuid) -> AppResult<GameList> {
    let mut tx = db.begin().await?;
    let now = OffsetDateTime::now_utc();
    // Settle what time has decided first, in a fixed order (no deadlocks between players).
    let due: Vec<Uuid> = sqlx::query_scalar(
        "SELECT id FROM online_games
          WHERE (south_user = $1 OR north_user = $1 OR invited_user = $1)
            AND ((status = 'active' AND turn_deadline < $2)
                 OR (status = 'invited' AND created_at < $2 - make_interval(days => $3)))
          ORDER BY id",
    )
    .bind(me)
    .bind(now)
    .bind(INVITE_TTL_DAYS as i32)
    .fetch_all(&mut *tx)
    .await?;
    for id in due {
        if let Some(mut row) = lock(&mut tx, id).await? {
            settle(&mut tx, &mut row, now).await?;
        }
    }
    let rows: Vec<Row> = sqlx::query_as(concat!(
        "SELECT ",
        columns!(),
        " FROM online_games
          WHERE (south_user = $1 OR north_user = $1 OR (invited_user = $1 AND status = 'invited'))
            AND (status IN ('invited', 'active') OR updated_at > $2 - make_interval(days => $3))
          ORDER BY updated_at DESC
          LIMIT $4"
    ))
    .bind(me)
    .bind(now)
    .bind(RECENT_DAYS as i32)
    .bind(LIST_LIMIT)
    .fetch_all(&mut *tx)
    .await?;
    let mut games = Vec::with_capacity(rows.len());
    for row in &rows {
        games.push(view(&mut tx, row, me).await?);
    }
    tx.commit().await?;
    Ok(GameList { games })
}

/// `GET /v1/games/{id}`.
pub async fn get(db: &PgPool, me: Uuid, id: Uuid) -> AppResult<OnlineGame> {
    let mut tx = db.begin().await?;
    let (row, _) = lock_mine(&mut tx, id, me).await?;
    let game = view(&mut tx, &row, me).await?;
    tx.commit().await?;
    Ok(game)
}

/// Start an accepted game: `north` takes the North seat, the first turn's clock starts.
async fn start(conn: &mut PgConnection, id: Uuid, north: Uuid) -> AppResult<()> {
    sqlx::query(
        "UPDATE online_games
            SET north_user = $2, status = 'active', invite_code = NULL, started_at = now(),
                turn_deadline = now() + make_interval(days => $3), updated_at = now()
          WHERE id = $1",
    )
    .bind(id)
    .bind(north)
    .bind(TURN_DEADLINE_DAYS as i32)
    .execute(&mut *conn)
    .await?;
    Ok(())
}

/// `POST /v1/games/{id}/accept`: the invited player accepts.
pub async fn accept(db: &PgPool, me: Uuid, id: Uuid) -> AppResult<OnlineGame> {
    let mut tx = db.begin().await?;
    let (row, _) = lock_mine(&mut tx, id, me).await?;
    if row.status != "invited" || row.invited_user != Some(me) {
        return Err(AppError::conflict("this game isn't waiting for you"));
    }
    start(&mut tx, id, me).await?;
    let row = reload(&mut tx, id).await?;
    let game = view(&mut tx, &row, me).await?;
    tx.commit().await?;
    Ok(game)
}

/// `POST /v1/games/join`: accept an invite link.
pub async fn join(db: &PgPool, me: Uuid, code: &str) -> AppResult<OnlineGame> {
    let mut tx = db.begin().await?;
    let id: Uuid = sqlx::query_scalar("SELECT id FROM online_games WHERE invite_code = $1")
        .bind(code)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(AppError::not_found)?;
    let mut row = lock(&mut tx, id).await?.ok_or_else(AppError::not_found)?;
    settle(&mut tx, &mut row, OffsetDateTime::now_utc()).await?;
    if row.status != "invited" || row.invited_user.is_some() {
        return Err(AppError::conflict("this invitation is no longer open"));
    }
    if row.south_user == me {
        return Err(AppError::bad_request("that's your own invitation"));
    }
    start(&mut tx, id, me).await?;
    let row = reload(&mut tx, id).await?;
    let game = view(&mut tx, &row, me).await?;
    tx.commit().await?;
    Ok(game)
}

/// `POST /v1/games/{id}/decline` (the invited player) or `/cancel` (the inviter).
pub async fn close_invitation(
    db: &PgPool,
    me: Uuid,
    id: Uuid,
    by_inviter: bool,
) -> AppResult<OnlineGame> {
    let mut tx = db.begin().await?;
    let (row, _) = lock_mine(&mut tx, id, me).await?;
    let allowed = row.status == "invited"
        && if by_inviter {
            row.south_user == me
        } else {
            row.invited_user == Some(me)
        };
    if !allowed {
        return Err(AppError::conflict("this invitation can't be closed now"));
    }
    sqlx::query(
        "UPDATE online_games SET status = $2, invite_code = NULL, updated_at = now() WHERE id = $1",
    )
    .bind(id)
    .bind(if by_inviter { "cancelled" } else { "declined" })
    .execute(&mut *tx)
    .await?;
    let row = reload(&mut tx, id).await?;
    let game = view(&mut tx, &row, me).await?;
    tx.commit().await?;
    Ok(game)
}

/// `POST /v1/games/{id}/moves`.
pub async fn play(db: &PgPool, me: Uuid, id: Uuid, req: MoveRequest) -> AppResult<OnlineGame> {
    let mut tx = db.begin().await?;
    let (mut row, seat) = lock_mine(&mut tx, id, me).await?;
    let ply = req.ply as usize;
    // A retry of a move that already went through: answer with the game as it is.
    if ply < row.moves.len() {
        if row.moves[ply] == i32::from(req.pit) {
            let game = view(&mut tx, &row, me).await?;
            tx.commit().await?;
            return Ok(game);
        }
        return Err(AppError::conflict("that move is out of date"));
    }
    if row.status != "active" {
        return Err(AppError::conflict("this game isn't being played"));
    }
    if ply > row.moves.len() {
        return Err(AppError::conflict("that move is out of date"));
    }
    let v = row.variant()?;
    let pos = row.position()?;
    if pos.to_move != player(seat) {
        return Err(AppError::conflict("it isn't your turn"));
    }
    let result = apply_move(v, &pos, req.pit).map_err(|_| AppError::bad_request("illegal move"))?;
    sqlx::query(
        "UPDATE online_games
            SET moves = array_append(moves, $2),
                turn_deadline = now() + make_interval(days => $3), updated_at = now()
          WHERE id = $1",
    )
    .bind(id)
    .bind(i32::from(req.pit))
    .bind(TURN_DEADLINE_DAYS as i32)
    .execute(&mut *tx)
    .await?;
    row = reload(&mut tx, id).await?;
    if result.state.status != Status::Playing {
        let outcome = match result.state.status {
            Status::Won(Player::South) => GameResult::SouthWins,
            Status::Won(Player::North) => GameResult::NorthWins,
            _ => GameResult::Draw,
        };
        let reason = result
            .events
            .iter()
            .find_map(|e| match e {
                MoveEvent::GameOver { reason, .. } => Some(RecordEndReason::from(*reason)),
                _ => None,
            })
            .ok_or_else(AppError::internal)?;
        finish(
            &mut tx,
            &mut row,
            outcome,
            reason,
            result.state.stores,
            OffsetDateTime::now_utc(),
            true,
        )
        .await?;
    }
    let game = view(&mut tx, &row, me).await?;
    tx.commit().await?;
    Ok(game)
}

/// `POST /v1/games/{id}/resign`.
pub async fn resign(db: &PgPool, me: Uuid, id: Uuid) -> AppResult<OnlineGame> {
    let mut tx = db.begin().await?;
    let (mut row, seat) = lock_mine(&mut tx, id, me).await?;
    if row.status != "active" {
        return Err(AppError::conflict("this game isn't being played"));
    }
    let stores = row.position()?.stores;
    finish(
        &mut tx,
        &mut row,
        winner(other(seat)),
        RecordEndReason::Resigned,
        stores,
        OffsetDateTime::now_utc(),
        true,
    )
    .await?;
    let game = view(&mut tx, &row, me).await?;
    tx.commit().await?;
    Ok(game)
}

/// `GET /v1/opponents`: people `me` has played against, most recent first.
pub async fn opponents(db: &PgPool, me: Uuid) -> AppResult<OpponentList> {
    let list = opponents::recent_opponents(db, me, LIST_LIMIT).await?;
    let mut conn = db.acquire().await?;
    let mut out = Vec::with_capacity(list.len());
    for o in list {
        out.push(OpponentSummary {
            player: summary(&mut conn, o.opponent_id).await?,
            games_played: o.games_played.max(0) as u32,
            last_played_at: rfc3339(o.last_played_at)?,
        });
    }
    Ok(OpponentList { opponents: out })
}

/// Account deletion: withdraw or decline open invitations, and resign every active game (the
/// opponent wins and keeps the game in their history). Runs inside the deletion's
/// transaction, after the user row is anonymized, so opponents see "Deleted player".
pub async fn end_for_deleted_account(conn: &mut PgConnection, me: Uuid) -> AppResult<()> {
    sqlx::query(
        "UPDATE online_games SET status = 'cancelled', invite_code = NULL, updated_at = now()
          WHERE status = 'invited' AND (south_user = $1 OR invited_user = $1)",
    )
    .bind(me)
    .execute(&mut *conn)
    .await?;
    let active: Vec<Uuid> = sqlx::query_scalar(
        "SELECT id FROM online_games
          WHERE status = 'active' AND (south_user = $1 OR north_user = $1)
          ORDER BY id",
    )
    .bind(me)
    .fetch_all(&mut *conn)
    .await?;
    for id in active {
        let Some(mut row) = lock(conn, id).await? else {
            continue;
        };
        let Some(seat) = row.seat_of(me) else {
            continue;
        };
        let stores = row.position()?.stores;
        finish(
            conn,
            &mut row,
            winner(other(seat)),
            RecordEndReason::Resigned,
            stores,
            OffsetDateTime::now_utc(),
            // An account being deleted doesn't hand out rating points.
            false,
        )
        .await?;
    }
    Ok(())
}
