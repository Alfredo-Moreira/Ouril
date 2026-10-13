//! Who has played against whom (`user_opponents`): games played, first and last played at.
//! The foundation for "people you've played against" and friends (roadmap Phase 4). Nothing
//! records games between players yet; games against the computer never appear here.

use sqlx::{PgExecutor, PgPool};
use time::OffsetDateTime;
use uuid::Uuid;

/// One opponent of a player.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct Opponent {
    pub opponent_id: Uuid,
    pub games_played: i32,
    pub first_played_at: OffsetDateTime,
    pub last_played_at: OffsetDateTime,
}

/// Record one finished game between `a` and `b`, ended at `played_at`, for both players. Call
/// it exactly once per game (it isn't idempotent: a second call counts the game twice). Out of
/// order calls are fine: first and last played at only ever widen.
pub async fn record_game_between<'e>(
    db: impl PgExecutor<'e>,
    a: Uuid,
    b: Uuid,
    played_at: OffsetDateTime,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO user_opponents (user_id, opponent_id, first_played_at, last_played_at)
         VALUES ($1, $2, $3, $3), ($2, $1, $3, $3)
         ON CONFLICT (user_id, opponent_id) DO UPDATE SET
             games_played    = user_opponents.games_played + 1,
             first_played_at = LEAST(user_opponents.first_played_at, EXCLUDED.first_played_at),
             last_played_at  = GREATEST(user_opponents.last_played_at, EXCLUDED.last_played_at)",
    )
    .bind(a)
    .bind(b)
    .bind(played_at)
    .execute(db)
    .await?;
    Ok(())
}

/// A player's opponents, most recently played first.
pub async fn recent_opponents(
    db: &PgPool,
    user_id: Uuid,
    limit: i64,
) -> Result<Vec<Opponent>, sqlx::Error> {
    sqlx::query_as(
        "SELECT opponent_id, games_played, first_played_at, last_played_at
           FROM user_opponents
          WHERE user_id = $1
          ORDER BY last_played_at DESC, opponent_id
          LIMIT $2",
    )
    .bind(user_id)
    .bind(limit)
    .fetch_all(db)
    .await
}
