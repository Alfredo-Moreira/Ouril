//! `POST /v1/sync/push` and `GET /v1/sync/pull` (ADR 0011, data-and-sync.md).

mod common;

use axum::http::StatusCode;
use common::*;
use ouril_protocol::{SyncPull, SyncPushResult, MAX_MUTATIONS_PER_PUSH};
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

async fn push(app: &TestApp, s: &Session, mutations: Vec<Value>) -> SyncPushResult {
    let res = app
        .call(
            Req::post("/v1/sync/push")
                .bearer(&s.access)
                .json(push_body(mutations)),
        )
        .await;
    res.assert_status(StatusCode::OK);
    serde_json::from_value(res.json()).expect("SyncPushResult")
}

async fn pull(app: &TestApp, s: &Session, query: &str) -> SyncPull {
    let res = app
        .call(Req::get(format!("/v1/sync/pull{query}")).bearer(&s.access))
        .await;
    res.assert_status(StatusCode::OK);
    serde_json::from_value(res.json()).expect("SyncPull")
}

/// `(status, reason)` per result, for compact assertions.
fn outcomes(r: &SyncPushResult) -> Vec<(String, Option<String>)> {
    r.results
        .iter()
        .map(|m| {
            (
                serde_json::to_value(m.status)
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .to_string(),
                m.reason.clone(),
            )
        })
        .collect()
}

fn applied() -> (String, Option<String>) {
    ("applied".into(), None)
}

fn rejected(reason: &str) -> (String, Option<String>) {
    ("rejected".into(), Some(reason.into()))
}

fn deferred(reason: &str) -> (String, Option<String>) {
    ("deferred".into(), Some(reason.into()))
}

// --- Auth and limits ----------------------------------------------------------------------

#[sqlx::test]
async fn sync_requires_auth(db: PgPool) {
    let app = TestApp::new(db);
    app.call(Req::post("/v1/sync/push").json(push_body(vec![])))
        .await
        .assert_error(StatusCode::UNAUTHORIZED, "unauthorized");
    app.call(Req::get("/v1/sync/pull"))
        .await
        .assert_error(StatusCode::UNAUTHORIZED, "unauthorized");
}

#[sqlx::test]
async fn empty_push_is_ok(db: PgPool) {
    let app = TestApp::new(db);
    let s = app.sign_in("empty").await;
    assert!(push(&app, &s, vec![]).await.results.is_empty());
}

#[sqlx::test]
async fn more_than_100_mutations_is_413(db: PgPool) {
    let app = TestApp::new(db.clone());
    let s = app.sign_in("toomany").await;
    let too_many: Vec<Value> = (0..=MAX_MUTATIONS_PER_PUSH)
        .map(|_| mutation("settings_updated", json!({"sound": true})))
        .collect();
    app.call(
        Req::post("/v1/sync/push")
            .bearer(&s.access)
            .json(push_body(too_many)),
    )
    .await
    .assert_error(StatusCode::PAYLOAD_TOO_LARGE, "payload_too_large");
    assert_eq!(
        count(
            &db,
            "SELECT count(*) FROM sync_mutation WHERE user_id = $1",
            s.user_id
        )
        .await,
        0,
        "nothing applied"
    );
    // Exactly 100 is fine.
    let ok: Vec<Value> = (0..MAX_MUTATIONS_PER_PUSH)
        .map(|_| mutation("settings_updated", json!({"sound": true})))
        .collect();
    assert_eq!(
        push(&app, &s, ok).await.results.len(),
        MAX_MUTATIONS_PER_PUSH
    );
}

// --- game_finished ------------------------------------------------------------------------

