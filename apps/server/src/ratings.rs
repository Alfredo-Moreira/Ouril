//! Ratings and leaderboards (ADR 0034).
//!
//! - A rated game against another player updates both players' rating in the `overall` pool
//!   and in the game's own pool (`<variant_id>:<mode>`), in the transaction that finishes it.
//!   The maths is `ouril-rating` (Glicko-2 with fractional rating periods).
//! - A game counts only if both players made a move, and only the first
//!   `RATED_GAMES_PER_PAIR_PER_DAY` rated games between the same two players in 24 hours do.
//! - Leaderboards are queries on `ratings`: players with enough recent rated games whose
//!   deviation, grown for the time since their last rated game, is low enough to rank.

use ouril_engine::{variant, GameResult};
use ouril_protocol::{
    Leaderboard, LeaderboardEntry, LeaderboardScope, MyRatings, PlayerSummary, PoolRating,
    RatingChange, LEADERBOARD_ACTIVE_DAYS, LEADERBOARD_MIN_GAMES, LEADERBOARD_SIZE,
    RATED_GAMES_PER_PAIR_PER_DAY, RATING_MODES, RATING_POOL_OVERALL,
};
use ouril_rating::{self as glicko, Config, Rating, Score};
use sqlx::{PgConnection, PgPool};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::error::{AppError, AppResult};

/// The rating system's constants (ADR 0034).
pub fn config() -> Config {
    Config::default()
}

/// The pool of a variant and mode: `cv.standard:async`.
pub fn pool_for(variant_id: &str, mode: &str) -> String {
    format!("{variant_id}:{mode}")
}

