//! `POST /v1/auth/dev` (ADR 0017): present only in debug builds with the `dev-auth` feature.
//!
//! `cargo test -p ouril-server` checks the route is absent (404);
//! `cargo test -p ouril-server --features dev-auth` exercises the full sign-in flow.

mod common;

use axum::http::StatusCode;
use common::*;
use serde_json::json;
use sqlx::PgPool;

#[cfg(not(feature = "dev-auth"))]
#[sqlx::test]
async fn dev_sign_in_does_not_exist_without_the_feature(db: PgPool) {
    const { assert!(!ouril_server::DEV_AUTH_ENABLED) };
    let app = TestApp::new(db.clone());
    for r in [
        Req::post("/v1/auth/dev"),
        Req::post("/v1/auth/dev").json(json!({"user": "dev"})),
        Req::post("/v1/auth/dev").origin(WEB_ORIGIN).json(json!({})),
    ] {
        app.call(r)
            .await
            .assert_error(StatusCode::NOT_FOUND, "not_found");
    }
    let users: i64 = sqlx::query_scalar("SELECT count(*) FROM users")
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(users, 0, "no account was created");
}

#[cfg(feature = "dev-auth")]
mod enabled {
    use super::*;
    use ouril_protocol::SignInResponse;

    #[sqlx::test]
    async fn native_dev_sign_in_returns_tokens_in_the_body(db: PgPool) {
        const { assert!(ouril_server::DEV_AUTH_ENABLED) };
        let app = TestApp::new(db);
        let res = app
            .call(
                Req::post("/v1/auth/dev")
                    .header("x-ouril-client", "ios/0.1.0 (1); core/0.1.0")
                    .json(json!({"user": "alice", "display_name": "Alice"})),
            )
            .await;
        res.assert_status(StatusCode::OK);
        assert!(res.set_cookies().is_empty());
        let r: SignInResponse = serde_json::from_value(res.json()).expect("SignInResponse");
        assert!(r.is_new_user);
        assert_eq!(r.user.display_name, "Alice");
        assert_eq!(r.user.handle, None);
        assert!(r.user.created_at.ends_with('Z'), "{}", r.user.created_at);
        assert_eq!(r.tokens.token_type, "Bearer");
        assert_eq!(r.tokens.expires_in, 900);
        let refresh = r
            .tokens
            .refresh_token
            .clone()
            .expect("native refresh token");

        // Usable tokens.
        let me = app
            .call(Req::get("/v1/me").bearer(&r.tokens.access_token))
            .await;
        me.assert_status(StatusCode::OK);
        assert_eq!(me.json()["id"], r.user.id);
        app.call(Req::post("/v1/auth/refresh").json(json!({"refresh_token": refresh})))
            .await
            .assert_status(StatusCode::OK);

        // Second sign-in: same user, not new; display_name only applies on creation.
        let again: SignInResponse = serde_json::from_value(
            app.call(
                Req::post("/v1/auth/dev").json(json!({"user": "alice", "display_name": "Other"})),
            )
            .await
            .json(),
        )
        .unwrap();
        assert!(!again.is_new_user);
        assert_eq!(again.user.id, r.user.id);
        assert_eq!(again.user.display_name, "Alice");
    }

    #[sqlx::test]
    async fn dev_sign_in_defaults(db: PgPool) {
        let app = TestApp::new(db.clone());
        // Empty body and `{}` both mean user "dev", name "Dev Player".
        let a = app.call(Req::post("/v1/auth/dev")).await;
        a.assert_status(StatusCode::OK);
        assert_eq!(a.json()["user"]["display_name"], "Dev Player");
        let b = app.call(Req::post("/v1/auth/dev").json(json!({}))).await;
        b.assert_status(StatusCode::OK);
        assert_eq!(a.json()["user"]["id"], b.json()["user"]["id"]);
        let subject: String = sqlx::query_scalar(
            "SELECT provider_subject FROM auth_identities WHERE provider = 'dev'",
        )
        .fetch_one(&db)
        .await
        .unwrap();
        assert_eq!(subject, "dev");
    }