#[sqlx::test]
async fn game_finished_is_applied_and_idempotent(db: PgPool) {
    let app = TestApp::new(db.clone());
    let s = app.sign_in("games").await;
    let record = played_record();
    let m = mutation("game_finished", serde_json::to_value(&record).unwrap());

    let first = push(&app, &s, vec![m.clone()]).await;
    assert_eq!(outcomes(&first), vec![applied()]);
    assert_eq!(first.results[0].id, m["id"].as_str().unwrap());

    // Same mutation again: same result, nothing duplicated.
    let again = push(&app, &s, vec![m.clone()]).await;
    assert_eq!(again, first);
    assert_eq!(
        count(
            &db,
            "SELECT count(*) FROM games WHERE user_id = $1",
            s.user_id
        )
        .await,
        1
    );
    assert_eq!(
        count(
            &db,
            "SELECT count(*) FROM sync_mutation WHERE user_id = $1",
            s.user_id
        )
        .await,
        1
    );

    // Same game under a new mutation id (e.g. re-queued by another device): union by game id.
    let other_mutation = mutation("game_finished", serde_json::to_value(&record).unwrap());
    assert_eq!(
        outcomes(&push(&app, &s, vec![other_mutation]).await),
        vec![applied()]
    );
    assert_eq!(
        count(
            &db,
            "SELECT count(*) FROM games WHERE user_id = $1",
            s.user_id
        )
        .await,
        1
    );

    // Stored columns match the record.
    let (moves, outcome, south, north): (Vec<i32>, String, i32, i32) = sqlx::query_as(
        "SELECT moves, result_outcome, store_south, store_north FROM games WHERE id = $1",
    )
    .bind(Uuid::parse_str(&record.id).unwrap())
    .fetch_one(&db)
    .await
    .unwrap();
    assert_eq!(
        moves,
        record
            .moves
            .iter()
            .map(|&m| i32::from(m))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        json!(outcome),
        serde_json::to_value(record.result.outcome).unwrap()
    );
    assert_eq!([south, north], record.result.stores.map(i32::from));
}

#[sqlx::test]
async fn concurrent_pushes_of_the_same_mutation_apply_once(db: PgPool) {
    let app = TestApp::new(db.clone());
    let s = app.sign_in("concurrent").await;
    let m = mutation(
        "game_finished",
        serde_json::to_value(played_record()).unwrap(),
    );
    let req = || {
        Req::post("/v1/sync/push")
            .bearer(&s.access)
            .json(push_body(vec![m.clone()]))
    };
    let (a, b) = tokio::join!(app.call(req()), app.call(req()));
    a.assert_status(StatusCode::OK);
    b.assert_status(StatusCode::OK);
    assert_eq!(a.json(), b.json());
    assert_eq!(a.json()["results"][0]["status"], "applied");
    assert_eq!(
        count(
            &db,
            "SELECT count(*) FROM games WHERE user_id = $1",
            s.user_id
        )
        .await,
        1
    );
}

#[sqlx::test]
async fn illegal_games_are_rejected_and_the_rejection_is_remembered(db: PgPool) {
    let app = TestApp::new(db.clone());
    let s = app.sign_in("cheater").await;
    let good = played_record();

    let mut unfinished = good.clone();
    unfinished.id = Uuid::now_v7().to_string();
    unfinished.moves.pop();

    let mut wrong_stores = good.clone();
    wrong_stores.id = Uuid::now_v7().to_string();
    wrong_stores.result.stores[0] = wrong_stores.result.stores[0].wrapping_add(1);

    let mut extra_move = good.clone();
    extra_move.id = Uuid::now_v7().to_string();
    extra_move.moves.push(0);

    let mut wrong_reason_outcome = good.clone();
    wrong_reason_outcome.id = Uuid::now_v7().to_string();
    wrong_reason_outcome.result.outcome = match good.result.outcome {
        ouril_engine::GameResult::Draw => ouril_engine::GameResult::SouthWins,
        _ => ouril_engine::GameResult::Draw,
    };

    let ms: Vec<Value> = [unfinished, wrong_stores, extra_move, wrong_reason_outcome]
        .iter()
        .map(|r| mutation("game_finished", serde_json::to_value(r).unwrap()))
        .collect();
    let first = push(&app, &s, ms.clone()).await;
    assert_eq!(outcomes(&first), vec![rejected("illegal_game"); 4]);
    assert_eq!(
        count(
            &db,
            "SELECT count(*) FROM games WHERE user_id = $1",
            s.user_id
        )
        .await,
        0,
        "rejected games are never stored"
    );
    // Retrying returns the same rejections.
    assert_eq!(push(&app, &s, ms).await, first);
}