/// `overall`, or `<known variant>:<mode>`.
pub fn is_valid_pool(pool: &str) -> bool {
    if pool == RATING_POOL_OVERALL {
        return true;
    }
    match pool.split_once(':') {
        Some((v, mode)) => variant(v).is_some() && RATING_MODES.contains(&mode),
        None => false,
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct Row {
    user_id: Uuid,
    pool: String,
    rating: f64,
    deviation: f64,
    volatility: f64,
    games: i32,
    wins: i32,
    draws: i32,
    losses: i32,
    peak: f64,
    last_rated_at: Option<OffsetDateTime>,
}

impl Row {
    fn glicko(&self) -> Rating {
        Rating {
            rating: self.rating,
            deviation: self.deviation,
            volatility: self.volatility,
        }
    }

    /// Rating periods since this player's last rated game in the pool.
    fn periods_until(&self, now: OffsetDateTime) -> f64 {
        self.last_rated_at
            .map(|t| glicko::periods((now - t).as_seconds_f64()))
            .unwrap_or(0.0)
    }

    fn view(&self, cfg: &Config, now: OffsetDateTime) -> PoolRating {
        let deviation = glicko::deviation_after(cfg, &self.glicko(), self.periods_until(now));
        PoolRating {
            pool: self.pool.clone(),
            rating: self.rating.round() as i32,
            deviation: deviation.round() as i32,
            provisional: cfg.is_provisional(deviation),
            games: self.games.max(0) as u32,
            wins: self.wins.max(0) as u32,
            draws: self.draws.max(0) as u32,
            losses: self.losses.max(0) as u32,
            peak: self.peak.round() as i32,
        }
    }
}

/// A finished game, as far as ratings care.
pub struct FinishedGame {
    pub id: Uuid,
    pub variant_id: String,
    pub mode: String,
    pub south: Uuid,
    pub north: Uuid,
    pub outcome: GameResult,
    pub plies: usize,
    pub ended: OffsetDateTime,
}

/// Rate a finished game that was created as rated, if it counts. Runs in the transaction that
/// finishes the game (the game row is locked).
pub async fn rate_finished_game(conn: &mut PgConnection, game: &FinishedGame) -> AppResult<()> {
    // Both players must have moved: a game resigned or timed out before that doesn't count.
    if game.plies < 2 {
        return Ok(());
    }
    let recent_between: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM online_games
          WHERE rating_applied AND ended_at > $3 - interval '24 hours'
            AND ((south_user = $1 AND north_user = $2) OR (south_user = $2 AND north_user = $1))",
    )
    .bind(game.south)
    .bind(game.north)
    .bind(game.ended)
    .fetch_one(&mut *conn)
    .await?;
    if recent_between >= RATED_GAMES_PER_PAIR_PER_DAY {
        return Ok(());
    }
    let score_south = match game.outcome {
        GameResult::SouthWins => Score::Win,
        GameResult::NorthWins => Score::Loss,
        GameResult::Draw => Score::Draw,
    };
    let cfg = config();
    for pool in [
        RATING_POOL_OVERALL.to_string(),
        pool_for(&game.variant_id, &game.mode),
    ] {
        let (south, north) = lock_pair(conn, &cfg, &pool, game.south, game.north).await?;
        let (south_after, north_after) = glicko::rate_game(
            &cfg,
            (&south.glicko(), south.periods_until(game.ended)),
            (&north.glicko(), north.periods_until(game.ended)),
            score_south,
        );
        save(conn, game, &south, south_after, score_south).await?;
        save(conn, game, &north, north_after, score_south.flip()).await?;
    }
    sqlx::query("UPDATE online_games SET rating_applied = true WHERE id = $1")
        .bind(game.id)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

/// Both players' rows in `pool`, created at the starting rating if missing, locked in a fixed
/// order (by user ID) so two games finishing at once can't deadlock.
async fn lock_pair(
    conn: &mut PgConnection,
    cfg: &Config,
    pool: &str,
    a: Uuid,
    b: Uuid,
) -> AppResult<(Row, Row)> {
    sqlx::query(
        "INSERT INTO ratings (user_id, pool, rating, deviation, volatility, peak)
         SELECT u, $2, $3, $4, $5, $3 FROM unnest($1::uuid[]) AS u
         ON CONFLICT (user_id, pool) DO NOTHING",
    )
    .bind(vec![a, b])
    .bind(pool)
    .bind(cfg.initial.rating)
    .bind(cfg.initial.deviation)
    .bind(cfg.initial.volatility)
    .execute(&mut *conn)
    .await?;
    let rows: Vec<Row> = sqlx::query_as(
        "SELECT user_id, pool, rating, deviation, volatility, games, wins, draws, losses, peak,
                last_rated_at
           FROM ratings WHERE pool = $1 AND user_id = ANY($2) ORDER BY user_id FOR UPDATE",
    )
    .bind(pool)
    .bind(vec![a, b])
    .fetch_all(&mut *conn)
    .await?;
    let find = |id: Uuid| {
        rows.iter()
            .find(|r| r.user_id == id)
            .cloned()
            .ok_or_else(AppError::internal)
    };
    Ok((find(a)?, find(b)?))
}

async fn save(
    conn: &mut PgConnection,
    game: &FinishedGame,
    before: &Row,
    after: Rating,
    score: Score,
) -> AppResult<()> {
    let (w, d, l) = match score {
        Score::Win => (1, 0, 0),
        Score::Draw => (0, 1, 0),
        Score::Loss => (0, 0, 1),
    };
    sqlx::query(
        "UPDATE ratings
            SET rating = $3, deviation = $4, volatility = $5, games = games + 1,
                wins = wins + $6, draws = draws + $7, losses = losses + $8,
                peak = greatest(peak, $3), last_rated_at = $9
          WHERE user_id = $1 AND pool = $2",
    )
    .bind(before.user_id)
    .bind(&before.pool)
    .bind(after.rating)
    .bind(after.deviation)
    .bind(after.volatility)
    .bind(w)
    .bind(d)
    .bind(l)
    .bind(game.ended)
    .execute(&mut *conn)
    .await?;
    sqlx::query(
        "INSERT INTO rating_changes (game_id, user_id, pool, rating_before, rating_after,
                                     deviation_before, deviation_after, at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
    )
    .bind(game.id)
    .bind(before.user_id)
    .bind(&before.pool)
    .bind(before.rating)
    .bind(after.rating)
    .bind(before.deviation)
    .bind(after.deviation)
    .bind(game.ended)
    .execute(&mut *conn)
    .await?;
    Ok(())
}

/// How `user`'s ratings changed in `game`: the overall pool first.
pub async fn changes_for(
    conn: &mut PgConnection,
    game: Uuid,
    user: Uuid,
) -> AppResult<Vec<RatingChange>> {
    let rows: Vec<(String, f64, f64)> = sqlx::query_as(
        "SELECT pool, rating_before, rating_after FROM rating_changes
          WHERE game_id = $1 AND user_id = $2
          ORDER BY pool <> $3, pool",
    )
    .bind(game)
    .bind(user)
    .bind(RATING_POOL_OVERALL)
    .fetch_all(&mut *conn)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(pool, before, after)| RatingChange {
            pool,
            before: before.round() as i32,
            after: after.round() as i32,
        })
        .collect())
}

