//! Games against other players (`/v1/games`, `/v1/opponents`, ADR 0031): invitations by
//! handle, by past opponent and by link; turn-by-turn moves checked by the engine; resign,
//! timeouts and expiry; and the finished game in both players' history and opponent lists.

mod common;

use axum::http::StatusCode;
use common::*;
use ouril_engine::{apply_move, legal_moves, new_game, variant, Player, Status};
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

async fn invite(app: &TestApp, from: &Session, body: Value) -> Value {
    let res = app
        .call(Req::post("/v1/games").bearer(&from.access).json(body))
        .await;
    res.assert_status(StatusCode::CREATED);
    res.json()
}

async fn post(app: &TestApp, who: &Session, path: String, body: Value) -> Value {
    let res = app
        .call(Req::post(path).bearer(&who.access).json(body))
        .await;
    res.assert_status(StatusCode::OK);
    res.json()
}

async fn get(app: &TestApp, who: &Session, path: String) -> Value {
    let res = app.call(Req::get(path).bearer(&who.access)).await;
    res.assert_status(StatusCode::OK);
    res.json()
}

/// Ana invites Rui by handle and Rui accepts.
async fn active_game(app: &TestApp) -> (Session, Session, String) {
    let ana = with_handle(app, "ana", "ana").await;
    let rui = with_handle(app, "rui", "rui").await;
    let game = invite(app, &ana, json!({ "opponent_handle": "@Rui" })).await;
    let id = game["id"].as_str().unwrap().to_string();
    let accepted = post(app, &rui, format!("/v1/games/{id}/accept"), json!({})).await;
    assert_eq!(accepted["status"], "active");
    (ana, rui, id)
}

/// Whoever is to move plays the first legal move (the engine decides), until the game ends.
async fn play_out(app: &TestApp, ana: &Session, rui: &Session, id: &str) -> Value {
    let v = variant("cv.standard").unwrap();
    let mut game = get(app, ana, format!("/v1/games/{id}")).await;
    let first = if game["first_player"] == "south" {
        Player::South
    } else {
        Player::North
    };
    let mut state = new_game(v, first);
    while state.status == Status::Playing {
        let pit = legal_moves(v, &state)[0];
        let who = if state.to_move == Player::South {
            ana
        } else {
            rui
        };
        let ply = game["moves"].as_array().unwrap().len();
        game = post(
            app,
            who,
            format!("/v1/games/{id}/moves"),
            json!({ "ply": ply, "pit": pit }),
        )
        .await;
        state = apply_move(v, &state, pit).unwrap().state;
    }
    game
}

#[sqlx::test]
async fn an_invitation_by_handle_is_seen_accepted_and_played(db: PgPool) {
    let app = TestApp::new(db);
    let ana = with_handle(&app, "ana", "ana").await;
    let rui = with_handle(&app, "rui", "rui").await;
    let game = invite(&app, &ana, json!({ "opponent_handle": "rui" })).await;
    assert_eq!(game["status"], "invited");
    assert_eq!(game["you"], "south");
    assert_eq!(game["invited_by_you"], true);
    assert_eq!(game["opponent"]["handle"], "rui");
    assert!(
        game.get("invite_code").is_none(),
        "no link for a named invitation"
    );

    let rui_list = get(&app, &rui, "/v1/games".into()).await;
    let theirs = &rui_list["games"][0];
    assert_eq!(theirs["status"], "invited");
    assert_eq!(theirs["you"], "north");
    assert_eq!(theirs["invited_by_you"], false);
    assert_eq!(theirs["opponent"]["display_name"], "Player ana");

    let id = game["id"].as_str().unwrap();
    let active = post(&app, &rui, format!("/v1/games/{id}/accept"), json!({})).await;
    assert_eq!(active["status"], "active");
    assert!(active["turn_deadline"].is_string());
    let rui_turn = active["your_turn"].as_bool().unwrap();
    let ana_view = get(&app, &ana, format!("/v1/games/{id}")).await;
    assert_eq!(ana_view["your_turn"].as_bool().unwrap(), !rui_turn);
}

