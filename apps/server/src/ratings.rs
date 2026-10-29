//! Ratings and leaderboards (ADR 0034).
//!
//! - A rated game against another player updates both players' rating in the `overall` pool
//!   and in the game's own pool (`<variant_id>:<mode>`), in the transaction that finishes it.
//!   The maths is `ouril-rating` (Glicko-2 with fractional rating periods).
//! - A game counts only if both players made a move, and only the first
//!   `RATED_GAMES_PER_PAIR_PER_DAY` rated games between the same two players in 24 hours do.
//! - Leaderboards are queries on `ratings`: players with enough recent rated games whose
//!   deviation, grown for the time since their last rated game, is low enough to rank.

use std::collections::HashMap;

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
    // One game per pair at a time from here on, so two games between the same players
    // finishing at once can't both read the old count below and both be rated. Under READ
    // COMMITTED the count, a new statement, then sees whatever the other game committed.
    let (low, high) = sorted(game.south, game.north);
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
        .bind(format!("ouril-rating-pair:{low}:{high}"))
        .execute(&mut *conn)
        .await?;
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

fn sorted(a: Uuid, b: Uuid) -> (Uuid, Uuid) {
    if a <= b {
        (a, b)
    } else {
        (b, a)
    }
}

/// Both players' rows in `pool`, created at the starting rating if missing, inserted and
/// locked in a fixed order (by user ID) so games finishing at once can't deadlock.
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
    .bind({
        let (low, high) = sorted(a, b);
        vec![low, high]
    })
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
                peak = greatest(peak, $3),
                -- A timeout settled late ends in the past: never move this back.
                last_rated_at = greatest(last_rated_at, $9)
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
    Ok(changes_for_games(conn, &[game], user)
        .await?
        .remove(&game)
        .unwrap_or_default())
}

/// `user`'s rating changes in each of `games`, overall pool first, in one query.
pub async fn changes_for_games(
    conn: &mut PgConnection,
    games: &[Uuid],
    user: Uuid,
) -> AppResult<HashMap<Uuid, Vec<RatingChange>>> {
    let mut out: HashMap<Uuid, Vec<RatingChange>> = HashMap::new();
    if games.is_empty() {
        return Ok(out);
    }
    let rows: Vec<(Uuid, String, f64, f64)> = sqlx::query_as(
        "SELECT game_id, pool, rating_before, rating_after FROM rating_changes
          WHERE game_id = ANY($1) AND user_id = $2
          ORDER BY game_id, pool <> $3, pool",
    )
    .bind(games)
    .bind(user)
    .bind(RATING_POOL_OVERALL)
    .fetch_all(&mut *conn)
    .await?;
    for (game, pool, before, after) in rows {
        out.entry(game).or_default().push(RatingChange {
            pool,
            before: before.round() as i32,
            after: after.round() as i32,
        });
    }
    Ok(out)
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

/// Who may appear on a board, as SQL over `ratings r JOIN users u` (shared by boards and
/// snapshots, so they can't disagree). Parameters: `$1` min games, `$2` now, `$3` active days,
/// `$4` the provisional deviation, `$5` ouril-rating's `SCALE`, `$6` its `PERIOD_SECS`. The
/// deviation is grown for time exactly like ouril-rating's `deviation_after`
/// (`deviation_grows_like_ouril_rating` keeps them equal).
macro_rules! eligible {
    () => {
        "u.deleted_at IS NULL
         AND r.games >= $1
         AND r.last_rated_at > $2 - make_interval(days => $3)
         AND sqrt(power(r.deviation / $5, 2)
                  + extract(epoch FROM ($2 - r.last_rated_at)) / $6
                    * power(r.volatility, 2)) * $5 <= $4"
    };
}

/// Bind the six parameters [`eligible!`] uses, in order.
fn bind_eligibility<'q, O>(
    q: sqlx::query::QueryAs<'q, sqlx::Postgres, O, sqlx::postgres::PgArguments>,
    now: OffsetDateTime,
) -> sqlx::query::QueryAs<'q, sqlx::Postgres, O, sqlx::postgres::PgArguments> {
    q.bind(LEADERBOARD_MIN_GAMES as i32)
        .bind(now)
        .bind(LEADERBOARD_ACTIVE_DAYS as i32)
        .bind(config().provisional_deviation)
        .bind(glicko::SCALE)
        .bind(glicko::PERIOD_SECS)
}

