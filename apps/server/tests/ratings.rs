//! Ratings and leaderboards (ADR 0034): rated games update the overall pool and the game's
//! own pool; casual, unplayed and farmed games don't count; boards rank eligible players,
//! world and by country.

mod common;

use axum::http::StatusCode;
use common::*;
use ouril_engine::{legal_moves, new_game, variant, Player};
use ouril_rating::{deviation_after, Config, Rating, PERIOD_SECS};
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

/// Ana invites Rui (`extra` merged into the invitation) and Rui accepts.
async fn start(app: &TestApp, ana: &Session, rui: &Session, extra: Value) -> Value {
    let mut body = json!({ "opponent_handle": "rui" });
    body.as_object_mut()
        .unwrap()
        .extend(extra.as_object().unwrap().clone());
    let game = post(app, ana, "/v1/games".into(), body).await;
    let id = game["id"].as_str().unwrap().to_string();
    post(app, rui, format!("/v1/games/{id}/accept"), json!({})).await
}

/// Both players make `plies` moves (the first legal move each time).
async fn play(app: &TestApp, ana: &Session, rui: &Session, game: &Value, plies: usize) {
    let v = variant(game["variant_id"].as_str().unwrap()).unwrap();
    let first = if game["first_player"] == "south" {
        Player::South
    } else {
        Player::North
    };
    let id = game["id"].as_str().unwrap();
    let mut state = new_game(v, first);
    for ply in 0..plies {
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
}

/// A game where both moved, then Rui resigned: Ana wins.
async fn ana_wins(app: &TestApp, ana: &Session, rui: &Session, extra: Value) -> Value {
    let game = start(app, ana, rui, extra).await;
    play(app, ana, rui, &game, 2).await;
    let id = game["id"].as_str().unwrap();
    post(app, rui, format!("/v1/games/{id}/resign"), json!({})).await;
    get(app, ana, &format!("/v1/games/{id}")).await
}

fn pools(changes: &Value) -> Vec<&str> {
    changes
        .as_array()
        .map(|a| a.iter().map(|c| c["pool"].as_str().unwrap()).collect())
        .unwrap_or_default()
}

#[sqlx::test]
async fn a_rated_game_moves_the_overall_and_the_variant_ratings(db: PgPool) {
    let app = TestApp::new(db);
    let ana = with_handle(&app, "ana", "ana").await;
    let rui = with_handle(&app, "rui", "rui").await;
    let game = ana_wins(&app, &ana, &rui, json!({ "variant_id": "cv.across" })).await;
    assert_eq!(game["rated"], true, "rated by default");
    assert_eq!(
        pools(&game["rating_changes"]),
        ["overall", "cv.across:async"]
    );
    for c in game["rating_changes"].as_array().unwrap() {
        assert_eq!(c["before"], 1500);
        assert!(c["after"].as_i64().unwrap() > 1500, "the winner goes up");
    }
    let id = game["id"].as_str().unwrap();
    let theirs = get(&app, &rui, &format!("/v1/games/{id}")).await;
    assert!(theirs["rating_changes"][0]["after"].as_i64().unwrap() < 1500);

    let mine = get(&app, &ana, "/v1/ratings/me").await;
    let ratings = mine["ratings"].as_array().unwrap();
    assert_eq!(ratings.len(), 2);
    assert_eq!(ratings[0]["pool"], "overall");
    assert_eq!(ratings[0]["games"], 1);
    assert_eq!(ratings[0]["wins"], 1);
    assert_eq!(ratings[0]["provisional"], true);
    assert_eq!(ratings[1]["pool"], "cv.across:async");
}

#[sqlx::test]
async fn casual_and_unplayed_games_dont_count(db: PgPool) {
    let app = TestApp::new(db);
    let ana = with_handle(&app, "ana", "ana").await;
    let rui = with_handle(&app, "rui", "rui").await;

    let casual = ana_wins(&app, &ana, &rui, json!({ "rated": false })).await;
    assert_eq!(casual["rated"], false);
    assert!(casual.get("rating_changes").is_none());

    // Resigned after a single move: the other player never moved.
    let game = start(&app, &ana, &rui, json!({})).await;
    play(&app, &ana, &rui, &game, 1).await;
    let id = game["id"].as_str().unwrap();
    let ended = post(&app, &rui, format!("/v1/games/{id}/resign"), json!({})).await;
    assert_eq!(ended["status"], "finished");
    assert!(ended.get("rating_changes").is_none());

    let mine = get(&app, &ana, "/v1/ratings/me").await;
    assert_eq!(mine["ratings"], json!([]));
}

#[sqlx::test]
async fn only_three_rated_games_a_day_count_between_the_same_two_players(db: PgPool) {
    let app = TestApp::new(db);
    let ana = with_handle(&app, "ana", "ana").await;
    let rui = with_handle(&app, "rui", "rui").await;
    for _ in 0..3 {
        let g = ana_wins(&app, &ana, &rui, json!({})).await;
        assert_eq!(pools(&g["rating_changes"]).len(), 2);
    }
    let fourth = ana_wins(&app, &ana, &rui, json!({})).await;
    assert!(fourth.get("rating_changes").is_none());
    let mine = get(&app, &ana, "/v1/ratings/me").await;
    assert_eq!(mine["ratings"][0]["games"], 3);
}

/// A player's `overall` rating to insert directly, `days_ago` after their last rated game.
struct Seed {
    rating: f64,
    deviation: f64,
    volatility: f64,
    games: i32,
    days_ago: f64,
}

impl Default for Seed {
    /// Eligible for the boards: 12 games, sure of the rating, played yesterday.
    fn default() -> Self {
        Seed {
            rating: 1500.0,
            deviation: 60.0,
            volatility: 0.06,
            games: 12,
            days_ago: 1.0,
        }
    }
}

async fn seed(db: &PgPool, user: Uuid, s: Seed) {
    sqlx::query(
        "INSERT INTO ratings (user_id, pool, rating, deviation, volatility, games, wins, peak,
                              last_rated_at)
         VALUES ($1, 'overall', $2, $3, $4, $5, $5, $2,
                 now() - make_interval(secs => $6))",
    )
    .bind(user)
    .bind(s.rating)
    .bind(s.deviation)
    .bind(s.volatility)
    .bind(s.games)
    .bind(s.days_ago * PERIOD_SECS)
    .execute(db)
    .await
    .unwrap();
}

