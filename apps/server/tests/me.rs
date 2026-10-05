//! `GET / PATCH / DELETE /v1/me`.

mod common;

use axum::http::StatusCode;
use common::*;
use ouril_protocol::Me;
use serde_json::json;
use sqlx::PgPool;

#[sqlx::test]
async fn get_me_returns_the_profile(db: PgPool) {
    let app = TestApp::new(db);
    let s = app.sign_in("getme").await;
    let res = app.call(Req::get("/v1/me").bearer(&s.access)).await;
    res.assert_status(StatusCode::OK);
    let me: Me = serde_json::from_value(res.json()).unwrap();
    assert_eq!(me.id, s.user_id.to_string());
    assert_eq!(me.display_name, "Player getme");
    assert_eq!(me.handle, None);
    assert!(time::OffsetDateTime::parse(
        &me.created_at,
        &time::format_description::well_known::Rfc3339
    )
    .is_ok());

    app.call(Req::get("/v1/me"))
        .await
        .assert_error(StatusCode::UNAUTHORIZED, "unauthorized");
}

#[sqlx::test]
async fn patch_me_validates_and_normalizes(db: PgPool) {
    let app = TestApp::new(db);
    let s = app.sign_in("patch").await;
    let res = app
        .call(Req::patch("/v1/me").bearer(&s.access).json(json!({
            "display_name": "  Ana  ",
            "handle": "@Mindelo_Master",
            "avatar_url": "https://img.example/a.png",
            "locale": "pt-CV",
            "country": "CV",
            "unknown_field": "ignored (additive API)"
        })))
        .await;
    res.assert_status(StatusCode::OK);
    let me: Me = serde_json::from_value(res.json()).unwrap();
    assert_eq!(me.display_name, "Ana");
    assert_eq!(me.handle.as_deref(), Some("mindelo_master"));
    assert_eq!(me.avatar_url.as_deref(), Some("https://img.example/a.png"));
    assert_eq!(me.locale.as_deref(), Some("pt-CV"));
    assert_eq!(me.country.as_deref(), Some("cv"));

    // Absent fields are unchanged.
    let res = app
        .call(
            Req::patch("/v1/me")
                .bearer(&s.access)
                .json(json!({"locale": "kea"})),
        )
        .await;
    res.assert_status(StatusCode::OK);
    let me2: Me = serde_json::from_value(res.json()).unwrap();
    assert_eq!(me2.locale.as_deref(), Some("kea"));
    assert_eq!(me2.display_name, "Ana");
    assert_eq!(me2.handle.as_deref(), Some("mindelo_master"));

    // Empty patch is a no-op 200.
    let res = app
        .call(Req::patch("/v1/me").bearer(&s.access).json(json!({})))
        .await;
    res.assert_status(StatusCode::OK);
    assert_eq!(serde_json::from_value::<Me>(res.json()).unwrap(), me2);

    // GET reflects the patch.
    let got: Me =
        serde_json::from_value(app.call(Req::get("/v1/me").bearer(&s.access)).await.json())
            .unwrap();
    assert_eq!(got, me2);
}

#[sqlx::test]
async fn patch_me_rejects_invalid_fields(db: PgPool) {
    let app = TestApp::new(db);
    let s = app.sign_in("invalid").await;
    for body in [
        json!({"display_name": ""}),
        json!({"display_name": "   "}),
        json!({"display_name": "x".repeat(51)}),
        json!({"display_name": "a\u{0007}b"}),
        json!({"handle": "ab"}),
        json!({"handle": "has space"}),
        json!({"handle": "x".repeat(21)}),
        json!({"handle": "ção"}),
        json!({"avatar_url": "http://insecure.example/a.png"}),
        json!({"avatar_url": "javascript:alert(1)"}),
        json!({"avatar_url": format!("https://x.example/{}", "a".repeat(2048))}),
        json!({"locale": "e"}),
        json!({"locale": "en_US"}),
        json!({"country": "cpv"}),
        json!({"country": "1a"}),
    ] {
        let res = app
            .call(Req::patch("/v1/me").bearer(&s.access).json(body.clone()))
            .await;
        assert_eq!(
            res.status,
            StatusCode::BAD_REQUEST,
            "{body}: {}",
            res.text()
        );
        assert_eq!(res.error_code(), "bad_request");
    }
    // Nothing was written.
    let me: Me =
        serde_json::from_value(app.call(Req::get("/v1/me").bearer(&s.access)).await.json())
            .unwrap();
    assert_eq!(me.display_name, "Player invalid");
    assert_eq!(me.handle, None);
}

