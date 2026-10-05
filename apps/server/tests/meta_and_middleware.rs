//! `/healthz`, `/v1/meta` and the cross-cutting middleware: 404 fallback, security headers,
//! CORS, `X-Ouril-Client` (426), rate limiting (429), body limits and JSON error bodies.

mod common;

use axum::http::{Method, StatusCode};
use common::*;
use serde_json::json;
use sqlx::PgPool;

#[sqlx::test]
async fn healthz_is_plain_ok(db: PgPool) {
    let app = TestApp::new(db);
    let res = app.call(Req::get("/healthz")).await;
    res.assert_status(StatusCode::OK);
    assert_eq!(res.text(), "ok");
}

#[sqlx::test]
async fn meta_reports_configured_versions(db: PgPool) {
    let app = TestApp::new(db);
    let res = app.call(Req::get("/v1/meta")).await;
    res.assert_status(StatusCode::OK);
    let v = res.json();
    assert_eq!(v["min_supported"]["web"], "0.1.0");
    assert_eq!(v["recommended"]["web"], "0.2.0");
    // Unset values default to 0.1.0.
    assert_eq!(v["min_supported"]["ios"], "0.1.0");
    assert_eq!(v["recommended"]["android"], "0.1.0");
    assert_eq!(v["api"], json!({ "current": "v1", "deprecated": [] }));
    assert_eq!(v["realtime_proto"], json!({ "current": 1, "min": 1 }));
    // Typed round-trip through the shared protocol type.
    let _: ouril_protocol::Meta = serde_json::from_value(v).expect("Meta shape");
}

#[sqlx::test]
async fn unknown_routes_are_json_404(db: PgPool) {
    let app = TestApp::new(db);
    for path in ["/nope", "/v1/nope", "/v1/auth/unknown", "/v2/meta"] {
        app.call(Req::get(path))
            .await
            .assert_error(StatusCode::NOT_FOUND, "not_found");
    }
}

#[sqlx::test]
async fn security_headers_on_every_response(db: PgPool) {
    let app = TestApp::new(db);
    for res in [
        app.call(Req::get("/healthz")).await,
        app.call(Req::get("/v1/meta")).await,
        app.call(Req::get("/v1/nope")).await,
        app.call(Req::get("/v1/me")).await,
    ] {
        assert_eq!(res.header("x-content-type-options"), Some("nosniff"));
        assert_eq!(res.header("cache-control"), Some("no-store"));
        assert_eq!(res.header("referrer-policy"), Some("no-referrer"));
        assert_eq!(
            res.header("content-security-policy"),
            Some("default-src 'none'; frame-ancestors 'none'")
        );
    }
}

#[sqlx::test]
async fn cors_allows_only_exact_origins_with_credentials(db: PgPool) {
    let app = TestApp::new(db);
    let preflight = |origin: &str| {
        Req::new(Method::OPTIONS, "/v1/sync/push")
            .origin(origin)
            .header("access-control-request-method", "POST")
            .header(
                "access-control-request-headers",
                "authorization,content-type,x-ouril-client",
            )
    };
    let ok = app.call(preflight(WEB_ORIGIN)).await;
    assert!(ok.status.is_success(), "preflight status {}", ok.status);
    assert_eq!(ok.header("access-control-allow-origin"), Some(WEB_ORIGIN));
    assert_eq!(ok.header("access-control-allow-credentials"), Some("true"));
    let allowed_headers = ok
        .header("access-control-allow-headers")
        .unwrap()
        .to_ascii_lowercase();
    for h in ["authorization", "content-type", "x-ouril-client"] {
        assert!(allowed_headers.contains(h), "{allowed_headers} lacks {h}");
    }

    let evil = app.call(preflight(EVIL_ORIGIN)).await;
    assert_eq!(evil.header("access-control-allow-origin"), None);
    // Prefix/suffix tricks must not match.
    let sneaky = app
        .call(preflight("http://localhost:5173.evil.example"))
        .await;
    assert_eq!(sneaky.header("access-control-allow-origin"), None);

    // Simple request from an allowed origin carries the CORS headers too.
    let meta = app.call(Req::get("/v1/meta").origin(HTTPS_ORIGIN)).await;
    assert_eq!(
        meta.header("access-control-allow-origin"),
        Some(HTTPS_ORIGIN)
    );
}