#[sqlx::test]
async fn a_finished_game_reaches_both_histories_and_opponent_lists(db: PgPool) {
    let app = TestApp::new(db);
    let (ana, rui, id) = active_game(&app).await;
    let end = play_out(&app, &ana, &rui, &id).await;
    assert_eq!(end["status"], "finished");
    assert!(end["result"]["outcome"].is_string());
    assert!(end.get("turn_deadline").is_none());

    for (me, them, seat) in [(&ana, "rui", "south"), (&rui, "ana", "north")] {
        let pulled = get(&app, me, "/v1/sync/pull?cursor=0".into()).await;
        let games: Vec<&Value> = pulled["changes"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|c| c["entity"] == "game")
            .collect();
        assert_eq!(games.len(), 1);
        let record = &games[0]["data"];
        assert_eq!(record["format"], 3);
        assert_eq!(record["mode"], "vs_player");
        assert_eq!(record["human_player"], seat);
        assert_eq!(record["opponent"]["handle"], them);
        assert_eq!(record["moves"], end["moves"]);
        // The record is a real game: it parses and replays like any other.
        let parsed: ouril_sync::GameRecord = serde_json::from_value(record.clone()).unwrap();
        assert_eq!(parsed.mode, ouril_sync::GameMode::VsPlayer);

        let opponents = get(&app, me, "/v1/opponents".into()).await;
        assert_eq!(opponents["opponents"][0]["player"]["handle"], them);
        assert_eq!(opponents["opponents"][0]["games_played"], 1);
    }
}

#[sqlx::test]
async fn moves_are_checked_against_the_turn_the_ply_and_the_rules(db: PgPool) {
    let app = TestApp::new(db);
    let (ana, rui, id) = active_game(&app).await;
    let game = get(&app, &ana, format!("/v1/games/{id}")).await;
    let (mover, waiter) = if game["your_turn"] == true {
        (&ana, &rui)
    } else {
        (&rui, &ana)
    };
    let my_pits: Vec<u8> = if game["first_player"] == "south" {
        (0..6).collect()
    } else {
        (6..12).collect()
    };
    let other_pit = if my_pits[0] == 0 { 6 } else { 0 };
    let path = format!("/v1/games/{id}/moves");

    // Not your turn.
    app.call(
        Req::post(path.clone())
            .bearer(&waiter.access)
            .json(json!({ "ply": 0, "pit": other_pit })),
    )
    .await
    .assert_error(StatusCode::CONFLICT, "conflict");
    // A pit that isn't yours: illegal.
    app.call(
        Req::post(path.clone())
            .bearer(&mover.access)
            .json(json!({ "ply": 0, "pit": other_pit })),
    )
    .await
    .assert_error(StatusCode::BAD_REQUEST, "bad_request");
    // A ply from the future.
    app.call(
        Req::post(path.clone())
            .bearer(&mover.access)
            .json(json!({ "ply": 5, "pit": my_pits[0] })),
    )
    .await
    .assert_error(StatusCode::CONFLICT, "conflict");

    let after = post(
        &app,
        mover,
        path.clone(),
        json!({ "ply": 0, "pit": my_pits[0] }),
    )
    .await;
    assert_eq!(after["moves"], json!([my_pits[0]]));
    // The same move again (a retry) is answered with the game, not applied twice.
    let retry = post(
        &app,
        mover,
        path.clone(),
        json!({ "ply": 0, "pit": my_pits[0] }),
    )
    .await;
    assert_eq!(retry["moves"], after["moves"]);
    // A different move for a ply already played is out of date.
    app.call(
        Req::post(path)
            .bearer(&mover.access)
            .json(json!({ "ply": 0, "pit": my_pits[1] })),
    )
    .await
    .assert_error(StatusCode::CONFLICT, "conflict");
}