#[sqlx::test]
async fn invalid_game_metadata_is_invalid_payload(db: PgPool) {
    let app = TestApp::new(db.clone());
    let s = app.sign_in("badmeta").await;
    let good = serde_json::to_value(played_record()).unwrap();
    let tampered = |f: &dyn Fn(&mut Value)| {
        let mut v = good.clone();
        v["id"] = json!(Uuid::now_v7().to_string());
        f(&mut v);
        mutation("game_finished", v)
    };
    let ms = vec![
        // A newer format or unknown variant@version is deferred, not invalid (see
        // records_from_newer_clients_are_deferred_then_applied_once_supported).
        tampered(&|v| v["format"] = json!(0)),
        tampered(&|v| v["id"] = json!("not-a-uuid")),
        tampered(&|v| v["started_at"] = json!("yesterday")),
        tampered(&|v| v["started_at"] = json!("2026-10-04T00:00:00Z")),
        tampered(&|v| v["ai_level"] = json!("impossible")),
        tampered(&|v| v["moves"] = json!(vec![0; 4097])),
        tampered(&|v| v["moves"] = json!("nope")),
        tampered(&|v| {
            v.as_object_mut().unwrap().remove("result");
        }),
        mutation("game_finished", json!(null)),
        mutation("game_finished", json!([1, 2, 3])),
    ];
    let n = ms.len();
    let r = push(&app, &s, ms).await;
    assert_eq!(outcomes(&r), vec![rejected("invalid_payload"); n]);
    assert_eq!(
        count(
            &db,
            "SELECT count(*) FROM games WHERE user_id = $1",
            s.user_id
        )
        .await,
        0
    );
}

#[sqlx::test]
async fn a_game_id_owned_by_someone_else_is_rejected(db: PgPool) {
    let app = TestApp::new(db.clone());
    let a = app.sign_in("owner").await;
    let b = app.sign_in("copycat").await;
    let record = serde_json::to_value(played_record()).unwrap();
    assert_eq!(
        outcomes(&push(&app, &a, vec![mutation("game_finished", record.clone())]).await),
        vec![applied()]
    );
    assert_eq!(
        outcomes(&push(&app, &b, vec![mutation("game_finished", record)]).await),
        vec![rejected("invalid_payload")]
    );
    assert_eq!(
        count(
            &db,
            "SELECT count(*) FROM games WHERE user_id = $1",
            b.user_id
        )
        .await,
        0
    );
    // B can't see A's game.
    let p = pull(&app, &b, "").await;
    assert!(p.changes.iter().all(|c| c.entity != "game"));
}

// --- Envelope validation ------------------------------------------------------------------

#[sqlx::test]
async fn unknown_types_and_schemas_are_deferred_and_bad_ids_rejected(db: PgPool) {
    let app = TestApp::new(db.clone());
    let s = app.sign_in("envelope").await;
    let unknown = mutation("board_reset", json!({}));
    let schema2 = mutation_with_id(
        &Uuid::now_v7().to_string(),
        "settings_updated",
        2,
        json!({"sound": true}),
    );
    let bad_id = mutation_with_id("not-a-uuid", "settings_updated", 1, json!({"sound": true}));
    let r = push(&app, &s, vec![unknown.clone(), schema2.clone(), bad_id]).await;
    assert_eq!(
        outcomes(&r),
        vec![
            deferred("unknown_type"),
            deferred("unsupported_schema"),
            rejected("invalid_payload"),
        ]
    );
    assert_eq!(r.results[2].id, "not-a-uuid", "results echo the id");
    // Deferrals are not recorded: the same ids are deferred again, not cached as rejected.
    let again = push(&app, &s, vec![unknown, schema2]).await;
    assert_eq!(
        outcomes(&again),
        vec![deferred("unknown_type"), deferred("unsupported_schema")]
    );
    assert_eq!(
        count(
            &db,
            "SELECT count(*) FROM sync_mutation WHERE user_id = $1",
            s.user_id
        )
        .await,
        0
    );
    // Nothing applied.
    assert_eq!(
        count(
            &db,
            "SELECT count(*) FROM user_settings WHERE user_id = $1",
            s.user_id
        )
        .await,
        0
    );
}

