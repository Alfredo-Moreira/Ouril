//! Scheduled jobs (`POST /v1/internal/jobs/*`, ADR 0032): the token, daily leaderboard
//! snapshots with rank movement and best ranks, and pruning.

mod common;

use axum::http::StatusCode;
use common::*;
use ouril_rating::PERIOD_SECS;
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

const TOKEN: &str = "jobs-token-for-tests-0123456789abcdefghij";

fn app_with_jobs(db: PgPool) -> TestApp {
    let mut env = base_env();
    env.insert("JOBS_TOKEN", TOKEN.into());
    TestApp::with_env(db, env)
}

async fn run_job(app: &TestApp, name: &str) -> Value {
    let res = app
        .call(
            Req::post(format!("/v1/internal/jobs/{name}"))
                .header("authorization", format!("Bearer {TOKEN}")),
        )
        .await;
    res.assert_status(StatusCode::OK);
    res.json()
}

/// An eligible `overall` rating, rated a day ago, and a country.
async fn rated(db: &PgPool, user: Uuid, rating: f64, country: &str) {
    sqlx::query(
        "INSERT INTO ratings (user_id, pool, rating, deviation, volatility, games, wins, peak,
                              last_rated_at)
         VALUES ($1, 'overall', $2, 60, 0.06, 12, 12, $2, now() - make_interval(secs => $3))
         ON CONFLICT (user_id, pool) DO UPDATE SET rating = EXCLUDED.rating",
    )
    .bind(user)
    .bind(rating)
    .bind(PERIOD_SECS)
    .execute(db)
    .await
    .unwrap();
    sqlx::query("UPDATE users SET country = $2 WHERE id = $1")
        .bind(user)
        .bind(country)
        .execute(db)
        .await
        .unwrap();
}