#[sqlx::test]
async fn handles_are_unique_ignoring_case(db: PgPool) {
    let app = TestApp::new(db);
    let a = app.sign_in("handle_a").await;
    let b = app.sign_in("handle_b").await;
    app.call(
        Req::patch("/v1/me")
            .bearer(&a.access)
            .json(json!({"handle": "tabanka"})),
    )
    .await
    .assert_status(StatusCode::OK);
    for taken in ["tabanka", "TABANKA", "@Tabanka"] {
        app.call(
            Req::patch("/v1/me")
                .bearer(&b.access)
                .json(json!({"handle": taken})),
        )
        .await
        .assert_error(StatusCode::CONFLICT, "handle_taken");
    }
    // A conflicting handle doesn't apply the rest of the patch either.
    app.call(
        Req::patch("/v1/me")
            .bearer(&b.access)
            .json(json!({"handle": "tabanka", "display_name": "Should not stick"})),
    )
    .await
    .assert_error(StatusCode::CONFLICT, "handle_taken");
    let me: Me =
        serde_json::from_value(app.call(Req::get("/v1/me").bearer(&b.access)).await.json())
            .unwrap();
    assert_eq!(me.display_name, "Player handle_b");

    // Re-setting your own handle (any case) is fine.
    app.call(
        Req::patch("/v1/me")
            .bearer(&a.access)
            .json(json!({"handle": "TaBanka"})),
    )
    .await
    .assert_status(StatusCode::OK);
}

#[sqlx::test]
async fn delete_me_removes_personal_data_and_revokes_sessions(db: PgPool) {
    let app = TestApp::new(db.clone());
    let s = app.sign_in("deleteme").await;
    let second_device = app.sign_in("deleteme").await;
    let bystander = app.sign_in("bystander").await;

    // Give the account some data: handle, settings, a game, sync history.
    app.call(Req::patch("/v1/me").bearer(&s.access).json(json!({
        "handle": "leaving", "country": "cv", "avatar_url": "https://img.example/x.png"
    })))
    .await
    .assert_status(StatusCode::OK);
    let game = serde_json::to_value(played_record()).unwrap();
    let res = app
        .call(
            Req::post("/v1/sync/push")
                .bearer(&s.access)
                .json(push_body(vec![
                    mutation("game_finished", game),
                    mutation("settings_updated", json!({"sound": false})),
                ])),
        )
        .await;
    res.assert_status(StatusCode::OK);
    // Bystander data must survive.
    app.call(
        Req::post("/v1/sync/push")
            .bearer(&bystander.access)
            .json(push_body(vec![mutation(
                "settings_updated",
                json!({"hints": true}),
            )])),
    )
    .await
    .assert_status(StatusCode::OK);

    let res = app.call(Req::delete("/v1/me").bearer(&s.access)).await;
    res.assert_status(StatusCode::NO_CONTENT);
    assert!(res.bytes.is_empty());
    assert!(
        res.set_cookies().is_empty(),
        "native caller: no cookie to clear"
    );

    // Every session is dead at once.
    for token in [&s.access, &second_device.access] {
        app.call(Req::get("/v1/me").bearer(token))
            .await
            .assert_error(StatusCode::UNAUTHORIZED, "unauthorized");
    }
    for refresh in [&s.refresh, &second_device.refresh] {
        app.call(Req::post("/v1/auth/refresh").json(json!({"refresh_token": refresh})))
            .await
            .assert_error(StatusCode::UNAUTHORIZED, "unauthorized");
    }
    app.call(
        Req::post("/v1/sync/push")
            .bearer(&s.access)
            .json(push_body(vec![])),
    )
    .await
    .assert_error(StatusCode::UNAUTHORIZED, "unauthorized");

    // Personal data is gone; the user row is an anonymized tombstone.
    let uid = s.user_id;
    for (table, sql) in [
        ("auth_identities", "SELECT count(*) FROM auth_identities WHERE user_id = $1"),
        ("user_settings", "SELECT count(*) FROM user_settings WHERE user_id = $1"),
        ("games", "SELECT count(*) FROM games WHERE user_id = $1"),
        ("sync_mutation", "SELECT count(*) FROM sync_mutation WHERE user_id = $1"),
        (
            "session_refresh_tokens",
            "SELECT count(*) FROM session_refresh_tokens t JOIN sessions s ON s.id = t.session_id WHERE s.user_id = $1",
        ),
        (
            "live sessions",
            "SELECT count(*) FROM sessions WHERE user_id = $1 AND revoked_at IS NULL",
        ),
    ] {
        assert_eq!(count(&db, sql, uid).await, 0, "{table} not cleaned");
    }
    let row: (
        String,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        bool,
    ) = sqlx::query_as(
        "SELECT display_name, handle, avatar_url, locale, country, deleted_at IS NOT NULL
             FROM users WHERE id = $1",
    )
    .bind(uid)
    .fetch_one(&db)
    .await
    .unwrap();
    assert_eq!(
        row,
        ("Deleted player".to_string(), None, None, None, None, true)
    );

    // Bystander unaffected.
    app.call(Req::get("/v1/me").bearer(&bystander.access))
        .await
        .assert_status(StatusCode::OK);
    assert_eq!(
        count(
            &db,
            "SELECT count(*) FROM user_settings WHERE user_id = $1",
            bystander.user_id
        )
        .await,
        1
    );

    // The handle is free again.
    app.call(
        Req::patch("/v1/me")
            .bearer(&bystander.access)
            .json(json!({"handle": "leaving"})),
    )
    .await
    .assert_status(StatusCode::OK);

    // Signing in again with the same identity creates a new, empty account.
    let again = app.sign_in("deleteme").await;
    assert_ne!(again.user_id, s.user_id);
    let pull = app
        .call(Req::get("/v1/sync/pull").bearer(&again.access))
        .await;
    pull.assert_status(StatusCode::OK);
    let changes = pull.json()["changes"].as_array().unwrap().clone();
    assert!(
        changes.iter().all(|c| c["entity"] == "profile"),
        "no games or settings carried over: {changes:?}"
    );
}