#[sqlx::test]
async fn mutation_ids_are_scoped_per_user(db: PgPool) {
    let app = TestApp::new(db.clone());
    let a = app.sign_in("scope_a").await;
    let b = app.sign_in("scope_b").await;
    let id = Uuid::now_v7().to_string();
    let rejected_for_a = mutation_with_id(&id, "settings_updated", 1, json!({"language": "!!"}));
    assert_eq!(
        outcomes(&push(&app, &a, vec![rejected_for_a]).await),
        vec![rejected("invalid_payload")]
    );
    // Same id from another user is a different mutation.
    let fine_for_b = mutation_with_id(&id, "settings_updated", 1, json!({"sound": true}));
    assert_eq!(
        outcomes(&push(&app, &b, vec![fine_for_b]).await),
        vec![applied()]
    );
}

#[sqlx::test]
async fn results_follow_request_order_and_mutations_are_independent(db: PgPool) {
    let app = TestApp::new(db.clone());
    let s = app.sign_in("ordering").await;
    let ms = vec![
        mutation("settings_updated", json!({"sound": false})),
        mutation("profile_updated", json!({"country": "not-a-country"})),
        mutation("profile_updated", json!({"display_name": "Nha Nome"})),
        mutation("nope", json!({})),
        mutation(
            "game_finished",
            serde_json::to_value(played_record()).unwrap(),
        ),
    ];
    let ids: Vec<String> = ms
        .iter()
        .map(|m| m["id"].as_str().unwrap().into())
        .collect();
    let r = push(&app, &s, ms).await;
    assert_eq!(
        r.results.iter().map(|m| m.id.clone()).collect::<Vec<_>>(),
        ids
    );
    assert_eq!(
        outcomes(&r),
        vec![
            applied(),
            rejected("invalid_payload"),
            applied(),
            deferred("unknown_type"),
            applied(),
        ]
    );
    let me = app.call(Req::get("/v1/me").bearer(&s.access)).await.json();
    assert_eq!(me["display_name"], "Nha Nome");
    assert_eq!(
        me["country"],
        Value::Null,
        "rejected mutation changed nothing"
    );
}

// --- profile / settings / handle ----------------------------------------------------------

#[sqlx::test]
async fn profile_and_settings_are_field_level_last_writer_wins(db: PgPool) {
    let app = TestApp::new(db.clone());
    let s = app.sign_in("lww").await;
    let r = push(
        &app,
        &s,
        vec![
            mutation("settings_updated", json!({"sound": true, "hints": true})),
            mutation(
                "settings_updated",
                json!({"sound": false, "language": "pt"}),
            ),
            mutation("settings_updated", json!({})),
            mutation(
                "profile_updated",
                json!({"display_name": "Ana", "locale": "pt"}),
            ),
            mutation("profile_updated", json!({"country": "CV"})),
        ],
    )
    .await;
    assert_eq!(outcomes(&r), vec![applied(); 5]);

    let p = pull(&app, &s, "").await;
    let settings = p.changes.iter().find(|c| c.entity == "settings").unwrap();
    assert_eq!(
        settings.data.as_ref().unwrap(),
        &json!({"sound": false, "hints": true, "language": "pt"})
    );
    assert_eq!(settings.id, s.user_id.to_string());
    let profile = p.changes.iter().find(|c| c.entity == "profile").unwrap();
    let data = profile.data.as_ref().unwrap();
    assert_eq!(data["display_name"], "Ana");
    assert_eq!(data["locale"], "pt");
    assert_eq!(data["country"], "cv");
    assert_eq!(profile.id, s.user_id.to_string());
}