#[sqlx::test]
async fn jobs_need_the_token_and_exist_only_when_configured(db: PgPool) {
    let off = TestApp::new(db.clone());
    off.call(
        Req::post("/v1/internal/jobs/prune").header("authorization", format!("Bearer {TOKEN}")),
    )
    .await
    .assert_status(StatusCode::NOT_FOUND);

    let app = app_with_jobs(db);
    for auth in [
        "",
        "Bearer wrong",
        "Bearer jobs-token-for-tests-0123456789abcdefghiX",
    ] {
        app.call(Req::post("/v1/internal/jobs/prune").header("authorization", auth))
            .await
            .assert_status(StatusCode::UNAUTHORIZED);
    }
    // A player's own sign-in doesn't open it either.
    let me = app.sign_in("me").await;
    app.call(Req::post("/v1/internal/jobs/prune").bearer(&me.access))
        .await
        .assert_status(StatusCode::UNAUTHORIZED);
    app.call(
        Req::post("/v1/internal/jobs/drop-tables")
            .header("authorization", format!("Bearer {TOKEN}")),
    )
    .await
    .assert_status(StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn snapshots_give_boards_movement_and_best_ranks(db: PgPool) {
    let app = app_with_jobs(db.clone());
    let me = app.sign_in("me").await;
    let ana = app.sign_in("ana").await;
    let rui = app.sign_in("rui").await;
    rated(&db, ana.user_id, 1900.0, "cv").await;
    rated(&db, rui.user_id, 1800.0, "pt").await;
    rated(&db, me.user_id, 1700.0, "cv").await;

    // Before any snapshot: no movement.
    let board = get_board(&app, &me, "/v1/leaderboard?pool=overall").await;
    assert!(board.get("movement_since").is_none());

    // World: 3 rows; Cape Verde: 2; Portugal: 1.
    let done = run_job(&app, "leaderboard-snapshot").await;
    assert_eq!(done["job"], "leaderboard-snapshot");
    assert_eq!(done["rows"], 6);
    // Running again the same day replaces today's rows instead of adding more.
    assert_eq!(run_job(&app, "leaderboard-snapshot").await["rows"], 6);

    // Make it yesterday's, then I overtake both players and Lia joins the board.
    sqlx::query("UPDATE leaderboard_snapshots SET taken_on = taken_on - 1")
        .execute(&db)
        .await
        .unwrap();
    rated(&db, me.user_id, 2000.0, "cv").await;
    let lia = app.sign_in("lia").await;
    rated(&db, lia.user_id, 1600.0, "cv").await;

    let world = get_board(&app, &me, "/v1/leaderboard?pool=overall").await;
    assert!(world["movement_since"].is_string());
    let changes: Vec<(i64, Option<i64>)> = world["entries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| (e["rank"].as_i64().unwrap(), e["change"].as_i64()))
        .collect();
    // Me 3rd → 1st (+2), Ana and Rui down one, Lia new.
    assert_eq!(
        changes,
        [(1, Some(2)), (2, Some(-1)), (3, Some(-1)), (4, None)]
    );
    assert_eq!(world["your_best_rank"], 3, "best so far, from the snapshot");

    let cv = get_board(&app, &me, "/v1/leaderboard?pool=overall&scope=country").await;
    assert_eq!(cv["entries"][0]["change"], 1, "2nd → 1st in Cape Verde");

    // Today's snapshot records my new best.
    run_job(&app, "leaderboard-snapshot").await;
    let world = get_board(&app, &me, "/v1/leaderboard?pool=overall").await;
    assert_eq!(world["your_best_rank"], 1);
    // A worse day later doesn't lower the best.
    rated(&db, me.user_id, 1500.0, "cv").await;
    run_job(&app, "leaderboard-snapshot").await;
    let best: i32 = sqlx::query_scalar(
        "SELECT best_rank FROM rank_bests WHERE user_id = $1 AND pool = 'overall' AND scope = 'world'",
    )
    .bind(me.user_id)
    .fetch_one(&db)
    .await
    .unwrap();
    assert_eq!(best, 1);
}

async fn get_board(app: &TestApp, who: &Session, path: &str) -> Value {
    let res = app.call(Req::get(path).bearer(&who.access)).await;
    res.assert_status(StatusCode::OK);
    res.json()
}

#[sqlx::test]
async fn prune_removes_old_snapshots_and_abandoned_seeks_and_runs_are_recorded(db: PgPool) {
    let app = app_with_jobs(db.clone());
    let ana = app.sign_in("ana").await;
    let rui = app.sign_in("rui").await;
    rated(&db, ana.user_id, 1900.0, "cv").await;
    run_job(&app, "leaderboard-snapshot").await;
    sqlx::query(
        "INSERT INTO leaderboard_snapshots (taken_on, pool, scope, user_id, rank, rating)
         VALUES (current_date - 200, 'overall', 'world', $1, 1, 1900)",
    )
    .bind(ana.user_id)
    .execute(&db)
    .await
    .unwrap();
    for (user, minutes) in [(ana.user_id, 120), (rui.user_id, 1)] {
        sqlx::query(
            "INSERT INTO seeks (user_id, variant_id, initial_ms, increment_ms, rated, rating,
                                heartbeat_at)
             VALUES ($1, 'cv.standard', 300000, 3000, true, 1500,
                     now() - make_interval(mins => $2))",
        )
        .bind(user)
        .bind(minutes)
        .execute(&db)
        .await
        .unwrap();
    }

    assert_eq!(run_job(&app, "prune").await["rows"], 2);
    let snapshots: i64 = sqlx::query_scalar("SELECT count(*) FROM leaderboard_snapshots")
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(snapshots, 2, "today's world and country rows stay");
    let seeks: Vec<Uuid> = sqlx::query_scalar("SELECT user_id FROM seeks")
        .fetch_all(&db)
        .await
        .unwrap();
    assert_eq!(seeks, [rui.user_id], "a recent seek stays");

    let runs: Vec<(String, i64)> =
        sqlx::query_as("SELECT name, last_rows FROM job_runs ORDER BY name")
            .fetch_all(&db)
            .await
            .unwrap();
    assert_eq!(
        runs,
        [
            ("leaderboard-snapshot".to_string(), 2),
            ("prune".to_string(), 2)
        ]
    );
}

#[sqlx::test]
async fn deleting_an_account_removes_its_snapshots_and_best_ranks(db: PgPool) {
    let app = app_with_jobs(db.clone());
    let ana = app.sign_in("ana").await;
    rated(&db, ana.user_id, 1900.0, "cv").await;
    run_job(&app, "leaderboard-snapshot").await;
    app.call(Req::delete("/v1/me").bearer(&ana.access))
        .await
        .assert_status(StatusCode::NO_CONTENT);
    for sql in [
        "SELECT count(*) FROM leaderboard_snapshots WHERE user_id = $1",
        "SELECT count(*) FROM rank_bests WHERE user_id = $1",
    ] {
        assert_eq!(count(&db, sql, ana.user_id).await, 0, "{sql}");
    }
}
