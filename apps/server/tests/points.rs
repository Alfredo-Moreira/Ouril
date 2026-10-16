//! The seasonal points board (`GET /v1/points`): online games that counted for ratings, wins
//! against the computer (capped per day), seasons, and scopes.

mod common;

use axum::http::StatusCode;
use common::*;
use ouril_engine::{legal_moves, new_game, variant, GameResult, Player};
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

async fn with_handle(app: &TestApp, subject: &str, handle: &str) -> Session {
    let s = app.sign_in(subject).await;
    sqlx::query("UPDATE users SET handle = $2 WHERE id = $1")
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

/// A game where both moved, then Rui resigned (`extra` merged into the invitation).
async fn ana_beats_rui(app: &TestApp, ana: &Session, rui: &Session, extra: Value) {
    let mut body = json!({ "opponent_handle": "rui" });
    body.as_object_mut()
        .unwrap()
        .extend(extra.as_object().unwrap().clone());
    let game = post(app, ana, "/v1/games".into(), body).await;
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

/// A win against the computer at `level`, as a device pushes it.
fn ai_win(level: &str) -> Value {
    let mut r = played_record();
    r.id = Uuid::now_v7().to_string();
    r.ai_level = Some(level.into());
    r.human_player = Some(match r.result.outcome {
        GameResult::SouthWins => Player::South,
        GameResult::NorthWins => Player::North,
        GameResult::Draw => panic!("the fixture game must have a winner"),
    });
    serde_json::to_value(r).unwrap()
}

async fn push(app: &TestApp, who: &Session, records: Vec<Value>) {
    let res = app
        .call(
            Req::post("/v1/sync/push")
                .bearer(&who.access)
                .json(push_body(
                    records
                        .into_iter()
                        .map(|r| mutation("game_finished", r))
                        .collect(),
                )),
        )
        .await;
    res.assert_status(StatusCode::OK);
    for r in res.json()["results"].as_array().unwrap() {
        assert_eq!(r["status"], "applied", "{r}");
    }
}

fn points_of(board: &Value, user: Uuid) -> Option<i64> {
    board["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["player"]["user_id"] == user.to_string())
        .map(|e| e["points"].as_i64().unwrap())
}

#[sqlx::test]
async fn online_games_that_counted_score_for_both_players(db: PgPool) {
    let app = TestApp::new(db);
    let ana = with_handle(&app, "ana", "ana").await;
    let rui = with_handle(&app, "rui", "rui").await;
    ana_beats_rui(&app, &ana, &rui, json!({})).await;
    // A casual game scores nothing.
    ana_beats_rui(&app, &ana, &rui, json!({ "rated": false })).await;

    let board = get(&app, &ana, "/v1/points").await;
    assert_eq!(board["current"], true);
    assert_eq!(board["scope"], "world");
    assert_eq!(points_of(&board, ana.user_id), Some(10), "a win");
    assert_eq!(
        points_of(&board, rui.user_id),
        Some(1),
        "a loss still counts"
    );
    assert_eq!(board["entries"][0]["online_games"], 1);
    assert_eq!(board["you"]["rank"], 1);
    assert_eq!(board["seasons"][0], board["season"]);
}

#[sqlx::test]
async fn wins_against_the_computer_score_by_level_up_to_a_daily_cap(db: PgPool) {
    let app = TestApp::new(db.clone());
    let me = app.sign_in("me").await;
    // Hard 5 + medium 3 + easy 1 = 9.
    push(
        &app,
        &me,
        vec![ai_win("hard"), ai_win("medium"), ai_win("easy")],
    )
    .await;
    let board = get(&app, &me, "/v1/points").await;
    assert_eq!(points_of(&board, me.user_id), Some(9));
    assert_eq!(board["you"]["ai_wins"], 3);
    // A loss never scores (checked while there's room under the cap).
    let mut loss = ai_win("hard");
    loss["human_player"] = json!(if loss["human_player"] == "south" {
        "north"
    } else {
        "south"
    });
    push(&app, &me, vec![loss]).await;
    assert_eq!(
        points_of(&get(&app, &me, "/v1/points").await, me.user_id),
        Some(9)
    );

    // Three more hard wins would be 15; only 11 more fit under today's 20.
    push(
        &app,
        &me,
        vec![ai_win("hard"), ai_win("hard"), ai_win("hard")],
    )
    .await;
    let board = get(&app, &me, "/v1/points").await;
    assert_eq!(points_of(&board, me.user_id), Some(20));
    // The same game pushed again scores nothing more.
    let again = ai_win("hard");
    push(&app, &me, vec![again.clone()]).await;
    push(&app, &me, vec![again]).await;
    assert_eq!(
        points_of(&get(&app, &me, "/v1/points").await, me.user_id),
        Some(20)
    );
}

#[sqlx::test]
async fn seasons_and_country_boards(db: PgPool) {
    let app = TestApp::new(db.clone());
    let me = app.sign_in("me").await;
    let pt = app.sign_in("pt").await;
    sqlx::query("UPDATE users SET country = 'cv' WHERE id = $1")
        .bind(me.user_id)
        .execute(&db)
        .await
        .unwrap();
    sqlx::query("UPDATE users SET country = 'pt' WHERE id = $1")
        .bind(pt.user_id)
        .execute(&db)
        .await
        .unwrap();
    for (user, season, points) in [
        (me.user_id, "2026-09", 40),
        (pt.user_id, "2026-09", 55),
        (me.user_id, "2026-08", 7),
    ] {
        sqlx::query(
            "INSERT INTO season_points (season, user_id, points, online_games) VALUES ($1, $2, $3, 1)",
        )
        .bind(season)
        .bind(user)
        .bind(points)
        .execute(&db)
        .await
        .unwrap();
    }
    let sept = get(&app, &me, "/v1/points?season=2026-09").await;
    assert_eq!(sept["current"], false);
    assert_eq!(points_of(&sept, pt.user_id), Some(55));
    assert_eq!(sept["you"]["rank"], 2);
    let seasons: Vec<&str> = sept["seasons"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s.as_str().unwrap())
        .collect();
    assert!(seasons.ends_with(&["2026-09", "2026-08"]), "{seasons:?}");

    let cv = get(&app, &me, "/v1/points?season=2026-09&scope=country").await;
    assert_eq!(cv["country"], "cv");
    assert_eq!(cv["entries"].as_array().unwrap().len(), 1);
    assert_eq!(cv["you"]["rank"], 1);

    for bad in [
        "/v1/points?season=2026-13",
        "/v1/points?season=last",
        "/v1/points?scope=country",
    ] {
        let who = if bad.contains("scope") { &pt } else { &me };
        if bad.contains("scope") {
            sqlx::query("UPDATE users SET country = NULL WHERE id = $1")
                .bind(pt.user_id)
                .execute(&db)
                .await
                .unwrap();
        }
        app.call(Req::get(bad).bearer(&who.access))
            .await
            .assert_status(StatusCode::BAD_REQUEST);
    }
}

#[sqlx::test]
async fn deleting_the_account_removes_its_points(db: PgPool) {
    let app = TestApp::new(db.clone());
    let me = app.sign_in("me").await;
    push(&app, &me, vec![ai_win("medium")]).await;
    app.call(Req::delete("/v1/me").bearer(&me.access))
        .await
        .assert_status(StatusCode::NO_CONTENT);
    for sql in [
        "SELECT count(*) FROM season_points WHERE user_id = $1",
        "SELECT count(*) FROM ai_points_daily WHERE user_id = $1",
    ] {
        assert_eq!(count(&db, sql, me.user_id).await, 0, "{sql}");
    }
}