#[sqlx::test]
async fn invalid_profile_and_settings_payloads_are_rejected(db: PgPool) {
    let app = TestApp::new(db);
    let s = app.sign_in("badprofile").await;
    let ms = vec![
        mutation("profile_updated", json!({"display_name": ""})),
        mutation(
            "profile_updated",
            json!({"avatar_url": "http://x.example/a.png"}),
        ),
        mutation("profile_updated", json!({"locale": "x"})),
        mutation("profile_updated", json!({"display_name": 5})),
        mutation("settings_updated", json!({"sound": "loud"})),
        mutation("settings_updated", json!({"language": "en_US"})),
        mutation("handle_requested", json!({"handle": "no"})),
        mutation("handle_requested", json!({})),
    ];
    let n = ms.len();
    assert_eq!(
        outcomes(&push(&app, &s, ms).await),
        vec![rejected("invalid_payload"); n]
    );
}

#[sqlx::test]
async fn handle_requested_applies_only_when_free(db: PgPool) {
    let app = TestApp::new(db);
    let a = app.sign_in("handle_owner").await;
    let b = app.sign_in("handle_wanter").await;
    assert_eq!(
        outcomes(
            &push(
                &app,
                &a,
                vec![mutation("handle_requested", json!({"handle": "@Morabeza"}))]
            )
            .await
        ),
        vec![applied()]
    );
    let me = app.call(Req::get("/v1/me").bearer(&a.access)).await.json();
    assert_eq!(me["handle"], "morabeza");

    let r = push(
        &app,
        &b,
        vec![
            mutation("handle_requested", json!({"handle": "MORABEZA"})),
            // The transaction for a rejected mutation must not poison the next one.
            mutation("handle_requested", json!({"handle": "sodade"})),
        ],
    )
    .await;
    assert_eq!(outcomes(&r), vec![rejected("handle_taken"), applied()]);
    let me = app.call(Req::get("/v1/me").bearer(&b.access)).await.json();
    assert_eq!(me["handle"], "sodade");
}

// --- Pull ---------------------------------------------------------------------------------

#[sqlx::test]
async fn pull_returns_changes_in_order_with_full_rows(db: PgPool) {
    let app = TestApp::new(db);
    let s = app.sign_in("puller").await;
    let g1 = played_record();
    let g2 = played_record();
    push(
        &app,
        &s,
        vec![
            mutation("game_finished", serde_json::to_value(&g1).unwrap()),
            mutation("settings_updated", json!({"sound": true})),
            mutation("game_finished", serde_json::to_value(&g2).unwrap()),
        ],
    )
    .await;

    let p = pull(&app, &s, "").await;
    assert!(!p.has_more);
    let entities: Vec<&str> = p.changes.iter().map(|c| c.entity.as_str()).collect();
    assert_eq!(entities, vec!["profile", "game", "settings", "game"]);
    let seqs: Vec<u64> = p.changes.iter().map(|c| c.change_seq).collect();
    assert!(seqs.windows(2).all(|w| w[0] < w[1]), "ordered: {seqs:?}");
    assert_eq!(p.cursor, *seqs.last().unwrap());

    // Game rows round-trip as the exact GameRecord.
    let game = p.changes.iter().find(|c| c.id == g1.id).unwrap();
    assert!(!game.deleted);
    let back: ouril_sync::GameRecord = serde_json::from_value(game.data.clone().unwrap()).unwrap();
    assert_eq!(back, g1);
    // Profile rows are `Me`.
    let profile = &p.changes[0];
    let me: ouril_protocol::Me = serde_json::from_value(profile.data.clone().unwrap()).unwrap();
    assert_eq!(me.id, s.user_id.to_string());

    // Nothing new after the cursor: empty, cursor unchanged.
    let next = pull(&app, &s, &format!("?cursor={}", p.cursor)).await;
    assert!(next.changes.is_empty());
    assert_eq!(next.cursor, p.cursor);
    assert!(!next.has_more);

    // A later change shows up after the cursor.
    push(
        &app,
        &s,
        vec![mutation("settings_updated", json!({"hints": false}))],
    )
    .await;
    let later = pull(&app, &s, &format!("?cursor={}", p.cursor)).await;
    assert_eq!(later.changes.len(), 1);
    assert_eq!(later.changes[0].entity, "settings");
    assert_eq!(
        later.changes[0].data.as_ref().unwrap(),
        &json!({"sound": true, "hints": false})
    );
    assert!(later.cursor > p.cursor);
}