#[sqlx::test]
async fn delete_me_from_the_web_clears_the_cookie(db: PgPool) {
    let app = TestApp::new(db);
    let s = app.sign_in("webdelete").await;
    let res = app
        .call(Req::delete("/v1/me").origin(WEB_ORIGIN).bearer(&s.access))
        .await;
    res.assert_status(StatusCode::NO_CONTENT);
    assert_eq!(res.refresh_cookie().as_deref(), Some(""));
    // Second delete with the dead token: 401, not a crash.
    app.call(Req::delete("/v1/me").bearer(&s.access))
        .await
        .assert_error(StatusCode::UNAUTHORIZED, "unauthorized");
}

#[sqlx::test]
async fn delete_me_requires_auth(db: PgPool) {
    let app = TestApp::new(db);
    app.call(Req::delete("/v1/me"))
        .await
        .assert_error(StatusCode::UNAUTHORIZED, "unauthorized");
    app.call(Req::patch("/v1/me").json(json!({"display_name": "x"})))
        .await
        .assert_error(StatusCode::UNAUTHORIZED, "unauthorized");
}

#[sqlx::test]
async fn apple_tokens_are_read_for_revocation_and_removed_with_the_account(db: PgPool) {
    let app = TestApp::new(db.clone());
    let s = app.sign_in("apple_revoke").await;
    // An Apple identity with a stored refresh token (as a real Apple sign-in would leave it).
    sqlx::query(
        "INSERT INTO auth_identities (id, user_id, provider, provider_subject, email_is_private_relay)
         VALUES ($1, $2, 'apple', 'apple-sub-1', false)",
    )
    .bind(uuid::Uuid::now_v7())
    .bind(s.user_id)
    .execute(&db)
    .await
    .unwrap();
    ouril_server::auth::session::store_provider_token(
        &db,
        "apple",
        "apple-sub-1",
        "com.example.ouril.web",
        "apple-refresh-1",
    )
    .await
    .unwrap();
    assert_eq!(
        ouril_server::users::apple_tokens(&db, s.user_id)
            .await
            .unwrap(),
        vec![(
            "com.example.ouril.web".to_string(),
            "apple-refresh-1".to_string()
        )]
    );

    // Deleting the account still succeeds when Apple isn't configured (the failed revocation
    // is logged), and the stored token is gone with the identity.
    app.call(Req::delete("/v1/me").bearer(&s.access))
        .await
        .assert_status(StatusCode::NO_CONTENT);
    assert!(ouril_server::users::apple_tokens(&db, s.user_id)
        .await
        .unwrap()
        .is_empty());
}
