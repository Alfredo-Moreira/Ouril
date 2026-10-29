//! Scheduled jobs (ADR 0032): housekeeping run by the `ouril-jobs` container on the homelab,
//! which calls `POST /v1/internal/jobs/<name>` with `Authorization: Bearer <JOBS_TOKEN>`.
//!
//! Every job is idempotent and safe to run late, twice, or not for a while: nothing depends on
//! it running on time (games settle on access anyway). Each run is recorded in `job_runs`.

use sqlx::PgPool;
use time::OffsetDateTime;

use crate::{
    error::{AppError, AppResult},
    ratings,
};

/// Snapshots older than this are deleted (best ranks are kept).
const SNAPSHOT_KEEP_DAYS: i32 = 120;
/// Seeks with no heartbeat for this long are deleted (they stopped counting after 30 s).
const SEEK_KEEP_MINUTES: i32 = 60;

/// The jobs this server knows.
pub const JOBS: [&str; 2] = ["leaderboard-snapshot", "prune"];

/// Run job `name`; returns how many rows it wrote or deleted.
pub async fn run(db: &PgPool, name: &str) -> AppResult<u64> {
    let result = match name {
        "leaderboard-snapshot" => ratings::snapshot_boards(db).await,
        "prune" => prune(db).await,
        _ => return Err(AppError::not_found()),
    };
    let (rows, error) = match &result {
        Ok(n) => (*n as i64, None),
        Err(e) => (0, Some(format!("{e:?}"))),
    };
    sqlx::query(
        "INSERT INTO job_runs (name, last_finished_at, last_rows, last_error)
         VALUES ($1, $2, $3, $4)
         ON CONFLICT (name) DO UPDATE SET last_finished_at = EXCLUDED.last_finished_at,
             last_rows = EXCLUDED.last_rows, last_error = EXCLUDED.last_error",
    )
    .bind(name)
    .bind(OffsetDateTime::now_utc())
    .bind(rows)
    .bind(error)
    .execute(db)
    .await?;
    result
}

/// `prune`: old snapshots and abandoned seeks.
async fn prune(db: &PgPool) -> AppResult<u64> {
    let snapshots = sqlx::query(
        "DELETE FROM leaderboard_snapshots
          WHERE taken_on < (now() AT TIME ZONE 'UTC')::date - $1",
    )
    .bind(SNAPSHOT_KEEP_DAYS)
    .execute(db)
    .await?
    .rows_affected();
    let seeks = sqlx::query(
        "DELETE FROM seeks
          WHERE game_id IS NULL AND heartbeat_at < now() - make_interval(mins => $1)",
    )
    .bind(SEEK_KEEP_MINUTES)
    .execute(db)
    .await?
    .rows_affected();
    Ok(snapshots + seeks)
}
