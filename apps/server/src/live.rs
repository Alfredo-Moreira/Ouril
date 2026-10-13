//! Matchmaking for live games (ADR 0033): a `seeks` table, no queue server.
//!
//! - `POST /v1/live/seek` records (or refreshes) the player's seek and, in the same
//!   transaction, looks for a compatible one: same variant, clock and rated choice, a fresh
//!   heartbeat, and a rating within a window that widens the longer the other player has
//!   waited (anyone after `WINDOW_OPEN_SECS`). Candidates are locked with `SKIP LOCKED`, so
//!   two seekers can't take the same opponent.
//! - Clients repeat the request as a heartbeat while waiting. A seek without one for
//!   `SEEK_STALE_SECS` is ignored. When two seeks pair up, both rows point at the new game
//!   until the players pick it up.

use ouril_engine::variant;
use ouril_protocol::{SeekRequest, SeekState, SeekStatus, SEEK_STALE_SECS};
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    online::{self, live_time_control, NewGame, DEFAULT_VARIANT},
    ratings,
};

/// The rating window starts at ± this...
const WINDOW_START: f64 = 150.0;
/// ...grows by this every `WINDOW_STEP_SECS` the other player has waited...
const WINDOW_GROWTH: f64 = 50.0;
const WINDOW_STEP_SECS: f64 = 5.0;
/// ...and is open to anyone after this.
const WINDOW_OPEN_SECS: f64 = 30.0;

/// A pairing: the new game and the player who was already waiting (to notify).
pub struct Matched {
    pub game: Uuid,
    pub waiting: Uuid,
}

/// `POST /v1/live/seek`.
pub async fn seek(
    db: &PgPool,
    me: Uuid,
    req: SeekRequest,
) -> AppResult<(SeekStatus, Option<Matched>)> {
    let tc = live_time_control(req.time_control)?;
    let variant_id = req.variant_id.as_deref().unwrap_or(DEFAULT_VARIANT);
    let v = variant(variant_id).ok_or_else(|| AppError::bad_request("unknown variant"))?;
    let rated = req.rated.unwrap_or(true);
    let initial_ms = i64::from(tc.initial_secs) * 1000;
    let increment_ms = i64::from(tc.increment_secs) * 1000;

    let mut tx = db.begin().await?;
    let rating = live_rating(&mut tx, me, &v.id).await?;
    // Refresh the same seek (keeping how long it has waited, or its match), or start anew.
    let game_id: Option<Uuid> = sqlx::query_scalar(
        "INSERT INTO seeks (user_id, variant_id, initial_ms, increment_ms, rated, rating)
         VALUES ($1, $2, $3, $4, $5, $6)
         ON CONFLICT (user_id) DO UPDATE SET
             heartbeat_at = now(),
             rating = EXCLUDED.rating,
             created_at = CASE
                 WHEN (seeks.variant_id, seeks.initial_ms, seeks.increment_ms, seeks.rated)
                    = (EXCLUDED.variant_id, EXCLUDED.initial_ms, EXCLUDED.increment_ms,
                       EXCLUDED.rated)
                 THEN seeks.created_at ELSE now() END,
             game_id = CASE
                 WHEN (seeks.variant_id, seeks.initial_ms, seeks.increment_ms, seeks.rated)
                    = (EXCLUDED.variant_id, EXCLUDED.initial_ms, EXCLUDED.increment_ms,
                       EXCLUDED.rated)
                 THEN seeks.game_id ELSE NULL END,
             variant_id = EXCLUDED.variant_id,
             initial_ms = EXCLUDED.initial_ms,
             increment_ms = EXCLUDED.increment_ms,
             rated = EXCLUDED.rated
         RETURNING game_id",
    )
    .bind(me)
    .bind(&v.id)
    .bind(initial_ms)
    .bind(increment_ms)
    .bind(rated)
    .bind(rating)
    .fetch_one(&mut *tx)
    .await?;
    if let Some(game) = game_id {
        let seeking = seeking(&mut tx).await?;
        tx.commit().await?;
        return Ok((matched_status(game, seeking), None));
    }

    let other: Option<Uuid> = sqlx::query_scalar(
        "SELECT user_id FROM seeks
          WHERE user_id <> $1 AND game_id IS NULL
            AND variant_id = $2 AND initial_ms = $3 AND increment_ms = $4 AND rated = $5
            AND heartbeat_at > now() - make_interval(secs => $6)
            AND (extract(epoch FROM now() - created_at) >= $9
                 OR abs(rating - $7) <= $8 + $10 * floor(extract(epoch FROM now() - created_at) / $11))
          ORDER BY created_at
          LIMIT 1
          FOR UPDATE SKIP LOCKED",
    )
    .bind(me)
    .bind(&v.id)
    .bind(initial_ms)
    .bind(increment_ms)
    .bind(rated)
    .bind(SEEK_STALE_SECS as f64)
    .bind(rating)
    .bind(WINDOW_START)
    .bind(WINDOW_OPEN_SECS)
    .bind(WINDOW_GROWTH)
    .bind(WINDOW_STEP_SECS)
    .fetch_optional(&mut *tx)
    .await?;

    let Some(waiting) = other else {
        let seeking = seeking(&mut tx).await?;
        tx.commit().await?;
        return Ok((
            SeekStatus {
                state: SeekState::Waiting,
                game_id: None,
                players_seeking: seeking,
            },
            None,
        ));
    };
    // The player who waited sits South (like an inviter); who moves first is a coin toss.
    let game = online::insert_game(
        &mut tx,
        NewGame {
            variant: v,
            south: waiting,
            invited: Some(me),
            invite_code: None,
            rated,
            time_control: Some(tc),
        },
    )
    .await?;
    online::start(&mut tx, game, me).await?;
    sqlx::query("UPDATE seeks SET game_id = $1 WHERE user_id = ANY($2)")
        .bind(game)
        .bind(vec![me, waiting])
        .execute(&mut *tx)
        .await?;
    let seeking = seeking(&mut tx).await?;
    tx.commit().await?;
    Ok((
        matched_status(game, seeking),
        Some(Matched { game, waiting }),
    ))
}