#[sqlx::test]
async fn pull_pages_with_limit_and_has_more(db: PgPool) {
    let app = TestApp::new(db);
    let s = app.sign_in("pager").await;
    let games: Vec<Value> = (0..5)
        .map(|_| {
            mutation(
                "game_finished",
                serde_json::to_value(played_record()).unwrap(),
            )
        })
        .collect();
    push(&app, &s, games).await;
    let full = pull(&app, &s, "").await;
    assert_eq!(full.changes.len(), 6, "profile + 5 games");

    let mut cursor = 0;
    let mut seen = Vec::new();
    let mut pages = 0;
    loop {
        let page = pull(&app, &s, &format!("?cursor={cursor}&limit=2")).await;
        pages += 1;
        assert!(page.changes.len() <= 2);
        assert!(page.cursor >= cursor);
        seen.extend(page.changes.iter().map(|c| c.change_seq));
        cursor = page.cursor;
        if !page.has_more {
            break;
        }
        assert!(pages < 10, "paging did not terminate");
    }
    assert_eq!(pages, 3);
    assert_eq!(
        seen,
        full.changes
            .iter()
            .map(|c| c.change_seq)
            .collect::<Vec<_>>()
    );

    // Exact page boundary: limit = total -> has_more false.
    let exact = pull(&app, &s, "?limit=6").await;
    assert_eq!(exact.changes.len(), 6);
    assert!(!exact.has_more);
    let short = pull(&app, &s, "?limit=5").await;
    assert!(short.has_more);
}

#[sqlx::test]
async fn pull_validates_limit(db: PgPool) {
    let app = TestApp::new(db);
    let s = app.sign_in("limits").await;
    for q in [
        "?limit=0",
        "?limit=501",
        "?limit=-1",
        "?limit=abc",
        "?cursor=99999999999999999999",
    ] {
        app.call(Req::get(format!("/v1/sync/pull{q}")).bearer(&s.access))
            .await
            .assert_error(StatusCode::BAD_REQUEST, "bad_request");
    }
    for q in [
        "?limit=1",
        "?limit=500",
        "?cursor=0",
        "?cursor=9223372036854775807",
    ] {
        app.call(Req::get(format!("/v1/sync/pull{q}")).bearer(&s.access))
            .await
            .assert_status(StatusCode::OK);
    }
    // u64 cursor above i64::MAX is a 400, not a 500.
    app.call(Req::get("/v1/sync/pull?cursor=9223372036854775808").bearer(&s.access))
        .await
        .assert_error(StatusCode::BAD_REQUEST, "bad_request");
}

#[sqlx::test]
async fn pull_only_returns_the_callers_rows(db: PgPool) {
    let app = TestApp::new(db);
    let a = app.sign_in("mine").await;
    let b = app.sign_in("theirs").await;
    push(
        &app,
        &b,
        vec![
            mutation(
                "game_finished",
                serde_json::to_value(played_record()).unwrap(),
            ),
            mutation("settings_updated", json!({"sound": true})),
        ],
    )
    .await;
    let p = pull(&app, &a, "").await;
    assert_eq!(p.changes.len(), 1);
    assert_eq!(p.changes[0].entity, "profile");
    assert_eq!(p.changes[0].id, a.user_id.to_string());
}