#[sqlx::test]
async fn outsiders_and_bad_invitations_are_refused(db: PgPool) {
    let app = TestApp::new(db);
    let (ana, _rui, id) = active_game(&app).await;
    let eve = app.sign_in("eve").await;
    app.call(Req::get(format!("/v1/games/{id}")).bearer(&eve.access))
        .await
        .assert_error(StatusCode::NOT_FOUND, "not_found");
    app.call(Req::get("/v1/games/not-a-uuid").bearer(&eve.access))
        .await
        .assert_error(StatusCode::NOT_FOUND, "not_found");
    app.call(Req::get("/v1/games"))
        .await
        .assert_error(StatusCode::UNAUTHORIZED, "unauthorized");
    app.call(
        Req::post("/v1/games")
            .bearer(&ana.access)
            .json(json!({ "opponent_handle": "nobody_here" })),
    )
    .await
    .assert_error(StatusCode::NOT_FOUND, "not_found");
    app.call(
        Req::post("/v1/games")
            .bearer(&ana.access)
            .json(json!({ "opponent_handle": "ana" })),
    )
    .await
    .assert_error(StatusCode::BAD_REQUEST, "bad_request");
    app.call(
        Req::post("/v1/games")
            .bearer(&ana.access)
            .json(json!({ "variant_id": "xx.unknown" })),
    )
    .await
    .assert_error(StatusCode::BAD_REQUEST, "bad_request");
}

#[sqlx::test]
async fn an_invite_link_can_be_joined_once_by_someone_else(db: PgPool) {
    let app = TestApp::new(db);
    let ana = app.sign_in("ana").await;
    let rui = app.sign_in("rui").await;
    let eve = app.sign_in("eve").await;
    let game = invite(&app, &ana, json!({})).await;
    assert!(game.get("opponent").is_none());
    let code = game["invite_code"].as_str().unwrap().to_string();
    assert_eq!(code.len(), 32);

    app.call(
        Req::post("/v1/games/join")
            .bearer(&ana.access)
            .json(json!({ "invite_code": code })),
    )
    .await
    .assert_error(StatusCode::BAD_REQUEST, "bad_request");
    let joined = post(
        &app,
        &rui,
        "/v1/games/join".into(),
        json!({ "invite_code": code }),
    )
    .await;
    assert_eq!(joined["status"], "active");
    assert_eq!(joined["you"], "north");
    assert_eq!(joined["opponent"]["display_name"], "Player ana");
    // The link is used up.
    app.call(
        Req::post("/v1/games/join")
            .bearer(&eve.access)
            .json(json!({ "invite_code": code })),
    )
    .await
    .assert_error(StatusCode::NOT_FOUND, "not_found");
}

#[sqlx::test]
async fn invitations_can_be_declined_or_cancelled_and_expire(db: PgPool) {
    let app = TestApp::new(db);
    let ana = with_handle(&app, "ana", "ana").await;
    let rui = with_handle(&app, "rui", "rui").await;

    let a = invite(&app, &ana, json!({ "opponent_handle": "rui" })).await;
    let declined = post(
        &app,
        &rui,
        format!("/v1/games/{}/decline", a["id"].as_str().unwrap()),
        json!({}),
    )
    .await;
    assert_eq!(declined["status"], "declined");

    let b = invite(
        &app,
        &ana,
        json!({ "opponent_id": rui.user_id.to_string() }),
    )
    .await;
    let b_id = b["id"].as_str().unwrap();
    // Only the inviter can cancel, only the invited player can accept.
    app.call(
        Req::post(format!("/v1/games/{b_id}/cancel"))
            .bearer(&rui.access)
            .json(json!({})),
    )
    .await
    .assert_error(StatusCode::CONFLICT, "conflict");
    app.call(
        Req::post(format!("/v1/games/{b_id}/accept"))
            .bearer(&ana.access)
            .json(json!({})),
    )
    .await
    .assert_error(StatusCode::CONFLICT, "conflict");
    let cancelled = post(&app, &ana, format!("/v1/games/{b_id}/cancel"), json!({})).await;
    assert_eq!(cancelled["status"], "cancelled");

    let c = invite(&app, &ana, json!({ "opponent_handle": "rui" })).await;
    let c_id = Uuid::parse_str(c["id"].as_str().unwrap()).unwrap();
    sqlx::query("UPDATE online_games SET created_at = now() - interval '8 days' WHERE id = $1")
        .bind(c_id)
        .execute(&app.db)
        .await
        .unwrap();
    let list = get(&app, &rui, "/v1/games".into()).await;
    assert!(
        list["games"]
            .as_array()
            .unwrap()
            .iter()
            .all(|g| g["id"] != c["id"]),
        "an expired invitation leaves the invited player's list"
    );
    let expired = get(&app, &ana, format!("/v1/games/{c_id}")).await;
    assert_eq!(expired["status"], "expired");
}