async fn set_country(db: &PgPool, user: Uuid, country: &str) {
    sqlx::query("UPDATE users SET country = $2 WHERE id = $1")
        .bind(user)
        .bind(country)
        .execute(db)
        .await
        .unwrap();
}

#[sqlx::test]
async fn boards_rank_eligible_players_worldwide_and_by_country(db: PgPool) {
    let app = TestApp::new(db.clone());
    let me = app.sign_in("me").await;
    let mut ids = Vec::new();
    for (name, rating, country) in [
        ("top", 1900.0, "cv"),
        ("second", 1800.0, "pt"),
        ("third", 1700.0, "cv"),
    ] {
        let s = app.sign_in(name).await;
        seed(
            &db,
            s.user_id,
            Seed {
                rating,
                ..Seed::default()
            },
        )
        .await;
        set_country(&db, s.user_id, country).await;
        ids.push(s.user_id);
    }
    // Not eligible: too few games, too uncertain, inactive, deleted.
    for (name, games, deviation, days) in [
        ("rookie", 3, 60.0, 1.0),
        ("unsure", 30, 200.0, 1.0),
        ("gone", 30, 60.0, 90.0),
        ("deleted", 30, 60.0, 1.0),
    ] {
        let s = app.sign_in(name).await;
        let not_eligible = Seed {
            rating: 2500.0,
            deviation,
            games,
            days_ago: days,
            ..Seed::default()
        };
        seed(&db, s.user_id, not_eligible).await;
        set_country(&db, s.user_id, "cv").await;
        if name == "deleted" {
            sqlx::query("UPDATE users SET deleted_at = now() WHERE id = $1")
                .bind(s.user_id)
                .execute(&db)
                .await
                .unwrap();
        }
    }
    seed(
        &db,
        me.user_id,
        Seed {
            rating: 1600.0,
            ..Seed::default()
        },
    )
    .await;
    set_country(&db, me.user_id, "cv").await;

    let world = get(&app, &me, "/v1/leaderboard?pool=overall").await;
    assert_eq!(world["scope"], "world");
    let ratings: Vec<i64> = world["entries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["rating"].as_i64().unwrap())
        .collect();
    assert_eq!(ratings, [1900, 1800, 1700, 1600]);
    assert_eq!(world["entries"][0]["rank"], 1);
    assert_eq!(world["entries"][0]["country"], "cv");
    assert_eq!(world["you"]["rank"], 4);

    // My country by default; Portugal's player isn't on it.
    let cv = get(&app, &me, "/v1/leaderboard?pool=overall&scope=country").await;
    assert_eq!(cv["country"], "cv");
    let ranks: Vec<(i64, i64)> = cv["entries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| (e["rank"].as_i64().unwrap(), e["rating"].as_i64().unwrap()))
        .collect();
    assert_eq!(ranks, [(1, 1900), (2, 1700), (3, 1600)]);
    let pt = get(
        &app,
        &me,
        "/v1/leaderboard?pool=overall&scope=country&country=PT",
    )
    .await;
    assert_eq!(pt["entries"].as_array().unwrap().len(), 1);
    assert!(pt.get("you").is_none(), "I'm not on Portugal's board");

    // Another pool is empty.
    let other = get(&app, &me, "/v1/leaderboard?pool=cv.standard:async").await;
    assert_eq!(other["entries"], json!([]));
}

/// The board's SQL grows the deviation exactly like `ouril_rating::deviation_after`: a
/// volatile player crosses the provisional line between one and three days of inactivity.
#[sqlx::test]
async fn deviation_grows_like_ouril_rating(db: PgPool) {
    let cfg = Config::default();
    let r = Rating {
        rating: 1500.0,
        deviation: 100.0,
        volatility: 0.2,
    };
    assert!(deviation_after(&cfg, &r, 1.0) <= cfg.provisional_deviation);
    assert!(deviation_after(&cfg, &r, 3.0) > cfg.provisional_deviation);

    let app = TestApp::new(db.clone());
    let me = app.sign_in("me").await;
    let fresh = app.sign_in("fresh").await;
    let stale = app.sign_in("stale").await;
    let volatile = |days_ago| Seed {
        deviation: 100.0,
        volatility: 0.2,
        days_ago,
        ..Seed::default()
    };
    seed(&db, fresh.user_id, volatile(1.0)).await;
    seed(&db, stale.user_id, volatile(3.0)).await;
    let board = get(&app, &me, "/v1/leaderboard?pool=overall").await;
    let on: Vec<&str> = board["entries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["player"]["user_id"].as_str().unwrap())
        .collect();
    assert_eq!(on, [fresh.user_id.to_string()]);
}

#[sqlx::test]
async fn bad_queries_are_refused(db: PgPool) {
    let app = TestApp::new(db);
    let me = app.sign_in("me").await;
    for path in [
        "/v1/leaderboard?pool=nope",
        "/v1/leaderboard?pool=cv.standard:blitz",
        // No country in my profile, none asked for.
        "/v1/leaderboard?pool=overall&scope=country",
        "/v1/leaderboard?pool=overall&scope=country&country=cpv",
    ] {
        app.call(Req::get(path).bearer(&me.access))
            .await
            .assert_status(StatusCode::BAD_REQUEST);
    }
    app.call(Req::get("/v1/leaderboard?pool=overall"))
        .await
        .assert_status(StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn deleting_the_account_removes_its_ratings(db: PgPool) {
    let app = TestApp::new(db.clone());
    let ana = with_handle(&app, "ana", "ana").await;
    let rui = with_handle(&app, "rui", "rui").await;
    ana_wins(&app, &ana, &rui, json!({})).await;
    app.call(Req::delete("/v1/me").bearer(&ana.access))
        .await
        .assert_status(StatusCode::NO_CONTENT);
    assert_eq!(
        count(
            &db,
            "SELECT count(*) FROM ratings WHERE user_id = $1",
            ana.user_id
        )
        .await,
        0
    );
    assert_eq!(
        count(
            &db,
            "SELECT count(*) FROM rating_changes WHERE user_id = $1",
            ana.user_id
        )
        .await,
        0
    );
    // Rui keeps his own.
    assert_eq!(
        count(
            &db,
            "SELECT count(*) FROM ratings WHERE user_id = $1",
            rui.user_id
        )
        .await,
        2
    );
}

/// Two games between the same players finishing at once: the second waits for the first to
/// commit, then sees its count. With two rated games already that day, only one more counts.
#[sqlx::test]
async fn the_daily_pair_limit_holds_when_games_end_at_once(db: PgPool) {
    use ouril_engine::GameResult;
    use ouril_server::ratings::{rate_finished_game, FinishedGame};
    use time::OffsetDateTime;

    let app = TestApp::new(db.clone());
    let ana = with_handle(&app, "ana", "ana").await;
    let rui = with_handle(&app, "rui", "rui").await;
    for _ in 0..2 {
        ana_wins(&app, &ana, &rui, json!({})).await;
    }
    // Two more finished games, casual for now, to rate by hand side by side.
    let mut games = Vec::new();
    for _ in 0..2 {
        let g = ana_wins(&app, &ana, &rui, json!({ "rated": false })).await;
        let id: Uuid = g["id"].as_str().unwrap().parse().unwrap();
        sqlx::query("UPDATE online_games SET rated = true WHERE id = $1")
            .bind(id)
            .execute(&db)
            .await
            .unwrap();
        let south = if g["you"] == "south" {
            ana.user_id
        } else {
            rui.user_id
        };
        let north = if south == ana.user_id {
            rui.user_id
        } else {
            ana.user_id
        };
        games.push(FinishedGame {
            id,
            variant_id: "cv.standard".into(),
            mode: "async".into(),
            south,
            north,
            outcome: GameResult::Draw,
            plies: 2,
            ended: OffsetDateTime::now_utc(),
        });
    }
    let second = games.pop().unwrap();
    let first = games.pop().unwrap();

    let mut tx1 = db.begin().await.unwrap();
    rate_finished_game(&mut tx1, &first).await.unwrap();
    // The second game starts while the first is still uncommitted...
    let pool = db.clone();
    let other = tokio::spawn(async move {
        let mut tx2 = pool.begin().await.unwrap();
        rate_finished_game(&mut tx2, &second).await.unwrap();
        tx2.commit().await.unwrap();
    });
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    assert!(!other.is_finished(), "the second game waits for the first");
    tx1.commit().await.unwrap();
    other.await.unwrap();

    let applied: i64 = sqlx::query_scalar("SELECT count(*) FROM online_games WHERE rating_applied")
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(
        applied, 3,
        "two earlier games, then only the first of the pair"
    );
}

/// A timeout settled days late ends in the past; it must not move "last rated" back.
#[sqlx::test]
async fn a_late_timeout_never_moves_the_last_rated_time_back(db: PgPool) {
    let app = TestApp::new(db.clone());
    let ana = with_handle(&app, "ana", "ana").await;
    let rui = with_handle(&app, "rui", "rui").await;
    let old = start(&app, &ana, &rui, json!({})).await;
    play(&app, &ana, &rui, &old, 2).await;
    // A newer rated game finishes first.
    ana_wins(&app, &ana, &rui, json!({})).await;
    // The old game's deadline passed two days ago; reading it settles the timeout now.
    let id = old["id"].as_str().unwrap();
    sqlx::query(
        "UPDATE online_games SET turn_deadline = now() - interval '2 days' WHERE id = $1::uuid",
    )
    .bind(id)
    .execute(&db)
    .await
    .unwrap();
    let settled = get(&app, &ana, &format!("/v1/games/{id}")).await;
    assert_eq!(settled["result"]["reason"], "timeout");
    assert_eq!(
        pools(&settled["rating_changes"]).len(),
        2,
        "it still counts"
    );
    let recent: bool = sqlx::query_scalar(
        "SELECT last_rated_at > now() - interval '1 hour' FROM ratings
          WHERE user_id = $1 AND pool = 'overall'",
    )
    .bind(ana.user_id)
    .fetch_one(&db)
    .await
    .unwrap();
    assert!(recent);
}

#[sqlx::test]
async fn bad_query_strings_get_the_json_error_body(db: PgPool) {
    let app = TestApp::new(db);
    let me = app.sign_in("me").await;
    for path in [
        "/v1/leaderboard",
        "/v1/leaderboard?pool=overall&scope=World",
    ] {
        app.call(Req::get(path).bearer(&me.access))
            .await
            .assert_error(StatusCode::BAD_REQUEST, "bad_request");
    }
}
