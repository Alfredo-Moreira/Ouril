//! Player profiles (`GET /v1/players/{id}`): by ID or handle, ratings and their history,
//! badges from best ranks, and the record between two players.

mod common;

use axum::http::StatusCode;
use common::*;
use ouril_engine::{legal_moves, new_game, variant, Player};
use serde_json::{json, Value};
use sqlx::PgPool;

async fn with_handle(app: &TestApp, subject: &str, handle: &str) -> Session {
    let s = app.sign_in(subject).await;
    sqlx::query("UPDATE users SET handle = $2, country = 'cv' WHERE id = $1")
        .bind(s.user_id)
        .bind(handle)
        .execute(&app.db)
        .await
        .unwrap();
    s
}

async fn post(app: &TestApp, who: &Session, path: String, body: Value) -> Value {
    let res = app
        .call(Req::post(path).bearer(&who.access).json(body))
        .await;
    assert!(res.status.is_success(), "{} {}", res.status, res.text());
    res.json()
}

async fn get(app: &TestApp, who: &Session, path: &str) -> Value {
    let res = app.call(Req::get(path).bearer(&who.access)).await;
    res.assert_status(StatusCode::OK);
    res.json()
}

/// Ana invites Rui to a rated game; both move, then Rui resigns.
async fn ana_beats_rui(app: &TestApp, ana: &Session, rui: &Session) {
    let game = post(
        app,
        ana,
        "/v1/games".into(),
        json!({ "opponent_handle": "rui" }),
    )
    .await;
    let id = game["id"].as_str().unwrap().to_string();
    let game = post(app, rui, format!("/v1/games/{id}/accept"), json!({})).await;
    let v = variant("cv.standard").unwrap();
    let first = if game["first_player"] == "south" {
        Player::South
    } else {
        Player::North
    };
    let mut state = new_game(v, first);
    for ply in 0..2 {
        let pit = legal_moves(v, &state)[0];
        let who = if state.to_move == Player::South {
            ana
        } else {
            rui
        };
        post(
            app,
            who,
            format!("/v1/games/{id}/moves"),
            json!({ "ply": ply, "pit": pit }),
        )
        .await;
        state = ouril_engine::apply_move(v, &state, pit).unwrap().state;
    }
    post(app, rui, format!("/v1/games/{id}/resign"), json!({})).await;
}

#[sqlx::test]
async fn a_profile_shows_ratings_history_and_the_record_between_players(db: PgPool) {
    let app = TestApp::new(db);
    let ana = with_handle(&app, "ana", "ana").await;
    let rui = with_handle(&app, "rui", "rui").await;
    ana_beats_rui(&app, &ana, &rui).await;
    ana_beats_rui(&app, &ana, &rui).await;

    // Rui looks at Ana, by handle (any case).
    let p = get(&app, &rui, "/v1/players/@ANA").await;
    assert_eq!(p["player"]["user_id"], ana.user_id.to_string());
    assert_eq!(p["player"]["handle"], "ana");
    assert_eq!(p["country"], "cv");
    assert_eq!(p["you"], false);
    assert!(p["member_since"].is_string());
    assert_eq!(p["ratings"][0]["pool"], "overall");
    assert_eq!(p["ratings"][0]["wins"], 2);
    let history = p["history"].as_array().unwrap();
    assert_eq!(history[0]["pool"], "overall");
    assert_eq!(history[1]["pool"], "cv.standard:async");
    let points = history[0]["points"].as_array().unwrap();
    assert_eq!(points.len(), 2, "one point per rated game");
    assert!(points[1]["rating"].as_i64() > points[0]["rating"].as_i64());
    assert_eq!(
        p["head_to_head"],
        json!({ "wins": 0, "draws": 0, "losses": 2 }),
        "from Rui's side"
    );

    // By ID; on your own profile there's no record against yourself.
    let mine = get(&app, &ana, &format!("/v1/players/{}", ana.user_id)).await;
    assert_eq!(mine["you"], true);
    assert!(mine.get("head_to_head").is_none());
}

#[sqlx::test]
async fn badges_come_from_top_ten_best_ranks(db: PgPool) {
    let app = TestApp::new(db.clone());
    let ana = with_handle(&app, "ana", "ana").await;
    for (pool, scope, rank) in [
        ("overall", "cv", 1),
        ("overall", "world", 4),
        ("cv.standard:live", "world", 10),
        ("cv.across:async", "world", 11),
    ] {
        sqlx::query(
            "INSERT INTO rank_bests (user_id, pool, scope, best_rank, achieved_on)
             VALUES ($1, $2, $3, $4, current_date)",
        )
        .bind(ana.user_id)
        .bind(pool)
        .bind(scope)
        .bind(rank)
        .execute(&db)
        .await
        .unwrap();
    }
    let p = get(&app, &ana, "/v1/players/@ana").await;
    let badges: Vec<(String, String, i64)> = p["badges"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| {
            (
                b["pool"].as_str().unwrap().to_string(),
                b["scope"].as_str().unwrap().to_string(),
                b["rank"].as_i64().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        badges,
        [
            ("overall".into(), "cv".into(), 1),
            ("overall".into(), "world".into(), 4),
            ("cv.standard:live".into(), "world".into(), 10),
        ],
        "best first; 11th isn't a badge"
    );
}

#[sqlx::test]
async fn unknown_deleted_or_anonymous_requests_get_nothing(db: PgPool) {
    let app = TestApp::new(db);
    let ana = with_handle(&app, "ana", "ana").await;
    let rui = with_handle(&app, "rui", "rui").await;
    for path in [
        "/v1/players/@nobody",
        "/v1/players/not-a-uuid",
        "/v1/players/01a137aa-33cf-7667-a7bb-000000000000",
    ] {
        app.call(Req::get(path).bearer(&ana.access))
            .await
            .assert_status(StatusCode::NOT_FOUND);
    }
    app.call(Req::delete("/v1/me").bearer(&rui.access))
        .await
        .assert_status(StatusCode::NO_CONTENT);
    app.call(Req::get(format!("/v1/players/{}", rui.user_id)).bearer(&ana.access))
        .await
        .assert_status(StatusCode::NOT_FOUND);
    app.call(Req::get("/v1/players/@ana"))
        .await
        .assert_status(StatusCode::UNAUTHORIZED);
}
