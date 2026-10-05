//! `/v1/auth/*` and the bearer-token extractor: OAuth placeholders (501), refresh rotation and
//! reuse detection, web cookies vs native bodies, Origin checks, logout.
//!
//! These tests create sessions directly in the database, so they run with or without the
//! `dev-auth` feature. The dev sign-in endpoint itself is covered in `dev_auth.rs`.

mod common;

use axum::http::StatusCode;
use common::*;
use jsonwebtoken::{encode, EncodingKey, Header};
use ouril_server::auth::tokens::{AccessClaims, AUDIENCE, ISSUER};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

// --- OAuth placeholders -------------------------------------------------------------------

#[sqlx::test]
async fn google_and_apple_are_501_until_configured(db: PgPool) {
    let app = TestApp::new(db);
    let res = app
        .call(Req::post("/v1/auth/google").json(json!({"id_token": "tok", "nonce": "n"})))
        .await;
    res.assert_error(StatusCode::NOT_IMPLEMENTED, "not_configured");
    assert!(res.json()["message"].as_str().unwrap().contains("Google"));

    let res = app
        .call(Req::post("/v1/auth/apple").json(json!({
            "id_token": "tok", "nonce": "n", "given_name": "Ana", "family_name": "Lopes"
        })))
        .await;
    res.assert_error(StatusCode::NOT_IMPLEMENTED, "not_configured");
    assert!(res.json()["message"].as_str().unwrap().contains("Apple"));

    // Web clients get the same answer (and no cookie).
    let res = app
        .call(
            Req::post("/v1/auth/google")
                .origin(WEB_ORIGIN)
                .json(json!({"id_token": "tok", "nonce": "n"})),
        )
        .await;
    res.assert_error(StatusCode::NOT_IMPLEMENTED, "not_configured");
    assert!(res.set_cookies().is_empty());
}

#[sqlx::test]
async fn partial_apple_config_is_still_not_configured(db: PgPool) {
    let mut env = base_env();
    env.insert("APPLE_SERVICE_ID", "com.example.ouril.web".into());
    env.insert("APPLE_TEAM_ID", "TEAM123456".into());
    // APPLE_KEY_ID and APPLE_PRIVATE_KEY_PATH stay empty.
    let app = TestApp::with_env(db, env);
    assert!(app.state.apple.is_none());
    app.call(Req::post("/v1/auth/apple").json(json!({"id_token": "tok", "nonce": "n"})))
        .await
        .assert_error(StatusCode::NOT_IMPLEMENTED, "not_configured");
}

#[sqlx::test]
async fn configured_google_rejects_garbage_tokens_without_network(db: PgPool) {
    // With a client ID set the endpoint verifies the token. A malformed token fails before any
    // JWKS fetch, so this stays offline.
    let mut env = base_env();
    env.insert(
        "GOOGLE_CLIENT_ID",
        "test-client.apps.googleusercontent.com".into(),
    );
    let app = TestApp::with_env(db, env);
    app.call(Req::post("/v1/auth/google").json(json!({"id_token": "not-a-jwt", "nonce": "n"})))
        .await
        .assert_error(StatusCode::UNAUTHORIZED, "unauthorized");
    app.call(Req::post("/v1/auth/google").json(json!({"id_token": "a.b.c", "nonce": ""})))
        .await
        .assert_error(StatusCode::UNAUTHORIZED, "unauthorized");
}

#[sqlx::test]
async fn placeholder_sign_in_validates_the_body_first(db: PgPool) {
    let app = TestApp::new(db);
    app.call(Req::post("/v1/auth/google").json(json!({"nonce": "n"})))
        .await
        .assert_error(StatusCode::BAD_REQUEST, "bad_request");
}

// --- Bearer extractor ---------------------------------------------------------------------

#[sqlx::test]
async fn bearer_token_is_required_and_verified(db: PgPool) {
    let app = TestApp::new(db);
    let s = app.sign_in("bearer").await;

    app.call(Req::get("/v1/me").bearer(&s.access))
        .await
        .assert_status(StatusCode::OK);
    // Scheme is case-insensitive.
    app.call(Req::get("/v1/me").header("authorization", format!("bearer {}", s.access)))
        .await
        .assert_status(StatusCode::OK);

    for bad in [
        None,
        Some("Bearer".to_string()),
        Some("Bearer ".to_string()),
        Some(format!("Basic {}", s.access)),
        Some("Bearer not.a.jwt".to_string()),
        Some(format!("Bearer {}x", s.access)),
    ] {
        let mut r = Req::get("/v1/me");
        if let Some(v) = bad {
            r = r.header("authorization", v);
        }
        app.call(r)
            .await
            .assert_error(StatusCode::UNAUTHORIZED, "unauthorized");
    }
    // The refresh token is not an access token.
    app.call(Req::get("/v1/me").bearer(&s.refresh))
        .await
        .assert_error(StatusCode::UNAUTHORIZED, "unauthorized");
}