/// `GET /v1/ratings/me`: the overall pool first, then the rest by name.
pub async fn mine(db: &PgPool, me: Uuid) -> AppResult<MyRatings> {
    let rows: Vec<Row> = sqlx::query_as(
        "SELECT user_id, pool, rating, deviation, volatility, games, wins, draws, losses, peak,
                last_rated_at
           FROM ratings WHERE user_id = $1 AND games > 0
          ORDER BY pool <> $2, pool",
    )
    .bind(me)
    .bind(RATING_POOL_OVERALL)
    .fetch_all(db)
    .await?;
    let cfg = config();
    let now = OffsetDateTime::now_utc();
    Ok(MyRatings {
        ratings: rows.iter().map(|r| r.view(&cfg, now)).collect(),
    })
}

#[derive(sqlx::FromRow)]
struct BoardRow {
    user_id: Uuid,
    display_name: String,
    handle: Option<String>,
    country: Option<String>,
    rating: f64,
    games: i32,
    rank: i64,
}

/// `GET /v1/leaderboard`. `country` is already validated (lowercase ISO 3166-1 alpha-2) for a
/// country board.
pub async fn leaderboard(
    db: &PgPool,
    me: Uuid,
    pool: &str,
    scope: LeaderboardScope,
    country: Option<String>,
) -> AppResult<Leaderboard> {
    let cfg = config();
    // The deviation grown for time (ouril-rating `deviation_after`, computed in SQL so ranks
    // can be counted in the database; `deviation_grows_like_ouril_rating` keeps them equal).
    let rows: Vec<BoardRow> = sqlx::query_as(
        "WITH eligible AS (
            SELECT r.user_id, u.display_name, u.handle, u.country, r.rating, r.games,
                   row_number() OVER (ORDER BY r.rating DESC, r.user_id) AS rank
              FROM ratings r JOIN users u ON u.id = r.user_id
             WHERE r.pool = $1
               AND u.deleted_at IS NULL
               AND r.games >= $2
               AND r.last_rated_at > $3 - make_interval(days => $4)
               AND ($5::text IS NULL OR u.country = $5)
               AND sqrt(power(r.deviation / 173.7178, 2)
                        + extract(epoch FROM ($3 - r.last_rated_at)) / 86400.0
                          * power(r.volatility, 2)) * 173.7178 <= $6
         )
         SELECT * FROM eligible WHERE rank <= $7 OR user_id = $8 ORDER BY rank",
    )
    .bind(pool)
    .bind(LEADERBOARD_MIN_GAMES as i32)
    .bind(OffsetDateTime::now_utc())
    .bind(LEADERBOARD_ACTIVE_DAYS as i32)
    .bind(
        country
            .as_deref()
            .filter(|_| scope == LeaderboardScope::Country),
    )
    .bind(cfg.provisional_deviation)
    .bind(i64::from(LEADERBOARD_SIZE))
    .bind(me)
    .fetch_all(db)
    .await?;
    let entry = |r: &BoardRow| LeaderboardEntry {
        rank: r.rank.max(0) as u32,
        player: PlayerSummary {
            user_id: r.user_id.to_string(),
            display_name: r.display_name.clone(),
            handle: r.handle.clone(),
        },
        country: r.country.clone(),
        rating: r.rating.round() as i32,
        games: r.games.max(0) as u32,
    };
    Ok(Leaderboard {
        pool: pool.to_string(),
        scope,
        country: country.filter(|_| scope == LeaderboardScope::Country),
        entries: rows
            .iter()
            .filter(|r| r.rank <= i64::from(LEADERBOARD_SIZE))
            .map(entry)
            .collect(),
        you: rows.iter().find(|r| r.user_id == me).map(entry),
    })
}

/// A player's country, for a country board without an explicit one.
pub async fn country_of(db: &PgPool, me: Uuid) -> AppResult<Option<String>> {
    Ok(
        sqlx::query_scalar("SELECT country FROM users WHERE id = $1")
            .bind(me)
            .fetch_optional(db)
            .await?
            .flatten(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pools() {
        assert!(is_valid_pool("overall"));
        assert!(is_valid_pool("cv.standard:async"));
        assert!(is_valid_pool("cv.across:live"));
        assert!(!is_valid_pool("cv.standard"));
        assert!(!is_valid_pool("cv.standard:blitz"));
        assert!(!is_valid_pool("xx.nope:async"));
        assert!(!is_valid_pool(""));
        assert_eq!(pool_for("cv.standard", "async"), "cv.standard:async");
    }
}
