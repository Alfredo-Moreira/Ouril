//! Live games (ADR 0033): clocks computed from timestamps, flag falls and aborts settled on
//! access, matchmaking through `seeks`, relay tokens, and publishing to the relay.

mod common;

use std::sync::{Arc, Mutex};

use axum::{http::StatusCode, routing, Router};
use common::*;
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use ouril_engine::{legal_moves, new_game, variant, Player};
use ouril_protocol::{RelayClaims, RELAY_AUDIENCE};
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

const FIVE_PLUS_THREE: &str = r#"{"initial_secs": 300, "increment_secs": 3}"#;

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

fn tc() -> Value {
    serde_json::from_str(FIVE_PLUS_THREE).unwrap()
}

/// Ana invites Rui to a live 5+3 game and Rui accepts.
async fn live_game(app: &TestApp, extra: Value) -> (Session, Session, Value) {
    let ana = with_handle(app, "ana", "ana").await;
    let rui = with_handle(app, "rui", "rui").await;
    let mut body = json!({ "opponent_handle": "rui", "time_control": tc() });
    body.as_object_mut()
        .unwrap()
        .extend(extra.as_object().unwrap().clone());
    let game = post(app, &ana, "/v1/games".into(), body).await;
    let id = game["id"].as_str().unwrap().to_string();
    let game = post(app, &rui, format!("/v1/games/{id}/accept"), json!({})).await;
    (ana, rui, game)
}

/// The player to move plays the first legal move; returns the game after it.
async fn move_once(app: &TestApp, ana: &Session, rui: &Session, game: &Value) -> Value {
    let v = variant("cv.standard").unwrap();
    let first = if game["first_player"] == "south" {
        Player::South
    } else {
        Player::North
    };
    let mut state = new_game(v, first);
    for pit in game["moves"].as_array().unwrap() {
        state = ouril_engine::apply_move(v, &state, pit.as_u64().unwrap() as u8)
            .unwrap()
            .state;
    }
    let pit = legal_moves(v, &state)[0];
    // Ana always sits South (the inviter).
    let who = if state.to_move == Player::South {
        ana
    } else {
        rui
    };
    let id = game["id"].as_str().unwrap();
    let ply = game["moves"].as_array().unwrap().len();
    post(
        app,
        who,
        format!("/v1/games/{id}/moves"),
        json!({ "ply": ply, "pit": pit }),
    )
    .await
}

/// Pretend the current turn started `secs` ago (and move the deadline with it).
async fn rewind_turn(db: &PgPool, id: &str, secs: f64) {
    sqlx::query(
        "UPDATE online_games
            SET turn_started_at = turn_started_at - make_interval(secs => $2),
                turn_deadline = turn_deadline - make_interval(secs => $2)
          WHERE id = $1::uuid",
    )
    .bind(id)
    .bind(secs)
    .execute(db)
    .await
    .unwrap();
}

#[sqlx::test]
async fn a_live_game_starts_with_full_clocks_that_run_after_both_players_move(db: PgPool) {
    let app = TestApp::new(db.clone());
    let (ana, rui, game) = live_game(&app, json!({})).await;
    assert_eq!(game["status"], "active");
    assert_eq!(game["mode"], "live");
    assert_eq!(game["time_control"], tc());
    assert_eq!(game["clocks"]["south_ms"], 300_000);
    assert_eq!(game["clocks"]["north_ms"], 300_000);
    assert_eq!(game["clocks"]["running"], false);
    assert!(game["server_now"].is_string());

    // The first two moves cost no time, however long they take.
    let id = game["id"].as_str().unwrap().to_string();
    rewind_turn(&db, &id, 20.0).await;
    let game = move_once(&app, &ana, &rui, &game).await;
    rewind_turn(&db, &id, 20.0).await;
    let game = move_once(&app, &ana, &rui, &game).await;
    assert_eq!(game["clocks"]["south_ms"], 300_000);
    assert_eq!(game["clocks"]["north_ms"], 300_000);
    assert_eq!(game["clocks"]["running"], true);

    // Now the mover pays for the time taken, and gets the increment back.
    let mover = if game["your_turn"] == true {
        game["you"].as_str().unwrap().to_string()
    } else if game["you"] == "south" {
        "north".to_string()
    } else {
        "south".to_string()
    };
    rewind_turn(&db, &id, 10.0).await;
    let game = move_once(&app, &ana, &rui, &game).await;
    let left = game["clocks"][format!("{mover}_ms")].as_i64().unwrap();
    assert!(
        (292_500..=293_100).contains(&left),
        "300 s - 10 s + 3 s, give or take the request: {left}"
    );
}