fn forge(secret: &str, claims: &AccessClaims) -> String {
    encode(
        &Header::default(),
        claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .unwrap()
}

#[sqlx::test]
async fn forged_or_expired_access_tokens_are_rejected(db: PgPool) {
    let app = TestApp::new(db);
    let s = app.sign_in("forged").await;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let good = AccessClaims {
        sub: s.user_id.to_string(),
        sid: s.session_id.to_string(),
        iat: now,
        exp: now + 600,
        iss: ISSUER.into(),
        aud: AUDIENCE.into(),
    };
    // Sanity: a correctly signed token works.
    app.call(Req::get("/v1/me").bearer(&forge(JWT_SECRET, &good)))
        .await
        .assert_status(StatusCode::OK);

    let other_secret = "some-other-secret-some-other-secret-xx";
    let cases = [
        forge(other_secret, &good),
        forge(
            JWT_SECRET,
            &AccessClaims {
                exp: now - 120,
                iat: now - 1000,
                ..good.clone()
            },
        ),
        forge(
            JWT_SECRET,
            &AccessClaims {
                iss: "someone-else".into(),
                ..good.clone()
            },
        ),
        forge(
            JWT_SECRET,
            &AccessClaims {
                aud: "other-api".into(),
                ..good.clone()
            },
        ),
        // A real user with a session ID that doesn't exist.
        forge(
            JWT_SECRET,
            &AccessClaims {
                sid: Uuid::now_v7().to_string(),
                ..good.clone()
            },
        ),
        // Session of user A presented as user B.
        forge(
            JWT_SECRET,
            &AccessClaims {
                sub: Uuid::now_v7().to_string(),
                ..good.clone()
            },
        ),
    ];
    for (i, token) in cases.iter().enumerate() {
        let res = app.call(Req::get("/v1/me").bearer(token)).await;
        assert_eq!(
            res.status,
            StatusCode::UNAUTHORIZED,
            "case {i}: {}",
            res.text()
        );
    }
}

#[sqlx::test]
async fn expired_or_revoked_session_invalidates_access_token(db: PgPool) {
    let app = TestApp::new(db.clone());
    let s = app.sign_in("expiring").await;
    sqlx::query("UPDATE sessions SET expires_at = now() - interval '1 second' WHERE id = $1")
        .bind(s.session_id)
        .execute(&db)
        .await
        .unwrap();
    app.call(Req::get("/v1/me").bearer(&s.access))
        .await
        .assert_error(StatusCode::UNAUTHORIZED, "unauthorized");
    // ...and the refresh token no longer works either.
    app.call(Req::post("/v1/auth/refresh").json(json!({"refresh_token": s.refresh})))
        .await
        .assert_error(StatusCode::UNAUTHORIZED, "unauthorized");
}

// --- Refresh (native) ---------------------------------------------------------------------

#[sqlx::test]
async fn native_refresh_rotates_the_token(db: PgPool) {
    let app = TestApp::new(db.clone());
    let s = app.sign_in("native").await;

    let res = app
        .call(Req::post("/v1/auth/refresh").json(json!({"refresh_token": s.refresh})))
        .await;
    res.assert_status(StatusCode::OK);
    assert!(res.set_cookies().is_empty(), "native clients get no cookie");
    let pair: ouril_protocol::TokenPair = serde_json::from_value(res.json()).unwrap();
    assert_eq!(pair.token_type, "Bearer");
    assert_eq!(pair.expires_in, 900);
    let new_refresh = pair
        .refresh_token
        .expect("native gets refresh_token in body");
    assert_ne!(new_refresh, s.refresh);
    assert_eq!(new_refresh.len(), 43);

    // The new access token belongs to the same user and session.
    let (user, session) = app.state.tokens.verify(&pair.access_token).unwrap();
    assert_eq!((user, session), (s.user_id, s.session_id));
    app.call(Req::get("/v1/me").bearer(&pair.access_token))
        .await
        .assert_status(StatusCode::OK);

    // The new refresh token rotates again.
    let res = app
        .call(Req::post("/v1/auth/refresh").json(json!({"refresh_token": new_refresh})))
        .await;
    res.assert_status(StatusCode::OK);

    // Only hashes are stored: the clear token never appears in the database.
    let stored: Vec<Vec<u8>> =
        sqlx::query_scalar("SELECT token_hash FROM session_refresh_tokens WHERE session_id = $1")
            .bind(s.session_id)
            .fetch_all(&db)
            .await
            .unwrap();
    assert_eq!(stored.len(), 3, "one row per issued token");
    for h in &stored {
        assert_eq!(h.len(), 32, "SHA-256");
        assert_ne!(h.as_slice(), s.refresh.as_bytes());
    }
}

#[sqlx::test]
async fn refresh_extends_the_session(db: PgPool) {
    let app = TestApp::new(db.clone());
    let s = app.sign_in("sliding").await;
    sqlx::query("UPDATE sessions SET expires_at = now() + interval '1 day' WHERE id = $1")
        .bind(s.session_id)
        .execute(&db)
        .await
        .unwrap();
    app.call(Req::post("/v1/auth/refresh").json(json!({"refresh_token": s.refresh})))
        .await
        .assert_status(StatusCode::OK);
    let days_left: f64 = sqlx::query_scalar(
        "SELECT EXTRACT(EPOCH FROM (expires_at - now()))::float8 / 86400 FROM sessions WHERE id = $1",
    )
    .bind(s.session_id)
    .fetch_one(&db)
    .await
    .unwrap();
    assert!(
        (29.9..=30.1).contains(&days_left),
        "expires in {days_left} days"
    );
}

#[sqlx::test]
async fn reusing_a_rotated_refresh_token_revokes_the_session(db: PgPool) {
    let app = TestApp::new(db.clone());
    let s = app.sign_in("reuse").await;
    let other = app.sign_in("bystander").await;

    let first = app
        .call(Req::post("/v1/auth/refresh").json(json!({"refresh_token": s.refresh})))
        .await;
    first.assert_status(StatusCode::OK);
    let pair = first.json();
    let new_refresh = pair["tokens"]["refresh_token"]
        .as_str()
        .or(pair["refresh_token"].as_str())
        .unwrap()
        .to_string();
    let new_access = pair["access_token"].as_str().unwrap().to_string();

    // Replay of the old token: reuse detected.
    app.call(Req::post("/v1/auth/refresh").json(json!({"refresh_token": s.refresh})))
        .await
        .assert_error(StatusCode::UNAUTHORIZED, "unauthorized");

    // The whole session is revoked: the newest refresh token and its access token are dead.
    app.call(Req::post("/v1/auth/refresh").json(json!({"refresh_token": new_refresh})))
        .await
        .assert_error(StatusCode::UNAUTHORIZED, "unauthorized");
    app.call(Req::get("/v1/me").bearer(&new_access))
        .await
        .assert_error(StatusCode::UNAUTHORIZED, "unauthorized");
    let revoked: bool =
        sqlx::query_scalar("SELECT revoked_at IS NOT NULL FROM sessions WHERE id = $1")
            .bind(s.session_id)
            .fetch_one(&db)
            .await
            .unwrap();
    assert!(revoked);

    // Other users' sessions are untouched.
    app.call(Req::get("/v1/me").bearer(&other.access))
        .await
        .assert_status(StatusCode::OK);
}

#[sqlx::test]
async fn other_sessions_of_the_same_user_survive_reuse(db: PgPool) {
    let app = TestApp::new(db);
    let a = app.sign_in("twodevices").await;
    let b = app.sign_in("twodevices").await;
    assert_eq!(a.user_id, b.user_id);
    assert_ne!(a.session_id, b.session_id);
    app.call(Req::post("/v1/auth/refresh").json(json!({"refresh_token": a.refresh})))
        .await
        .assert_status(StatusCode::OK);
    app.call(Req::post("/v1/auth/refresh").json(json!({"refresh_token": a.refresh})))
        .await
        .assert_status(StatusCode::UNAUTHORIZED);
    app.call(Req::get("/v1/me").bearer(&b.access))
        .await
        .assert_status(StatusCode::OK);
}

#[sqlx::test]
async fn refresh_rejects_missing_unknown_and_malformed_tokens(db: PgPool) {
    let app = TestApp::new(db);
    let unknown = "A".repeat(43);
    for body in [
        None,
        Some(json!({})),
        Some(json!({"refresh_token": unknown})),
        Some(json!({"refresh_token": "short"})),
        Some(json!({"refresh_token": "!".repeat(43)})),
    ] {
        let mut r = Req::post("/v1/auth/refresh");
        if let Some(b) = body {
            r = r.json(b);
        }
        app.call(r)
            .await
            .assert_error(StatusCode::UNAUTHORIZED, "unauthorized");
    }
    // Malformed JSON is a 400, not a 401.
    app.call(Req::post("/v1/auth/refresh").raw_json("{nope"))
        .await
        .assert_error(StatusCode::BAD_REQUEST, "bad_request");
}

#[sqlx::test]
async fn concurrent_refreshes_with_one_token_issue_at_most_one_pair(db: PgPool) {
    // Two tabs refreshing at once: strict reuse detection (documented in API.md) means one wins
    // and the other revokes the session. Never two valid new tokens.
    let app = TestApp::new(db);
    let s = app.sign_in("race").await;
    let req = || Req::post("/v1/auth/refresh").json(json!({"refresh_token": s.refresh}));
    let (a, b) = tokio::join!(app.call(req()), app.call(req()));
    let oks = [a.status, b.status]
        .iter()
        .filter(|s| **s == StatusCode::OK)
        .count();
    assert!(
        oks <= 1,
        "both refreshes succeeded: {} / {}",
        a.text(),
        b.text()
    );
}

// --- Web (cookie) flow --------------------------------------------------------------------

#[sqlx::test]
async fn web_refresh_uses_the_httponly_cookie(db: PgPool) {
    let app = TestApp::new(db);
    let s = app.sign_in("webcookie").await;

    let res = app
        .call(
            Req::post("/v1/auth/refresh")
                .origin(WEB_ORIGIN)
                .cookie(&s.refresh)
                .json(json!({})),
        )
        .await;
    res.assert_status(StatusCode::OK);
    let body = res.json();
    assert!(
        body.get("refresh_token").is_none(),
        "never in the web body: {body}"
    );
    assert!(body["access_token"].is_string());
    let cookies = res.set_cookies();
    assert_eq!(cookies.len(), 1, "{cookies:?}");
    let c = &cookies[0];
    for attr in [
        "HttpOnly",
        "SameSite=Lax",
        "Path=/v1/auth",
        "Max-Age=2592000",
    ] {
        assert!(c.contains(attr), "{c} lacks {attr}");
    }
    assert!(!c.contains("Secure"), "no Secure on http://localhost: {c}");
    let rotated = res.refresh_cookie().unwrap();
    assert_ne!(rotated, s.refresh);
    assert_eq!(rotated.len(), 43);

    // No body at all also works for the web (cookie only).
    let res = app
        .call(
            Req::post("/v1/auth/refresh")
                .origin(WEB_ORIGIN)
                .cookie(&rotated),
        )
        .await;
    res.assert_status(StatusCode::OK);
}

#[sqlx::test]
async fn https_origins_get_secure_cookies(db: PgPool) {
    let app = TestApp::new(db);
    let s = app.sign_in("secure").await;
    let res = app
        .call(
            Req::post("/v1/auth/refresh")
                .origin(HTTPS_ORIGIN)
                .cookie(&s.refresh),
        )
        .await;
    res.assert_status(StatusCode::OK);
    let c = &res.set_cookies()[0];
    assert!(c.contains("; Secure"), "{c}");
}

#[sqlx::test]
async fn cookie_without_allowed_origin_is_refused(db: PgPool) {
    let app = TestApp::new(db.clone());
    let s = app.sign_in("csrf").await;

    // No Origin: treated as native, the cookie is ignored.
    app.call(
        Req::post("/v1/auth/refresh")
            .cookie(&s.refresh)
            .json(json!({})),
    )
    .await
    .assert_error(StatusCode::UNAUTHORIZED, "unauthorized");
    // Disallowed Origin: 403 on refresh and logout.
    app.call(
        Req::post("/v1/auth/refresh")
            .origin(EVIL_ORIGIN)
            .cookie(&s.refresh),
    )
    .await
    .assert_error(StatusCode::FORBIDDEN, "forbidden");
    app.call(
        Req::post("/v1/auth/logout")
            .origin(EVIL_ORIGIN)
            .cookie(&s.refresh),
    )
    .await
    .assert_error(StatusCode::FORBIDDEN, "forbidden");
    app.call(
        Req::post("/v1/auth/refresh")
            .origin("null")
            .cookie(&s.refresh),
    )
    .await
    .assert_error(StatusCode::FORBIDDEN, "forbidden");

    // None of that consumed or revoked the token.
    app.call(Req::post("/v1/auth/refresh").json(json!({"refresh_token": s.refresh})))
        .await
        .assert_status(StatusCode::OK);
}

#[sqlx::test]
async fn body_token_takes_precedence_over_cookie(db: PgPool) {
    let app = TestApp::new(db);
    let a = app.sign_in("precedence_a").await;
    let b = app.sign_in("precedence_b").await;
    let res = app
        .call(
            Req::post("/v1/auth/refresh")
                .origin(WEB_ORIGIN)
                .cookie(&a.refresh)
                .json(json!({"refresh_token": b.refresh})),
        )
        .await;
    res.assert_status(StatusCode::OK);
    let access = res.json()["access_token"].as_str().unwrap().to_string();
    let (user, _) = app.state.tokens.verify(&access).unwrap();
    assert_eq!(user, b.user_id);
    // Web client: still a cookie, never a body token.
    assert!(res.json().get("refresh_token").is_none());
    assert!(res.refresh_cookie().is_some());
}

#[sqlx::test]
async fn failed_web_refresh_clears_the_cookie(db: PgPool) {
    let app = TestApp::new(db);
    let res = app
        .call(
            Req::post("/v1/auth/refresh")
                .origin(WEB_ORIGIN)
                .cookie(&"B".repeat(43)),
        )
        .await;
    res.assert_error(StatusCode::UNAUTHORIZED, "unauthorized");
    let c = res.set_cookies().join("\n");
    assert!(
        c.contains("ouril_refresh=;") && c.contains("Max-Age=0"),
        "{c}"
    );
}

// --- Logout -------------------------------------------------------------------------------

#[sqlx::test]
async fn native_logout_revokes_the_session(db: PgPool) {
    let app = TestApp::new(db);
    let s = app.sign_in("logout").await;
    let res = app
        .call(Req::post("/v1/auth/logout").json(json!({"refresh_token": s.refresh})))
        .await;
    res.assert_status(StatusCode::NO_CONTENT);
    assert!(res.bytes.is_empty());
    assert!(res.set_cookies().is_empty());
    app.call(Req::get("/v1/me").bearer(&s.access))
        .await
        .assert_error(StatusCode::UNAUTHORIZED, "unauthorized");
    app.call(Req::post("/v1/auth/refresh").json(json!({"refresh_token": s.refresh})))
        .await
        .assert_error(StatusCode::UNAUTHORIZED, "unauthorized");
    // Idempotent.
    app.call(Req::post("/v1/auth/logout").json(json!({"refresh_token": s.refresh})))
        .await
        .assert_status(StatusCode::NO_CONTENT);
}

#[sqlx::test]
async fn logout_with_a_rotated_token_still_revokes(db: PgPool) {
    let app = TestApp::new(db);
    let s = app.sign_in("logout_old").await;
    let res = app
        .call(Req::post("/v1/auth/refresh").json(json!({"refresh_token": s.refresh})))
        .await;
    let new_access = res.json()["access_token"].as_str().unwrap().to_string();
    app.call(Req::post("/v1/auth/logout").json(json!({"refresh_token": s.refresh})))
        .await
        .assert_status(StatusCode::NO_CONTENT);
    app.call(Req::get("/v1/me").bearer(&new_access))
        .await
        .assert_error(StatusCode::UNAUTHORIZED, "unauthorized");
}

#[sqlx::test]
async fn logout_is_idempotent_for_unknown_or_missing_tokens(db: PgPool) {
    let app = TestApp::new(db);
    for r in [
        Req::post("/v1/auth/logout"),
        Req::post("/v1/auth/logout").json(json!({})),
        Req::post("/v1/auth/logout").json(json!({"refresh_token": "C".repeat(43)})),
        Req::post("/v1/auth/logout").json(json!({"refresh_token": "garbage"})),
    ] {
        app.call(r).await.assert_status(StatusCode::NO_CONTENT);
    }
}

#[sqlx::test]
async fn web_logout_clears_the_cookie(db: PgPool) {
    let app = TestApp::new(db);
    let s = app.sign_in("weblogout").await;
    let res = app
        .call(
            Req::post("/v1/auth/logout")
                .origin(WEB_ORIGIN)
                .cookie(&s.refresh),
        )
        .await;
    res.assert_status(StatusCode::NO_CONTENT);
    assert_eq!(res.refresh_cookie().as_deref(), Some(""));
    assert!(res.set_cookies()[0].contains("Max-Age=0"));
    app.call(
        Req::post("/v1/auth/refresh")
            .origin(WEB_ORIGIN)
            .cookie(&s.refresh),
    )
    .await
    .assert_error(StatusCode::UNAUTHORIZED, "unauthorized");
}