#[sqlx::test]
async fn old_clients_get_426_except_on_meta(db: PgPool) {
    let mut env = base_env();
    env.insert("MIN_SUPPORTED_WEB", "1.2.0".into());
    let app = TestApp::with_env(db, env);
    let old = "web/1.1.9 (7); core/0.1.0";

    // /v1/meta stays reachable so the client can learn it is too old.
    app.call(Req::get("/v1/meta").header("x-ouril-client", old))
        .await
        .assert_status(StatusCode::OK);
    for r in [
        Req::get("/v1/me"),
        Req::post("/v1/auth/google").json(json!({"id_token": "x", "nonce": "n"})),
        Req::post("/v1/auth/refresh").json(json!({})),
        Req::post("/v1/sync/push").json(json!({"mutations": []})),
        Req::get("/v1/sync/pull"),
    ] {
        app.call(r.header("x-ouril-client", old))
            .await
            .assert_error(StatusCode::UPGRADE_REQUIRED, "upgrade_required");
    }
    // At or above the minimum: not gated (401 because there's no token).
    for ok in ["web/1.2.0", "web/1.10.0 (1); core/0.1.0", "web/2.0.0-beta"] {
        app.call(Req::get("/v1/me").header("x-ouril-client", ok))
            .await
            .assert_error(StatusCode::UNAUTHORIZED, "unauthorized");
    }
    // Other platforms use their own minimum (0.1.0 default).
    app.call(Req::get("/v1/me").header("x-ouril-client", "ios/0.1.0 (1)"))
        .await
        .assert_error(StatusCode::UNAUTHORIZED, "unauthorized");
    // Missing, unparseable or unknown-platform headers are allowed (curl, health checks).
    for h in ["garbage", "web/abc", "tvos/0.0.1"] {
        app.call(Req::get("/v1/me").header("x-ouril-client", h))
            .await
            .assert_error(StatusCode::UNAUTHORIZED, "unauthorized");
    }
    // /healthz is outside /v1 and never gated.
    app.call(Req::get("/healthz").header("x-ouril-client", old))
        .await
        .assert_status(StatusCode::OK);
}

#[sqlx::test]
async fn auth_routes_are_rate_limited_with_retry_after(db: PgPool) {
    let app = TestApp::new(db);
    let req = || Req::post("/v1/auth/google").json(json!({"id_token": "x", "nonce": "n"}));
    for _ in 0..ouril_server::ratelimit::AUTH_LIMIT_PER_MINUTE {
        app.call(req())
            .await
            .assert_error(StatusCode::NOT_IMPLEMENTED, "not_configured");
    }
    let limited = app.call(req()).await;
    limited.assert_error(StatusCode::TOO_MANY_REQUESTS, "rate_limited");
    let retry: u64 = limited
        .header("retry-after")
        .expect("Retry-After")
        .parse()
        .expect("seconds");
    assert!((1..=60).contains(&retry), "retry-after {retry}");
    // Other route classes have their own budget.
    app.call(Req::get("/v1/meta"))
        .await
        .assert_status(StatusCode::OK);
    // /healthz is never rate limited.
    app.call(Req::get("/healthz"))
        .await
        .assert_status(StatusCode::OK);
}

#[sqlx::test]
async fn default_routes_are_rate_limited(db: PgPool) {
    let app = TestApp::new(db);
    for _ in 0..ouril_server::ratelimit::DEFAULT_LIMIT_PER_MINUTE {
        app.call(Req::get("/v1/meta"))
            .await
            .assert_status(StatusCode::OK);
    }
    app.call(Req::get("/v1/meta"))
        .await
        .assert_error(StatusCode::TOO_MANY_REQUESTS, "rate_limited");
}

#[sqlx::test]
async fn malformed_bodies_are_json_errors(db: PgPool) {
    let app = TestApp::new(db.clone());
    let s = app.sign_in("bodies").await;

    // Syntax error.
    app.call(Req::patch("/v1/me").bearer(&s.access).raw_json("{not json"))
        .await
        .assert_error(StatusCode::BAD_REQUEST, "bad_request");
    // Wrong types.
    app.call(
        Req::patch("/v1/me")
            .bearer(&s.access)
            .json(json!({"display_name": 42})),
    )
    .await
    .assert_error(StatusCode::BAD_REQUEST, "bad_request");
    // Missing content type.
    app.call(
        Req::patch("/v1/me")
            .bearer(&s.access)
            .raw(r#"{"display_name":"x"}"#),
    )
    .await
    .assert_error(StatusCode::BAD_REQUEST, "bad_request");
    // Missing required field.
    app.call(Req::post("/v1/sync/push").bearer(&s.access).json(json!({})))
        .await
        .assert_error(StatusCode::BAD_REQUEST, "bad_request");
    // Bad query string.
    app.call(Req::get("/v1/sync/pull?cursor=abc").bearer(&s.access))
        .await
        .assert_error(StatusCode::BAD_REQUEST, "bad_request");
    app.call(Req::get("/v1/sync/pull?cursor=-1").bearer(&s.access))
        .await
        .assert_error(StatusCode::BAD_REQUEST, "bad_request");
}

#[sqlx::test]
async fn bodies_over_one_mib_are_413(db: PgPool) {
    let app = TestApp::new(db);
    let s = app.sign_in("big").await;
    let big = format!(
        r#"{{"mutations":[],"pad":"{}"}}"#,
        "x".repeat(ouril_server::routes::MAX_BODY_BYTES + 1)
    );
    app.call(
        Req::post("/v1/sync/push")
            .bearer(&s.access)
            .raw_json(big.clone()),
    )
    .await
    .assert_error(StatusCode::PAYLOAD_TOO_LARGE, "payload_too_large");
    // Optional-body endpoints too.
    app.call(Req::post("/v1/auth/refresh").raw_json(big))
        .await
        .assert_error(StatusCode::PAYLOAD_TOO_LARGE, "payload_too_large");
}