#[sqlx::test]
async fn deleted_games_come_back_as_markers_without_data(db: PgPool) {
    // No API deletes games in the MVP; the tombstone shape is still part of the contract.
    let app = TestApp::new(db.clone());
    let s = app.sign_in("tombstone").await;
    let g = played_record();
    push(
        &app,
        &s,
        vec![mutation("game_finished", serde_json::to_value(&g).unwrap())],
    )
    .await;
    let before = pull(&app, &s, "").await;
    sqlx::query(
        "UPDATE games SET deleted_at = now(), change_seq = nextval('change_seq') WHERE id = $1",
    )
    .bind(Uuid::parse_str(&g.id).unwrap())
    .execute(&db)
    .await
    .unwrap();
    let after = pull(&app, &s, &format!("?cursor={}", before.cursor)).await;
    assert_eq!(after.changes.len(), 1);
    let c = &after.changes[0];
    assert_eq!(
        (c.entity.as_str(), c.id.as_str(), c.deleted),
        ("game", g.id.as_str(), true)
    );
    assert!(c.data.is_none());
    // On the wire `data` is omitted entirely.
    let raw = app
        .call(Req::get(format!("/v1/sync/pull?cursor={}", before.cursor)).bearer(&s.access))
        .await
        .json();
    assert!(raw["changes"][0].get("data").is_none(), "{raw}");
}

// --- Records from newer clients ------------------------------------------------------------

#[sqlx::test]
async fn records_from_newer_clients_are_deferred_then_applied_once_supported(db: PgPool) {
    let app = TestApp::new(db.clone());
    let s = app.sign_in("newer_client").await;
    let id = Uuid::now_v7().to_string();

    // A newer client pushes a game for a variant version this server doesn't know yet.
    let mut future = played_record();
    future.variant.version = 99;
    let m = mutation_with_id(
        &id,
        "game_finished",
        1,
        serde_json::to_value(&future).unwrap(),
    );
    assert_eq!(
        outcomes(&push(&app, &s, vec![m]).await),
        vec![deferred("unsupported_variant")]
    );
    let mut newer_format = played_record();
    newer_format.format = ouril_sync::GAME_RECORD_FORMAT + 1;
    let m = mutation_with_id(
        &Uuid::now_v7().to_string(),
        "game_finished",
        1,
        serde_json::to_value(&newer_format).unwrap(),
    );
    assert_eq!(
        outcomes(&push(&app, &s, vec![m]).await),
        vec![deferred("unsupported_record_format")]
    );
    // Nothing stored or recorded.
    assert_eq!(
        count(
            &db,
            "SELECT count(*) FROM games WHERE user_id = $1",
            s.user_id
        )
        .await,
        0
    );
    assert_eq!(
        count(
            &db,
            "SELECT count(*) FROM sync_mutation WHERE user_id = $1",
            s.user_id
        )
        .await,
        0
    );

    // Once the server supports it (simulated by a record it knows), the same mutation id is
    // applied: the deferral didn't burn the id.
    let supported = mutation_with_id(
        &id,
        "game_finished",
        1,
        serde_json::to_value(played_record()).unwrap(),
    );
    assert_eq!(
        outcomes(&push(&app, &s, vec![supported]).await),
        vec![applied()]
    );
}

// --- Settings ordering by device time ------------------------------------------------------

fn settings_at(client_time: &str, payload: Value) -> Value {
    let mut m = mutation("settings_updated", payload);
    m["client_time"] = json!(client_time);
    m
}

async fn settings_of(app: &TestApp, s: &Session) -> Value {
    let p = pull(app, s, "").await;
    p.changes
        .iter()
        .find(|c| c.entity == "settings")
        .and_then(|c| c.data.clone())
        .unwrap_or(Value::Null)
}

