//! Player profiles (`GET /v1/players/{id}`, signed-in players): ratings, rating history, badges
//! from best ranks, and your record against them. Deleted accounts have no profile (404).

use std::collections::BTreeMap;

use ouril_protocol::{
    Badge, HeadToHead, PlayerProfile, PlayerSummary, RatingHistory, RatingPoint, BADGE_MAX_RANK,
    PROFILE_HISTORY_DAYS, PROFILE_HISTORY_POINTS, RATING_POOL_OVERALL,
};
use sqlx::PgPool;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    ratings,
};

fn rfc3339(t: OffsetDateTime) -> AppResult<String> {
    t.format(&Rfc3339).map_err(|_| AppError::internal())
}

/// A player by ID, or by `@handle` (any case). Deleted or unknown: 404.
async fn find(db: &PgPool, key: &str) -> AppResult<Uuid> {
    let id: Option<Uuid> = if let Some(handle) = key.strip_prefix('@') {
        sqlx::query_scalar(
            "SELECT id FROM users WHERE lower(handle) = lower($1) AND deleted_at IS NULL",
        )
        .bind(handle)
        .fetch_optional(db)
        .await?
    } else {
        let id = Uuid::parse_str(key).map_err(|_| AppError::not_found())?;
        sqlx::query_scalar("SELECT id FROM users WHERE id = $1 AND deleted_at IS NULL")
            .bind(id)
            .fetch_optional(db)
            .await?
    };
    id.ok_or_else(AppError::not_found)
}

/// `GET /v1/players/{key}`.
pub async fn profile(db: &PgPool, me: Uuid, key: &str) -> AppResult<PlayerProfile> {
    let user = find(db, key).await?;
    let (display_name, handle, country, created_at): (
        String,
        Option<String>,
        Option<String>,
        OffsetDateTime,
    ) = sqlx::query_as("SELECT display_name, handle, country, created_at FROM users WHERE id = $1")
        .bind(user)
        .fetch_one(db)
        .await?;

    // The latest points per pool, oldest first.
    let points: Vec<(String, OffsetDateTime, f64)> = sqlx::query_as(
        "SELECT pool, at, rating_after FROM (
             SELECT pool, at, rating_after,
                    row_number() OVER (PARTITION BY pool ORDER BY at DESC) AS n
               FROM rating_changes
              WHERE user_id = $1 AND at > now() - make_interval(days => $2)
         ) recent
          WHERE n <= $3
          ORDER BY pool <> $4, pool, at",
    )
    .bind(user)
    .bind(PROFILE_HISTORY_DAYS as i32)
    .bind(PROFILE_HISTORY_POINTS)
    .bind(RATING_POOL_OVERALL)
    .fetch_all(db)
    .await?;
    // Keep the overall pool first, then the rest by name.
    let mut grouped: BTreeMap<(bool, String), Vec<RatingPoint>> = BTreeMap::new();
    for (pool, at, rating) in points {
        grouped
            .entry((pool != RATING_POOL_OVERALL, pool))
            .or_default()
            .push(RatingPoint {
                at: rfc3339(at)?,
                rating: rating.round() as i32,
            });
    }
    let history = grouped
        .into_iter()
        .map(|((_, pool), points)| RatingHistory { pool, points })
        .collect();

    let badges: Vec<(String, String, i32, time::Date)> = sqlx::query_as(
        "SELECT pool, scope, best_rank, achieved_on FROM rank_bests
          WHERE user_id = $1 AND best_rank <= $2
          ORDER BY best_rank, scope = 'world' DESC, pool <> $3, pool",
    )
    .bind(user)
    .bind(BADGE_MAX_RANK as i32)
    .bind(RATING_POOL_OVERALL)
    .fetch_all(db)
    .await?;

    let head_to_head = if user == me {
        None
    } else {
        let (wins, draws, losses): (i64, i64, i64) = sqlx::query_as(
            "SELECT
                 count(*) FILTER (WHERE (south_user = $1 AND result_outcome = 'south_wins')
                                     OR (north_user = $1 AND result_outcome = 'north_wins')),
                 count(*) FILTER (WHERE result_outcome = 'draw'),
                 count(*) FILTER (WHERE (south_user = $1 AND result_outcome = 'north_wins')
                                     OR (north_user = $1 AND result_outcome = 'south_wins'))
               FROM online_games
              WHERE status = 'finished'
                AND ((south_user = $1 AND north_user = $2) OR (south_user = $2 AND north_user = $1))",
        )
        .bind(me)
        .bind(user)
        .fetch_one(db)
        .await?;
        Some(HeadToHead {
            wins: wins.max(0) as u32,
            draws: draws.max(0) as u32,
            losses: losses.max(0) as u32,
        })
    };

    Ok(PlayerProfile {
        player: PlayerSummary {
            user_id: user.to_string(),
            display_name,
            handle,
        },
        country,
        member_since: rfc3339(created_at)?,
        you: user == me,
        ratings: ratings::pools_of(db, user).await?,
        history,
        badges: badges
            .into_iter()
            .map(|(pool, scope, rank, on)| Badge {
                pool,
                scope,
                rank: rank.max(1) as u32,
                achieved_on: on.to_string(),
            })
            .collect(),
        head_to_head,
    })
}
