//! `user_opponents`: who has played against whom, for "people you've played against" and
//! friends later. Nothing writes to it from the API yet; these tests cover the storage.

mod common;

use common::*;
use ouril_server::opponents::{recent_opponents, record_game_between};
use sqlx::PgPool;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

fn at(s: &str) -> OffsetDateTime {
    OffsetDateTime::parse(s, &Rfc3339).unwrap()
}

async fn count(db: &PgPool) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM user_opponents")
        .fetch_one(db)
        .await
        .unwrap()
}

#[sqlx::test]
async fn a_game_is_recorded_for_both_players(db: PgPool) {
    let app = TestApp::new(db.clone());
    let (ana, rui) = (app.sign_in("ana").await, app.sign_in("rui").await);
    let first = at("2026-10-01T10:00:00Z");
    record_game_between(&db, ana.user_id, rui.user_id, first)
        .await
        .unwrap();

    for (me, them) in [(ana.user_id, rui.user_id), (rui.user_id, ana.user_id)] {
        let list = recent_opponents(&db, me, 10).await.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].opponent_id, them);
        assert_eq!(list[0].games_played, 1);
        assert_eq!(list[0].first_played_at, first);
        assert_eq!(list[0].last_played_at, first);
    }
}

#[sqlx::test]
async fn more_games_count_up_and_widen_the_dates_even_out_of_order(db: PgPool) {
    let app = TestApp::new(db.clone());
    let (ana, rui) = (app.sign_in("ana").await, app.sign_in("rui").await);
    let (early, mid, late) = (
        at("2026-09-01T10:00:00Z"),
        at("2026-10-01T10:00:00Z"),
        at("2026-10-05T10:00:00Z"),
    );
    for at in [mid, late, early] {
        record_game_between(&db, rui.user_id, ana.user_id, at)
            .await
            .unwrap();
    }
    let o = &recent_opponents(&db, ana.user_id, 10).await.unwrap()[0];
    assert_eq!(o.games_played, 3);
    assert_eq!(o.first_played_at, early);
    assert_eq!(o.last_played_at, late);
    assert_eq!(count(&db).await, 2, "one row per player");
}

#[sqlx::test]
async fn opponents_come_most_recent_first(db: PgPool) {
    let app = TestApp::new(db.clone());
    let me = app.sign_in("me").await;
    let (a, b, c) = (
        app.sign_in("a").await,
        app.sign_in("b").await,
        app.sign_in("c").await,
    );
    record_game_between(&db, me.user_id, a.user_id, at("2026-10-01T10:00:00Z"))
        .await
        .unwrap();
    record_game_between(&db, me.user_id, b.user_id, at("2026-10-03T10:00:00Z"))
        .await
        .unwrap();
    record_game_between(&db, me.user_id, c.user_id, at("2026-10-02T10:00:00Z"))
        .await
        .unwrap();
    let ids: Vec<_> = recent_opponents(&db, me.user_id, 2)
        .await
        .unwrap()
        .into_iter()
        .map(|o| o.opponent_id)
        .collect();
    assert_eq!(ids, vec![b.user_id, c.user_id]);
}

#[sqlx::test]
async fn nobody_is_their_own_opponent(db: PgPool) {
    let app = TestApp::new(db.clone());
    let ana = app.sign_in("ana").await;
    let err = record_game_between(&db, ana.user_id, ana.user_id, OffsetDateTime::now_utc())
        .await
        .expect_err("self-play is refused");
    assert!(err.to_string().contains("user_opponents_not_self"), "{err}");
    assert_eq!(count(&db).await, 0);
}

#[sqlx::test]
async fn deleting_an_account_removes_its_list_but_not_others(db: PgPool) {
    let app = TestApp::new(db.clone());
    let (ana, rui) = (app.sign_in("ana").await, app.sign_in("rui").await);
    record_game_between(&db, ana.user_id, rui.user_id, OffsetDateTime::now_utc())
        .await
        .unwrap();
    ouril_server::users::delete_account(&db, ana.user_id)
        .await
        .unwrap();
    assert!(recent_opponents(&db, ana.user_id, 10)
        .await
        .unwrap()
        .is_empty());
    // Rui keeps the game, against the anonymized "Deleted player".
    let rui_list = recent_opponents(&db, rui.user_id, 10).await.unwrap();
    assert_eq!(rui_list.len(), 1);
    assert_eq!(rui_list[0].opponent_id, ana.user_id);
}
