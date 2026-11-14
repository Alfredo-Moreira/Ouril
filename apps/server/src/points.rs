//! The seasonal points board (ADR 0034, "later"): points for activity and wins, per calendar
//! month (UTC), so casual players can climb too.
//!
//! - **Online games** score only when they counted for ratings (both players moved, the daily
//!   per-pair limit), so the anti-farming rules carry over: `POINTS_ONLINE` for a win, a draw
//!   and a loss.
//! - **Wins against the computer** are played on the device; the server replays them for legal
//!   moves but can't know they were played fairly, so they score little (`POINTS_AI_WIN` by
//!   level) and at most `POINTS_AI_DAILY_CAP` a day. They count in the season they reach the
//!   server, never in a past one.

use ouril_engine::GameResult;
use ouril_protocol::{
    LeaderboardScope, PlayerSummary, PointsBoard, PointsEntry, LEADERBOARD_SIZE,
    POINTS_AI_DAILY_CAP, POINTS_AI_WIN, POINTS_ONLINE,
};
use sqlx::{PgConnection, PgPool};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::error::{AppError, AppResult};

/// Seasons listed on the board, newest first.
const SEASONS_LISTED: i64 = 12;

/// The season (`YYYY-MM`, UTC) of `t`.
pub fn season_of(t: OffsetDateTime) -> String {
    format!("{:04}-{:02}", t.year(), u8::from(t.month()))
}

/// `YYYY-MM` with a real month.
pub fn is_valid_season(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 7
        && b[4] == b'-'
        && b.iter()
            .enumerate()
            .all(|(i, c)| i == 4 || c.is_ascii_digit())
        && matches!(s[5..].parse::<u8>(), Ok(1..=12))
}

async fn add(
    conn: &mut PgConnection,
    user: Uuid,
    points: i32,
    online_games: i32,
    ai_wins: i32,
) -> AppResult<()> {
    sqlx::query(
        "INSERT INTO season_points (season, user_id, points, online_games, ai_wins)
         VALUES ($1, $2, $3, $4, $5)
         ON CONFLICT (season, user_id) DO UPDATE
            SET points = season_points.points + EXCLUDED.points,
                online_games = season_points.online_games + EXCLUDED.online_games,
                ai_wins = season_points.ai_wins + EXCLUDED.ai_wins,
                updated_at = now()",
    )
    .bind(season_of(OffsetDateTime::now_utc()))
    .bind(user)
    .bind(points)
    .bind(online_games)
    .bind(ai_wins)
    .execute(&mut *conn)
    .await?;
    Ok(())
}

/// An online game that counted for ratings: both players score.
pub async fn award_online(
    conn: &mut PgConnection,
    south: Uuid,
    north: Uuid,
    outcome: GameResult,
) -> AppResult<()> {
    let [win, draw, loss] = POINTS_ONLINE;
    let (s, n) = match outcome {
        GameResult::SouthWins => (win, loss),
        GameResult::NorthWins => (loss, win),
        GameResult::Draw => (draw, draw),
    };
    // A fixed order, like the ratings rows, so two games finishing at once can't deadlock.
    let mut both = [(south, s), (north, n)];
    both.sort_by_key(|(u, _)| *u);
    for (user, points) in both {
        add(conn, user, points, 1, 0).await?;
    }
    Ok(())
}

/// A new win against the computer at `level` (`easy`, `medium`, `hard`): its points, up to
/// what's left of today's cap. Returns the points given.
pub async fn award_ai_win(conn: &mut PgConnection, user: Uuid, level: &str) -> AppResult<i32> {
    let worth = match level {
        "easy" => POINTS_AI_WIN[0],
        "medium" => POINTS_AI_WIN[1],
        "hard" => POINTS_AI_WIN[2],
        _ => return Ok(0),
    };
    let today = OffsetDateTime::now_utc().date();
    sqlx::query(
        "INSERT INTO ai_points_daily (user_id, day) VALUES ($1, $2) ON CONFLICT DO NOTHING",
    )
    .bind(user)
    .bind(today)
    .execute(&mut *conn)
    .await?;
    let so_far: i32 = sqlx::query_scalar(
        "SELECT points FROM ai_points_daily WHERE user_id = $1 AND day = $2 FOR UPDATE",
    )
    .bind(user)
    .bind(today)
    .fetch_one(&mut *conn)
    .await?;
    let given = worth.min(POINTS_AI_DAILY_CAP - so_far).max(0);
    sqlx::query("UPDATE ai_points_daily SET points = points + $3 WHERE user_id = $1 AND day = $2")
        .bind(user)
        .bind(today)
        .bind(given)
        .execute(&mut *conn)
        .await?;
    add(conn, user, given, 0, 1).await?;
    Ok(given)
}