#[sqlx::test]
async fn settings_merge_by_device_time_not_arrival(db: PgPool) {
    let app = TestApp::new(db.clone());
    let s = app.sign_in("settings_order").await;

    // The newer change arrives first...
    let newer = settings_at("2026-10-04T10:00:00Z", json!({"sound": false}));
    // ...then an older one that was deferred or queued offline. It also sets a field the
    // newer change didn't touch.
    let older = settings_at(
        "2026-10-04T09:00:00Z",
        json!({"sound": true, "hints": true}),
    );
    assert_eq!(
        outcomes(&push(&app, &s, vec![newer]).await),
        vec![applied()]
    );
    assert_eq!(
        outcomes(&push(&app, &s, vec![older]).await),
        vec![applied()]
    );

    let settings = settings_of(&app, &s).await;
    assert_eq!(
        settings["sound"], false,
        "older change must not undo a newer one"
    );
    assert_eq!(settings["hints"], true, "untouched fields still apply");

    // The field times are never exposed by pull.
    assert!(settings.get("field_times").is_none());
}

#[sqlx::test]
async fn settings_from_a_future_clock_are_clamped_to_server_time(db: PgPool) {
    let app = TestApp::new(db.clone());
    let s = app.sign_in("settings_future").await;

    // A device whose clock is years ahead...
    let future = settings_at("2100-01-01T00:00:00Z", json!({"language": "pt"}));
    assert_eq!(
        outcomes(&push(&app, &s, vec![future]).await),
        vec![applied()]
    );
    // ...doesn't block a later change made "now" on a correct device.
    let now = time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap();
    let later = settings_at(&now, json!({"language": "en"}));
    assert_eq!(
        outcomes(&push(&app, &s, vec![later]).await),
        vec![applied()]
    );
    assert_eq!(settings_of(&app, &s).await["language"], "en");
}

// --- Profile ordering by device time --------------------------------------------------------

fn profile_at(kind: &str, client_time: &str, payload: Value) -> Value {
    let mut m = mutation(kind, payload);
    m["client_time"] = json!(client_time);
    m
}

#[sqlx::test]
async fn profile_merges_by_device_time_not_arrival(db: PgPool) {
    let app = TestApp::new(db.clone());
    let s = app.sign_in("profile_order").await;

    let newer = profile_at(
        "profile_updated",
        "2026-10-04T10:00:00Z",
        json!({"display_name": "Newer Name"}),
    );
    let older = profile_at(
        "profile_updated",
        "2026-10-04T09:00:00Z",
        json!({"display_name": "Older Name", "country": "CV"}),
    );
    assert_eq!(
        outcomes(&push(&app, &s, vec![newer, older]).await),
        vec![applied(), applied()]
    );
    let me = app.call(Req::get("/v1/me").bearer(&s.access)).await.json();
    assert_eq!(
        me["display_name"], "Newer Name",
        "older change must not undo a newer one"
    );
    assert_eq!(me["country"], "cv", "untouched fields still apply");
}

#[sqlx::test]
async fn a_direct_edit_beats_older_queued_profile_and_handle_changes(db: PgPool) {
    let app = TestApp::new(db.clone());
    let s = app.sign_in("profile_direct").await;

    // Edited directly (online) now...
    let r = app
        .call(Req::patch("/v1/me").bearer(&s.access).json(json!({
            "display_name": "Direct Edit",
            "handle": "direct_handle",
        })))
        .await;
    assert_eq!(r.status, StatusCode::OK);

    // ...then changes queued earlier on an offline device arrive.
    let queued = vec![
        profile_at(
            "profile_updated",
            "2026-10-01T08:00:00Z",
            json!({"display_name": "Queued Offline"}),
        ),
        profile_at(
            "handle_requested",
            "2026-10-01T08:00:00Z",
            json!({"handle": "queued_handle"}),
        ),
    ];
    assert_eq!(
        outcomes(&push(&app, &s, queued).await),
        vec![applied(), applied()]
    );
    let me = app.call(Req::get("/v1/me").bearer(&s.access)).await.json();
    assert_eq!(me["display_name"], "Direct Edit");
    assert_eq!(me["handle"], "direct_handle");
}