/// A board's key in snapshots: `world`, or the country code.
fn scope_key(scope: LeaderboardScope, country: Option<&str>) -> String {
    match (scope, country) {
        (LeaderboardScope::Country, Some(c)) => c.to_string(),
        _ => "world".to_string(),
    }
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
/// country board. Movement compares with the latest snapshot before today, when the
/// scheduled job has written one.
pub async fn leaderboard(
    db: &PgPool,
    me: Uuid,
    pool: &str,
    scope: LeaderboardScope,
    country: Option<String>,
) -> AppResult<Leaderboard> {
    let now = OffsetDateTime::now_utc();
    let country = country.filter(|_| scope == LeaderboardScope::Country);
    let rows: Vec<BoardRow> = bind_eligibility(
        sqlx::query_as(concat!(
            "WITH eligible AS (
                SELECT r.user_id, u.display_name, u.handle, u.country, r.rating, r.games,
                       row_number() OVER (ORDER BY r.rating DESC, r.user_id) AS rank
                  FROM ratings r JOIN users u ON u.id = r.user_id
                 WHERE r.pool = $7 AND ($8::text IS NULL OR u.country = $8) AND ",
            eligible!(),
            "
             )
             SELECT * FROM eligible WHERE rank <= $9 OR user_id = $10 ORDER BY rank"
        )),
        now,
    )
    .bind(pool)
    .bind(country.as_deref())
    .bind(i64::from(LEADERBOARD_SIZE))
    .bind(me)
    .fetch_all(db)
    .await?;

    let key = scope_key(scope, country.as_deref());
    let since: Option<time::Date> = sqlx::query_scalar(
        "SELECT max(taken_on) FROM leaderboard_snapshots
          WHERE pool = $1 AND scope = $2 AND taken_on < $3",
    )
    .bind(pool)
    .bind(&key)
    .bind(now.date())
    .fetch_one(db)
    .await?;
    let before: HashMap<Uuid, i32> = match since {
        Some(day) => sqlx::query_as::<_, (Uuid, i32)>(
            "SELECT user_id, rank FROM leaderboard_snapshots
              WHERE taken_on = $1 AND pool = $2 AND scope = $3 AND user_id = ANY($4)",
        )
        .bind(day)
        .bind(pool)
        .bind(&key)
        .bind(rows.iter().map(|r| r.user_id).collect::<Vec<_>>())
        .fetch_all(db)
        .await?
        .into_iter()
        .collect(),
        None => HashMap::new(),
    };
    let best: Option<i32> = sqlx::query_scalar(
        "SELECT best_rank FROM rank_bests WHERE user_id = $1 AND pool = $2 AND scope = $3",
    )
    .bind(me)
    .bind(pool)
    .bind(&key)
    .fetch_optional(db)
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
        // Places gained since the earlier snapshot (negative: lost). None: new to the board.
        change: before.get(&r.user_id).map(|&old| old - r.rank as i32),
    };
    Ok(Leaderboard {
        pool: pool.to_string(),
        scope,
        country,
        entries: rows
            .iter()
            .filter(|r| r.rank <= i64::from(LEADERBOARD_SIZE))
            .map(entry)
            .collect(),
        you: rows.iter().find(|r| r.user_id == me).map(entry),
        movement_since: since.map(|d| d.to_string()),
        your_best_rank: best.map(|b| b.max(1) as u32),
    })
}

/// The `leaderboard-snapshot` job: today's ranks on every board (each pool, worldwide and
/// per country) replace today's earlier snapshot, and best ranks improve. Idempotent: running
/// it again the same day just refreshes today. Returns the rows written.
pub async fn snapshot_boards(db: &PgPool) -> AppResult<u64> {
    let now = OffsetDateTime::now_utc();
    let mut tx = db.begin().await?;
    sqlx::query("DELETE FROM leaderboard_snapshots WHERE taken_on = $1")
        .bind(now.date())
        .execute(&mut *tx)
        .await?;
    let written = bind_eligibility(
        sqlx::query_as::<_, (i64,)>(concat!(
            "WITH eligible AS (
                SELECT r.pool, r.user_id, u.country, r.rating
                  FROM ratings r JOIN users u ON u.id = r.user_id
                 WHERE ",
            eligible!(),
            "
             ), ranked AS (
                SELECT pool, 'world' AS scope, user_id, rating,
                       row_number() OVER (PARTITION BY pool ORDER BY rating DESC, user_id) AS rank
                  FROM eligible
                UNION ALL
                SELECT pool, country, user_id, rating,
                       row_number() OVER (PARTITION BY pool, country
                                          ORDER BY rating DESC, user_id)
                  FROM eligible WHERE country IS NOT NULL
             ), inserted AS (
                INSERT INTO leaderboard_snapshots (taken_on, pool, scope, user_id, rank, rating)
                SELECT $7, pool, scope, user_id, rank, rating FROM ranked
                RETURNING 1
             )
             SELECT count(*) FROM inserted"
        )),
        now,
    )
    .bind(now.date())
    .fetch_one(&mut *tx)
    .await?
    .0;
    sqlx::query(
        "INSERT INTO rank_bests (user_id, pool, scope, best_rank, achieved_on)
         SELECT user_id, pool, scope, rank, taken_on FROM leaderboard_snapshots WHERE taken_on = $1
         ON CONFLICT (user_id, pool, scope) DO UPDATE
            SET best_rank = EXCLUDED.best_rank, achieved_on = EXCLUDED.achieved_on
          WHERE EXCLUDED.best_rank < rank_bests.best_rank",
    )
    .bind(now.date())
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(written.max(0) as u64)
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