#[derive(sqlx::FromRow)]
struct BoardRow {
    user_id: Uuid,
    display_name: String,
    handle: Option<String>,
    country: Option<String>,
    points: i32,
    online_games: i32,
    ai_wins: i32,
    rank: i64,
}

/// `GET /v1/points`. `season` defaults to the current one; `country` is validated already.
pub async fn board(
    db: &PgPool,
    me: Uuid,
    season: Option<String>,
    scope: LeaderboardScope,
    country: Option<String>,
) -> AppResult<PointsBoard> {
    let current = season_of(OffsetDateTime::now_utc());
    let season = season.unwrap_or_else(|| current.clone());
    if !is_valid_season(&season) {
        return Err(AppError::bad_request("season must be YYYY-MM"));
    }
    let country = country.filter(|_| scope == LeaderboardScope::Country);
    let rows: Vec<BoardRow> = sqlx::query_as(
        "WITH ranked AS (
            SELECT p.user_id, u.display_name, u.handle, u.country, p.points, p.online_games,
                   p.ai_wins,
                   row_number() OVER (ORDER BY p.points DESC, p.updated_at, p.user_id) AS rank
              FROM season_points p JOIN users u ON u.id = p.user_id
             WHERE p.season = $1 AND p.points > 0 AND u.deleted_at IS NULL
               AND ($2::text IS NULL OR u.country = $2)
         )
         SELECT * FROM ranked WHERE rank <= $3 OR user_id = $4 ORDER BY rank",
    )
    .bind(&season)
    .bind(country.as_deref())
    .bind(i64::from(LEADERBOARD_SIZE))
    .bind(me)
    .fetch_all(db)
    .await?;
    let mut seasons: Vec<String> = sqlx::query_scalar(
        "SELECT DISTINCT season FROM season_points WHERE points > 0
          ORDER BY season DESC LIMIT $1",
    )
    .bind(SEASONS_LISTED)
    .fetch_all(db)
    .await?;
    if !seasons.contains(&current) {
        seasons.insert(0, current.clone());
    }
    let entry = |r: &BoardRow| PointsEntry {
        rank: r.rank.max(0) as u32,
        player: PlayerSummary {
            user_id: r.user_id.to_string(),
            display_name: r.display_name.clone(),
            handle: r.handle.clone(),
        },
        country: r.country.clone(),
        points: r.points.max(0) as u32,
        online_games: r.online_games.max(0) as u32,
        ai_wins: r.ai_wins.max(0) as u32,
    };
    Ok(PointsBoard {
        current: season == current,
        season,
        scope,
        country,
        entries: rows
            .iter()
            .filter(|r| r.rank <= i64::from(LEADERBOARD_SIZE))
            .map(entry)
            .collect(),
        you: rows.iter().find(|r| r.user_id == me).map(entry),
        seasons,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seasons_are_calendar_months() {
        let t = time::Date::from_calendar_date(2026, time::Month::October, 31)
            .unwrap()
            .with_hms(23, 59, 0)
            .unwrap()
            .assume_utc();
        assert_eq!(season_of(t), "2026-10");
        assert!(is_valid_season("2026-01"));
        assert!(!is_valid_season("2026-13"));
        assert!(!is_valid_season("2026-1"));
        assert!(!is_valid_season("26-10-01"));
        assert!(!is_valid_season("2026/10"));
    }
}