#[sqlx::test]
async fn resigning_or_missing_the_deadline_loses(db: PgPool) {
    let app = TestApp::new(db);
    let (ana, rui, id) = active_game(&app).await;
    let resigned = post(&app, &ana, format!("/v1/games/{id}/resign"), json!({})).await;
    assert_eq!(resigned["status"], "finished");
    assert_eq!(resigned["result"]["reason"], "resigned");
    assert_eq!(resigned["result"]["outcome"], "north_wins");
    app.call(
        Req::post(format!("/v1/games/{id}/resign"))
            .bearer(&rui.access)
            .json(json!({})),
    )
    .await
    .assert_error(StatusCode::CONFLICT, "conflict");

    // A second game: whoever is to move lets the deadline pass.
    let again = invite(&app, &ana, json!({ "opponent_handle": "rui" })).await;
    let id2 = again["id"].as_str().unwrap().to_string();
    let started = post(&app, &rui, format!("/v1/games/{id2}/accept"), json!({})).await;
    let loser_is_rui = started["your_turn"] == true;
    sqlx::query(
        "UPDATE online_games SET turn_deadline = now() - interval '1 minute' WHERE id = $1",
    )
    .bind(Uuid::parse_str(&id2).unwrap())
    .execute(&app.db)
    .await
    .unwrap();
    let timed_out = get(&app, &ana, format!("/v1/games/{id2}")).await;
    assert_eq!(timed_out["status"], "finished");
    assert_eq!(timed_out["result"]["reason"], "timeout");
    assert_eq!(
        timed_out["result"]["outcome"],
        if loser_is_rui {
            "south_wins"
        } else {
            "north_wins"
        }
    );
    let opponents = get(&app, &rui, "/v1/opponents".into()).await;
    assert_eq!(opponents["opponents"][0]["games_played"], 2);
}

#[sqlx::test]
async fn deleting_an_account_resigns_its_games(db: PgPool) {
    let app = TestApp::new(db);
    let (ana, rui, id) = active_game(&app).await;
    app.call(Req::delete("/v1/me").bearer(&ana.access))
        .await
        .assert_status(StatusCode::NO_CONTENT);
    let game = get(&app, &rui, format!("/v1/games/{id}")).await;
    assert_eq!(game["status"], "finished");
    assert_eq!(game["result"]["reason"], "resigned");
    assert_eq!(game["result"]["outcome"], "north_wins");
    assert_eq!(game["opponent"]["display_name"], "Deleted player");
    let pulled = get(&app, &rui, "/v1/sync/pull?cursor=0".into()).await;
    let record = pulled["changes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["entity"] == "game")
        .unwrap();
    assert_eq!(record["data"]["opponent"]["display_name"], "Deleted player");
    assert_eq!(
        count(
            &app.db,
            "SELECT count(*) FROM games WHERE user_id = $1",
            ana.user_id
        )
        .await,
        0
    );
}

#[sqlx::test]
async fn clients_cannot_push_games_against_players(db: PgPool) {
    let app = TestApp::new(db);
    let ana = app.sign_in("ana").await;
    let mut record = serde_json::to_value(played_record()).unwrap();
    record["mode"] = json!("vs_player");
    let res = app
        .call(
            Req::post("/v1/sync/push")
                .bearer(&ana.access)
                .json(push_body(vec![mutation("game_finished", record)])),
        )
        .await;
    res.assert_status(StatusCode::OK);
    let result = &res.json()["results"][0];
    assert_eq!(result["status"], "rejected");
    assert_eq!(result["reason"], "invalid_payload");
}

#[sqlx::test]
async fn clients_cannot_push_server_only_end_reasons(db: PgPool) {
    let app = TestApp::new(db);
    let ana = app.sign_in("ana").await;
    for reason in ["timeout", "agreement"] {
        let mut record = serde_json::to_value(played_record()).unwrap();
        record["result"]["reason"] = json!(reason);
        let res = app
            .call(
                Req::post("/v1/sync/push")
                    .bearer(&ana.access)
                    .json(push_body(vec![mutation("game_finished", record)])),
            )
            .await;
        res.assert_status(StatusCode::OK);
        assert_eq!(res.json()["results"][0]["status"], "rejected", "{reason}");
    }
}