#[sqlx::test]
async fn a_flag_fall_loses_on_time_and_counts_in_the_live_pool(db: PgPool) {
    let app = TestApp::new(db.clone());
    let (ana, rui, game) = live_game(&app, json!({})).await;
    let id = game["id"].as_str().unwrap().to_string();
    let game = move_once(&app, &ana, &rui, &game).await;
    let game = move_once(&app, &ana, &rui, &game).await;
    let ana_view = get(&app, &ana, &format!("/v1/games/{id}")).await;
    let ana_to_move = ana_view["your_turn"] == true;
    let _ = game;

    // The player to move sat on their clock for six minutes: reading the game settles it.
    rewind_turn(&db, &id, 360.0).await;
    let ended = get(&app, &rui, &format!("/v1/games/{id}")).await;
    assert_eq!(ended["status"], "finished");
    assert_eq!(ended["result"]["reason"], "timeout");
    let (loser, winner) = if ana_to_move {
        ("south", "north")
    } else {
        ("north", "south")
    };
    assert_eq!(ended["result"]["outcome"], format!("{winner}_wins"));
    assert_eq!(ended["clocks"][format!("{loser}_ms")], 0);
    assert_eq!(ended["clocks"]["running"], false);
    let pools: Vec<&str> = ended["rating_changes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["pool"].as_str().unwrap())
        .collect();
    assert_eq!(pools, ["overall", "cv.standard:live"]);
}

#[sqlx::test]
async fn no_first_move_aborts_the_game_with_no_result(db: PgPool) {
    let app = TestApp::new(db.clone());
    let (ana, _rui, game) = live_game(&app, json!({})).await;
    let id = game["id"].as_str().unwrap().to_string();
    rewind_turn(&db, &id, 31.0).await;
    let aborted = get(&app, &ana, &format!("/v1/games/{id}")).await;
    assert_eq!(aborted["status"], "aborted");
    assert!(aborted.get("result").is_none());
    assert_eq!(
        count(
            &db,
            "SELECT count(*) FROM games WHERE user_id = $1",
            ana.user_id
        )
        .await,
        0,
        "nothing in anyone's history"
    );
    assert_eq!(
        count(
            &db,
            "SELECT count(*) FROM ratings WHERE user_id = $1",
            ana.user_id
        )
        .await,
        0
    );
}

#[sqlx::test]
async fn a_player_can_abort_only_before_both_have_moved(db: PgPool) {
    let app = TestApp::new(db);
    let (ana, rui, game) = live_game(&app, json!({})).await;
    let id = game["id"].as_str().unwrap().to_string();
    let game = move_once(&app, &ana, &rui, &game).await;
    let game = move_once(&app, &ana, &rui, &game).await;
    app.call(
        Req::post(format!("/v1/games/{id}/abort"))
            .bearer(&ana.access)
            .json(json!({})),
    )
    .await
    .assert_error(StatusCode::CONFLICT, "conflict");
    let _ = game;

    let (ana2, _, fresh) = {
        let a = app.sign_in("ana2").await;
        let rui = with_handle(&app, "rui2", "rui2").await;
        let g = post(
            &app,
            &a,
            "/v1/games".into(),
            json!({ "opponent_handle": "rui2", "time_control": tc() }),
        )
        .await;
        let gid = g["id"].as_str().unwrap().to_string();
        let g = post(&app, &rui, format!("/v1/games/{gid}/accept"), json!({})).await;
        (a, rui, g)
    };
    let fid = fresh["id"].as_str().unwrap();
    let aborted = post(&app, &ana2, format!("/v1/games/{fid}/abort"), json!({})).await;
    assert_eq!(aborted["status"], "aborted");
}

#[sqlx::test]
async fn only_the_offered_clocks_are_accepted(db: PgPool) {
    let app = TestApp::new(db);
    let ana = app.sign_in("ana").await;
    for tc in [
        json!({ "initial_secs": 1, "increment_secs": 0 }),
        json!({ "initial_secs": 300, "increment_secs": 60 }),
    ] {
        app.call(
            Req::post("/v1/games")
                .bearer(&ana.access)
                .json(json!({ "time_control": tc })),
        )
        .await
        .assert_error(StatusCode::BAD_REQUEST, "bad_request");
        app.call(
            Req::post("/v1/live/seek")
                .bearer(&ana.access)
                .json(json!({ "time_control": tc })),
        )
        .await
        .assert_error(StatusCode::BAD_REQUEST, "bad_request");
    }
}

#[sqlx::test]
async fn two_seeks_for_the_same_clock_are_paired(db: PgPool) {
    let app = TestApp::new(db.clone());
    let ana = app.sign_in("ana").await;
    let rui = app.sign_in("rui").await;
    let lia = app.sign_in("lia").await;

    let waiting = post(
        &app,
        &ana,
        "/v1/live/seek".into(),
        json!({ "time_control": tc() }),
    )
    .await;
    assert_eq!(waiting["state"], "waiting");
    assert_eq!(waiting["players_seeking"], 1);
    // Another clock doesn't pair with Ana.
    let other = post(
        &app,
        &lia,
        "/v1/live/seek".into(),
        json!({ "time_control": { "initial_secs": 600, "increment_secs": 5 } }),
    )
    .await;
    assert_eq!(other["state"], "waiting");

    let matched = post(
        &app,
        &rui,
        "/v1/live/seek".into(),
        json!({ "time_control": tc() }),
    )
    .await;
    assert_eq!(matched["state"], "matched");
    let id = matched["game_id"].as_str().unwrap().to_string();
    // Ana finds out on her next status check (or heartbeat).
    let ana_status = get(&app, &ana, "/v1/live/seek").await;
    assert_eq!(ana_status["state"], "matched");
    assert_eq!(ana_status["game_id"], id);
    let again = post(
        &app,
        &ana,
        "/v1/live/seek".into(),
        json!({ "time_control": tc() }),
    )
    .await;
    assert_eq!(again["game_id"], id, "a heartbeat after the match keeps it");

    let game = get(&app, &ana, &format!("/v1/games/{id}")).await;
    assert_eq!(game["status"], "active");
    assert_eq!(game["mode"], "live");
    assert_eq!(game["you"], "south", "the player who waited sits South");
    assert_eq!(game["rated"], true);
    let lia_status = get(&app, &lia, "/v1/live/seek").await;
    assert_eq!(lia_status["state"], "waiting");

    let idle = app
        .call(Req::delete("/v1/live/seek").bearer(&ana.access))
        .await;
    idle.assert_status(StatusCode::OK);
    assert_eq!(idle.json()["state"], "idle");
}

#[sqlx::test]
async fn far_apart_ratings_wait_until_the_window_opens_and_stale_seeks_are_skipped(db: PgPool) {
    let app = TestApp::new(db.clone());
    let ana = app.sign_in("ana").await;
    let rui = app.sign_in("rui").await;
    sqlx::query(
        "INSERT INTO ratings (user_id, pool, rating, deviation, volatility, peak)
         VALUES ($1, 'cv.standard:live', 2200, 80, 0.06, 2200)",
    )
    .bind(rui.user_id)
    .execute(&db)
    .await
    .unwrap();
    post(
        &app,
        &ana,
        "/v1/live/seek".into(),
        json!({ "time_control": tc() }),
    )
    .await;
    let r = post(
        &app,
        &rui,
        "/v1/live/seek".into(),
        json!({ "time_control": tc() }),
    )
    .await;
    assert_eq!(r["state"], "waiting", "700 points apart, both just started");

    // A stale seek is never paired, however long it waited.
    sqlx::query(
        "UPDATE seeks SET created_at = now() - interval '60 seconds',
                          heartbeat_at = now() - interval '60 seconds'
          WHERE user_id = $1",
    )
    .bind(ana.user_id)
    .execute(&db)
    .await
    .unwrap();
    let r = post(
        &app,
        &rui,
        "/v1/live/seek".into(),
        json!({ "time_control": tc() }),
    )
    .await;
    assert_eq!(r["state"], "waiting");

    // Ana's heartbeat keeps her waiting time: after 30 s, anyone will do.
    sqlx::query("UPDATE seeks SET heartbeat_at = now() WHERE user_id = $1")
        .bind(ana.user_id)
        .execute(&db)
        .await
        .unwrap();
    let r = post(
        &app,
        &rui,
        "/v1/live/seek".into(),
        json!({ "time_control": tc() }),
    )
    .await;
    assert_eq!(r["state"], "matched");
}

#[sqlx::test]
async fn relay_tokens_need_a_configured_relay(db: PgPool) {
    let app = TestApp::new(db);
    let ana = app.sign_in("ana").await;
    app.call(
        Req::post("/v1/live/token")
            .bearer(&ana.access)
            .json(json!({})),
    )
    .await
    .assert_error(StatusCode::NOT_IMPLEMENTED, "not_configured");
}

const RELAY_KEY: &str = "relay-api-key-for-tests-0123456789abcdef";
const RELAY_SECRET: &str = "relay-token-secret-for-tests-0123456789ab";

/// A stand-in relay that records what the server publishes, and an app pointed at it.
async fn app_with_relay(db: PgPool) -> (TestApp, Arc<Mutex<Vec<(String, Value)>>>) {
    let seen: Arc<Mutex<Vec<(String, Value)>>> = Arc::default();
    let record = seen.clone();
    let relay = Router::new().route(
        "/publish",
        routing::post(move |headers: axum::http::HeaderMap, body: String| {
            let record = record.clone();
            async move {
                let auth = headers
                    .get("authorization")
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or_default()
                    .to_string();
                record
                    .lock()
                    .unwrap()
                    .push((auth, serde_json::from_str(&body).unwrap()));
                StatusCode::NO_CONTENT
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, relay).await.unwrap() });
    let mut env = base_env();
    env.insert("RELAY_URL", format!("http://{addr}"));
    env.insert("RELAY_API_KEY", RELAY_KEY.into());
    env.insert("RELAY_TOKEN_SECRET", RELAY_SECRET.into());
    (TestApp::with_env(db, env), seen)
}

#[sqlx::test]
async fn relay_tokens_list_only_the_players_own_channels(db: PgPool) {
    let (app, _) = app_with_relay(db).await;
    let (ana, _rui, game) = live_game(&app, json!({})).await;
    let id = game["id"].as_str().unwrap();
    let t = post(
        &app,
        &ana,
        "/v1/live/token".into(),
        json!({ "game_id": id }),
    )
    .await;
    assert!(t["url"].as_str().unwrap().ends_with("/ws"));
    assert!(t["url"].as_str().unwrap().starts_with("ws://"));
    let mut v = Validation::new(Algorithm::HS256);
    v.set_audience(&[RELAY_AUDIENCE]);
    let claims = decode::<RelayClaims>(
        t["token"].as_str().unwrap(),
        &DecodingKey::from_secret(RELAY_SECRET.as_bytes()),
        &v,
    )
    .unwrap()
    .claims;
    assert_eq!(claims.sub, ana.user_id.to_string());
    assert_eq!(
        claims.ch,
        [format!("user:{}", ana.user_id), format!("game:{id}")]
    );
    assert!(claims.exp - claims.iat <= 600);

    // Someone else's game: 404, like every other game you're not in.
    let lia = app.sign_in("lia").await;
    app.call(
        Req::post("/v1/live/token")
            .bearer(&lia.access)
            .json(json!({ "game_id": id })),
    )
    .await
    .assert_status(StatusCode::NOT_FOUND);
    app.call(
        Req::post("/v1/live/token")
            .bearer(&lia.access)
            .json(json!({ "game_id": Uuid::now_v7().to_string() })),
    )
    .await
    .assert_status(StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn moves_and_matches_are_published_to_the_relay_with_ids_only(db: PgPool) {
    let (app, seen) = app_with_relay(db).await;
    let (ana, rui, game) = live_game(&app, json!({})).await;
    let id = game["id"].as_str().unwrap().to_string();
    move_once(&app, &ana, &rui, &game).await;
    let lia = app.sign_in("lia").await;
    let tom = app.sign_in("tom").await;
    post(
        &app,
        &lia,
        "/v1/live/seek".into(),
        json!({ "time_control": tc() }),
    )
    .await;
    post(
        &app,
        &tom,
        "/v1/live/seek".into(),
        json!({ "time_control": tc() }),
    )
    .await;

    // Publishing runs in the background.
    let lia_channel = format!("user:{}", lia.user_id);
    for _ in 0..100 {
        let done =
            seen.lock().unwrap().iter().any(|(_, b)| {
                b["channel"] == lia_channel.as_str() && b["event"]["type"] == "matched"
            });
        if done {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    let seen = seen.lock().unwrap().clone();
    assert!(seen
        .iter()
        .all(|(auth, _)| auth == &format!("Bearer {RELAY_KEY}")));
    let bodies: Vec<&Value> = seen.iter().map(|(_, b)| b).collect();
    assert!(bodies.contains(&&json!({
        "channel": format!("game:{id}"),
        "event": { "type": "game_changed", "game_id": id, "moves": 1 }
    })));
    assert!(bodies.contains(&&json!({
        "channel": format!("user:{}", ana.user_id),
        "event": { "type": "games_changed" }
    })));
    assert!(bodies
        .iter()
        .any(|b| b["channel"] == lia_channel.as_str() && b["event"]["type"] == "matched"));
}

async fn draw(app: &TestApp, who: &Session, id: &str, action: &str) -> TestResponse {
    app.call(
        Req::post(format!("/v1/games/{id}/draw"))
            .bearer(&who.access)
            .json(json!({ "action": action })),
    )
    .await
}

#[sqlx::test]
async fn a_draw_offer_can_be_declined_closes_on_a_move_and_ends_the_game_when_accepted(db: PgPool) {
    let app = TestApp::new(db.clone());
    let (ana, rui, game) = live_game(&app, json!({})).await;
    let id = game["id"].as_str().unwrap().to_string();
    let game = move_once(&app, &ana, &rui, &game).await;
    let game = move_once(&app, &ana, &rui, &game).await;

    // Ana offers; Rui sees it; Ana can't answer her own offer or offer twice.
    let offered = draw(&app, &ana, &id, "offer").await;
    offered.assert_status(StatusCode::OK);
    assert_eq!(offered.json()["draw_offer"], "south");
    let rui_view = get(&app, &rui, &format!("/v1/games/{id}")).await;
    assert_eq!(rui_view["draw_offer"], "south");
    draw(&app, &ana, &id, "accept")
        .await
        .assert_status(StatusCode::CONFLICT);
    draw(&app, &ana, &id, "offer")
        .await
        .assert_status(StatusCode::CONFLICT);

    // Rui declines; Ana can't offer again right away.
    let declined = draw(&app, &rui, &id, "decline").await;
    declined.assert_status(StatusCode::OK);
    assert!(declined.json().get("draw_offer").is_none());
    draw(&app, &ana, &id, "offer")
        .await
        .assert_status(StatusCode::CONFLICT);

    // Rui's own offer closes with the next move.
    draw(&app, &rui, &id, "offer")
        .await
        .assert_status(StatusCode::OK);
    let game = move_once(&app, &ana, &rui, &game).await;
    assert!(game.get("draw_offer").is_none(), "a move closes the offer");
    draw(&app, &ana, &id, "accept")
        .await
        .assert_status(StatusCode::CONFLICT);

    // Ten moves after her offer, Ana may offer again; Rui accepts: a rated draw.
    sqlx::query("UPDATE online_games SET draw_offer_ply = draw_offer_ply - 10 WHERE id = $1::uuid")
        .bind(&id)
        .execute(&db)
        .await
        .unwrap();
    let _ = game;
    draw(&app, &ana, &id, "offer")
        .await
        .assert_status(StatusCode::OK);
    let done = draw(&app, &rui, &id, "accept").await;
    done.assert_status(StatusCode::OK);
    let done = done.json();
    assert_eq!(done["status"], "finished");
    assert_eq!(done["result"]["outcome"], "draw");
    assert_eq!(done["result"]["reason"], "agreement");
    assert_eq!(done["rating_changes"].as_array().unwrap().len(), 2);
    // Both histories hold the game, as a draw by agreement.
    let reason: String =
        sqlx::query_scalar("SELECT record->'result'->>'reason' FROM games WHERE user_id = $1")
            .bind(ana.user_id)
            .fetch_one(&db)
            .await
            .unwrap();
    assert_eq!(reason, "agreement");
}

#[sqlx::test]
async fn the_lobby_lists_waiting_players_and_joining_one_starts_the_game(db: PgPool) {
    let app = TestApp::new(db.clone());
    let ana = with_handle(&app, "ana", "ana").await;
    let rui = app.sign_in("rui").await;
    post(
        &app,
        &ana,
        "/v1/live/seek".into(),
        json!({ "time_control": tc(), "rated": false }),
    )
    .await;

    // Ana doesn't see herself; Rui sees her, with her clock and choice.
    let own = get(&app, &ana, "/v1/live/lobby").await;
    assert_eq!(own["seeks"], json!([]));
    let lobby = get(&app, &rui, "/v1/live/lobby").await;
    let seek = &lobby["seeks"][0];
    assert_eq!(seek["player"]["handle"], "ana");
    assert_eq!(seek["rating"], 1500);
    assert_eq!(seek["time_control"], tc());
    assert_eq!(seek["rated"], false);
    assert_eq!(seek["variant_id"], "cv.standard");

    let joined = post(
        &app,
        &rui,
        "/v1/live/join".into(),
        json!({ "user_id": ana.user_id.to_string() }),
    )
    .await;
    assert_eq!(joined["state"], "matched");
    let id = joined["game_id"].as_str().unwrap().to_string();
    let ana_status = get(&app, &ana, "/v1/live/seek").await;
    assert_eq!(ana_status["game_id"], id);
    let game = get(&app, &rui, &format!("/v1/games/{id}")).await;
    assert_eq!(game["status"], "active");
    assert_eq!(game["rated"], false);
    assert_eq!(game["you"], "north", "the joining player sits North");

    // She's no longer listed, and can't be joined twice.
    let lia = app.sign_in("lia").await;
    assert_eq!(get(&app, &lia, "/v1/live/lobby").await["seeks"], json!([]));
    app.call(
        Req::post("/v1/live/join")
            .bearer(&lia.access)
            .json(json!({ "user_id": ana.user_id.to_string() })),
    )
    .await
    .assert_error(StatusCode::CONFLICT, "conflict");
    app.call(
        Req::post("/v1/live/join")
            .bearer(&lia.access)
            .json(json!({ "user_id": lia.user_id.to_string() })),
    )
    .await
    .assert_status(StatusCode::BAD_REQUEST);
}