/// `GET /v1/live/seek`.
pub async fn status(db: &PgPool, me: Uuid) -> AppResult<SeekStatus> {
    let mut conn = db.acquire().await?;
    let row: Option<Option<Uuid>> = sqlx::query_scalar(
        "SELECT game_id FROM seeks
          WHERE user_id = $1
            AND (game_id IS NOT NULL OR heartbeat_at > now() - make_interval(secs => $2))",
    )
    .bind(me)
    .bind(SEEK_STALE_SECS as f64)
    .fetch_optional(&mut *conn)
    .await?;
    let seeking = seeking(&mut conn).await?;
    Ok(match row {
        Some(Some(game)) => matched_status(game, seeking),
        Some(None) => SeekStatus {
            state: SeekState::Waiting,
            game_id: None,
            players_seeking: seeking,
        },
        None => idle(seeking),
    })
}

/// `DELETE /v1/live/seek`: stop looking (or forget a match already picked up).
pub async fn cancel(db: &PgPool, me: Uuid) -> AppResult<SeekStatus> {
    let mut conn = db.acquire().await?;
    sqlx::query("DELETE FROM seeks WHERE user_id = $1")
        .bind(me)
        .execute(&mut *conn)
        .await?;
    Ok(idle(seeking(&mut conn).await?))
}

fn idle(players_seeking: u32) -> SeekStatus {
    SeekStatus {
        state: SeekState::Idle,
        game_id: None,
        players_seeking,
    }
}

fn matched_status(game: Uuid, players_seeking: u32) -> SeekStatus {
    SeekStatus {
        state: SeekState::Matched,
        game_id: Some(game.to_string()),
        players_seeking,
    }
}

/// Players looking for a game right now (fresh, unpaired seeks).
async fn seeking(conn: &mut PgConnection) -> AppResult<u32> {
    let n: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM seeks
          WHERE game_id IS NULL AND heartbeat_at > now() - make_interval(secs => $1)",
    )
    .bind(SEEK_STALE_SECS as f64)
    .fetch_one(&mut *conn)
    .await?;
    Ok(n.max(0) as u32)
}

/// The player's rating in the variant's live pool, or the starting rating.
async fn live_rating(conn: &mut PgConnection, me: Uuid, variant_id: &str) -> AppResult<f64> {
    let pool = ratings::pool_for(variant_id, "live");
    let r: Option<f64> =
        sqlx::query_scalar("SELECT rating FROM ratings WHERE user_id = $1 AND pool = $2")
            .bind(me)
            .bind(pool)
            .fetch_optional(&mut *conn)
            .await?;
    Ok(r.unwrap_or(ratings::config().initial.rating))
}