    #[sqlx::test]
    async fn dev_sign_in_validates_input(db: PgPool) {
        let app = TestApp::new(db);
        for body in [
            json!({"user": "Alice"}),
            json!({"user": ""}),
            json!({"user": "a".repeat(33)}),
            json!({"user": "bad user"}),
            json!({"display_name": ""}),
            json!({"display_name": "x".repeat(51)}),
        ] {
            app.call(Req::post("/v1/auth/dev").json(body.clone()))
                .await
                .assert_error(StatusCode::BAD_REQUEST, "bad_request");
        }
        app.call(Req::post("/v1/auth/dev").raw_json("{oops"))
            .await
            .assert_error(StatusCode::BAD_REQUEST, "bad_request");
    }

    #[sqlx::test]
    async fn web_dev_sign_in_sets_the_cookie(db: PgPool) {
        let app = TestApp::new(db);
        let res = app
            .call(
                Req::post("/v1/auth/dev")
                    .origin(WEB_ORIGIN)
                    .json(json!({"user": "web"})),
            )
            .await;
        res.assert_status(StatusCode::OK);
        let body = res.json();
        assert!(
            body["tokens"].get("refresh_token").is_none(),
            "web never gets the refresh token in JSON: {body}"
        );
        let cookie = res.refresh_cookie().expect("Set-Cookie ouril_refresh");
        assert_eq!(cookie.len(), 43);
        let raw = &res.set_cookies()[0];
        for attr in ["HttpOnly", "SameSite=Lax", "Path=/v1/auth"] {
            assert!(raw.contains(attr), "{raw} lacks {attr}");
        }
        assert!(!raw.contains("Secure"));

        // The cookie refreshes; the access token works.
        app.call(
            Req::post("/v1/auth/refresh")
                .origin(WEB_ORIGIN)
                .cookie(&cookie),
        )
        .await
        .assert_status(StatusCode::OK);
    }

    #[sqlx::test]
    async fn dev_sign_in_refuses_disallowed_origins(db: PgPool) {
        let app = TestApp::new(db.clone());
        app.call(
            Req::post("/v1/auth/dev")
                .origin(EVIL_ORIGIN)
                .json(json!({})),
        )
        .await
        .assert_error(StatusCode::FORBIDDEN, "forbidden");
        let sessions: i64 = sqlx::query_scalar("SELECT count(*) FROM sessions")
            .fetch_one(&db)
            .await
            .unwrap();
        assert_eq!(sessions, 0, "no session for a refused origin");
    }

    #[sqlx::test]
    async fn dev_sign_in_records_the_client_as_device_name(db: PgPool) {
        let app = TestApp::new(db.clone());
        app.call(
            Req::post("/v1/auth/dev")
                .header("x-ouril-client", "web/0.3.0 (12); core/0.1.0")
                .json(json!({})),
        )
        .await
        .assert_status(StatusCode::OK);
        let device: Option<String> = sqlx::query_scalar("SELECT device_name FROM sessions")
            .fetch_one(&db)
            .await
            .unwrap();
        assert_eq!(device.as_deref(), Some("web/0.3.0; core/0.1.0"));
    }

    #[sqlx::test]
    async fn signing_in_after_deletion_creates_a_new_account(db: PgPool) {
        let app = TestApp::new(db);
        let first: SignInResponse = serde_json::from_value(
            app.call(Req::post("/v1/auth/dev").json(json!({"user": "phoenix"})))
                .await
                .json(),
        )
        .unwrap();
        app.call(Req::delete("/v1/me").bearer(&first.tokens.access_token))
            .await
            .assert_status(StatusCode::NO_CONTENT);
        let second: SignInResponse = serde_json::from_value(
            app.call(Req::post("/v1/auth/dev").json(json!({"user": "phoenix"})))
                .await
                .json(),
        )
        .unwrap();
        assert!(second.is_new_user);
        assert_ne!(second.user.id, first.user.id);
    }
}
