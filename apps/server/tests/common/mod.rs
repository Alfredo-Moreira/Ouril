//! Shared helpers for the `ouril-server` integration tests.
//!
//! Every test gets its own database from `#[sqlx::test]` (migrations from
//! `apps/server/migrations` are applied automatically) and its own [`TestApp`], so rate-limit
//! counters and data never leak between tests. Requests go straight into the router with
//! `tower::ServiceExt::oneshot`: no sockets, no network.
//!
//! Run (Docker-first): `docker compose run --rm toolbox cargo test -p ouril-server`
//! (add `--features dev-auth` for the dev sign-in tests). Needs the compose `db` service.

#![allow(dead_code)] // Not every test binary uses every helper.

use std::collections::HashMap;

use axum::{
    body::Body,
    http::{header, HeaderMap, HeaderValue, Method, Request, StatusCode},
    Router,
};
use http_body_util::BodyExt;
use ouril_engine::{apply_move, legal_moves, new_game, variant_version, MoveEvent, Player, Status};
use ouril_server::{
    auth::session::{self, Identity},
    config::Config,
    AppState,
};
use ouril_sync::{GameMode, GameRecord, GameRecordResult, VariantRef, GAME_RECORD_FORMAT_RULES};
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

pub const JWT_SECRET: &str = "integration-test-secret-integration-test-secret";
pub const WEB_ORIGIN: &str = "http://localhost:5173";
pub const HTTPS_ORIGIN: &str = "https://ouril.example";
pub const EVIL_ORIGIN: &str = "https://evil.example";

/// Base configuration: OAuth placeholders empty (not configured), two allowed origins.
pub fn base_env() -> HashMap<&'static str, String> {
    HashMap::from([
        ("DATABASE_URL", "postgres://unused-in-tests".to_string()),
        ("JWT_SECRET", JWT_SECRET.to_string()),
        ("ALLOWED_ORIGINS", format!("{WEB_ORIGIN},{HTTPS_ORIGIN}")),
        ("GOOGLE_CLIENT_ID", String::new()),
        ("APPLE_SERVICE_ID", String::new()),
        ("APPLE_TEAM_ID", String::new()),
        ("APPLE_KEY_ID", String::new()),
        ("APPLE_PRIVATE_KEY_PATH", String::new()),
        ("MIN_SUPPORTED_WEB", "0.1.0".to_string()),
        ("RECOMMENDED_WEB", "0.2.0".to_string()),
    ])
}

pub fn config_from(env: &HashMap<&'static str, String>) -> Config {
    Config::from_lookup(|k| env.get(k).cloned()).expect("test config is valid")
}

pub struct TestApp {
    pub state: AppState,
    pub router: Router,
    pub db: PgPool,
}

impl TestApp {
    pub fn new(db: PgPool) -> Self {
        Self::with_env(db, base_env())
    }

    pub fn with_env(db: PgPool, env: HashMap<&'static str, String>) -> Self {
        let state = AppState::new(config_from(&env), db.clone());
        let router = ouril_server::app(state.clone());
        TestApp { state, router, db }
    }

    pub async fn send(&self, req: Request<Body>) -> TestResponse {
        let res = self
            .router
            .clone()
            .oneshot(req)
            .await
            .expect("router is infallible");
        let status = res.status();
        let headers = res.headers().clone();
        let bytes = res
            .into_body()
            .collect()
            .await
            .expect("body")
            .to_bytes()
            .to_vec();
        TestResponse {
            status,
            headers,
            bytes,
        }
    }

    /// Convenience: build and send a request.
    pub async fn call(&self, r: Req) -> TestResponse {
        self.send(r.build()).await
    }

    /// A signed-in user created directly in the database (works with or without `dev-auth`).
    pub async fn sign_in(&self, subject: &str) -> Session {
        let identity = Identity {
            provider: "dev",
            subject: subject.to_string(),
            email: None,
            email_is_private_relay: false,
            display_name: Some(format!("Player {subject}")),
        };
        let (user_id, _) = session::find_or_create_user(&self.db, &identity)
            .await
            .expect("create user");
        let (session_id, refresh) = session::create_session(&self.db, user_id, Some("test"), 30)
            .await
            .expect("create session");
        let access = self.state.tokens.issue(user_id, session_id).expect("jwt");
        Session {
            user_id,
            session_id,
            access,
            refresh,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Session {
    pub user_id: Uuid,
    pub session_id: Uuid,
    pub access: String,
    pub refresh: String,
}

/// Request builder.
pub struct Req {
    method: Method,
    uri: String,
    headers: Vec<(String, String)>,
    body: Option<Vec<u8>>,
    json: bool,
}

impl Req {
    pub fn new(method: Method, uri: impl Into<String>) -> Self {
        Req {
            method,
            uri: uri.into(),
            headers: Vec::new(),
            body: None,
            json: false,
        }
    }
    pub fn get(uri: impl Into<String>) -> Self {
        Self::new(Method::GET, uri)
    }
    pub fn post(uri: impl Into<String>) -> Self {
        Self::new(Method::POST, uri)
    }
    pub fn patch(uri: impl Into<String>) -> Self {
        Self::new(Method::PATCH, uri)
    }
    pub fn delete(uri: impl Into<String>) -> Self {
        Self::new(Method::DELETE, uri)
    }
    pub fn header(mut self, k: &str, v: impl Into<String>) -> Self {
        self.headers.push((k.to_string(), v.into()));
        self
    }
    pub fn bearer(self, token: &str) -> Self {
        self.header("authorization", format!("Bearer {token}"))
    }
    pub fn origin(self, origin: &str) -> Self {
        self.header("origin", origin)
    }
    pub fn cookie(self, refresh: &str) -> Self {
        self.header("cookie", format!("ouril_refresh={refresh}"))
    }
    pub fn json(mut self, v: Value) -> Self {
        self.body = Some(serde_json::to_vec(&v).unwrap());
        self.json = true;
        self
    }
    /// Raw body with `Content-Type: application/json`.
    pub fn raw_json(mut self, bytes: impl Into<Vec<u8>>) -> Self {
        self.body = Some(bytes.into());
        self.json = true;
        self
    }
    /// Raw body without a content type.
    pub fn raw(mut self, bytes: impl Into<Vec<u8>>) -> Self {
        self.body = Some(bytes.into());
        self
    }
    pub fn build(self) -> Request<Body> {
        let mut b = Request::builder().method(self.method).uri(self.uri);
        if self.json {
            b = b.header(header::CONTENT_TYPE, "application/json");
        }
        for (k, v) in self.headers {
            b = b.header(k, v);
        }
        b.body(self.body.map(Body::from).unwrap_or_else(Body::empty))
            .unwrap()
    }
}

#[derive(Debug)]
pub struct TestResponse {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub bytes: Vec<u8>,
}

impl TestResponse {
    pub fn json(&self) -> Value {
        serde_json::from_slice(&self.bytes).unwrap_or_else(|e| {
            panic!(
                "body is not JSON ({e}): {:?}",
                String::from_utf8_lossy(&self.bytes)
            )
        })
    }
    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.bytes).into_owned()
    }
    /// The `ApiError.code` of an error response (also checks the body shape).
    pub fn error_code(&self) -> String {
        let v = self.json();
        assert!(v["message"].is_string(), "ApiError needs a message: {v}");
        v["code"].as_str().expect("ApiError.code").to_string()
    }
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name).and_then(|v| v.to_str().ok())
    }
    pub fn set_cookies(&self) -> Vec<String> {
        self.headers
            .get_all(header::SET_COOKIE)
            .iter()
            .filter_map(|v: &HeaderValue| v.to_str().ok().map(str::to_string))
            .collect()
    }
    /// The `ouril_refresh` value from `Set-Cookie`, if any (empty string = cleared).
    pub fn refresh_cookie(&self) -> Option<String> {
        self.set_cookies().into_iter().find_map(|c| {
            c.strip_prefix("ouril_refresh=")
                .map(|rest| rest.split(';').next().unwrap_or("").to_string())
        })
    }
    #[track_caller]
    pub fn assert_status(&self, expected: StatusCode) -> &Self {
        assert_eq!(
            self.status,
            expected,
            "unexpected status; body: {}",
            self.text()
        );
        self
    }
    /// Assert an `ApiError` with this status and code.
    #[track_caller]
    pub fn assert_error(&self, status: StatusCode, code: &str) {
        self.assert_status(status);
        assert_eq!(self.error_code(), code, "body: {}", self.text());
        assert_eq!(
            self.header("content-type"),
            Some("application/json"),
            "errors are JSON"
        );
    }
}

/// A complete, legal cv.standard@1 game played with the engine (always the first legal move),
/// with its result taken from the engine itself. Game rules come only from the engine.
pub fn played_record() -> GameRecord {
    let v = variant_version("cv.standard", 1).expect("cv.standard@1");
    let mut s = new_game(v, Player::South);
    let mut moves = Vec::new();
    let mut reason = None;
    while s.status == Status::Playing {
        let m = legal_moves(v, &s)[0];
        let r = apply_move(v, &s, m).expect("legal");
        moves.push(m);
        for e in &r.events {
            if let MoveEvent::GameOver { reason: why, .. } = e {
                reason = Some(*why);
            }
        }
        s = r.state;
        assert!(moves.len() < 4096, "game did not end");
    }
    let outcome = match s.status {
        Status::Won(Player::South) => ouril_engine::GameResult::SouthWins,
        Status::Won(Player::North) => ouril_engine::GameResult::NorthWins,
        _ => ouril_engine::GameResult::Draw,
    };
    GameRecord {
        format: GAME_RECORD_FORMAT_RULES,
        id: Uuid::now_v7().to_string(),
        variant: VariantRef {
            id: "cv.standard".into(),
            version: 1,
        },
        first_player: Player::South,
        moves,
        core_version: "0.1.0".into(),
        result: GameRecordResult {
            outcome,
            stores: s.stores,
            reason: reason.expect("game over event").into(),
        },
        started_at: "2026-10-03T18:20:00Z".into(),
        ended_at: "2026-10-03T18:31:00Z".into(),
        mode: GameMode::VsAi,
        ai_level: Some("easy".into()),
        human_player: Some(Player::South),
    }
}

/// A mutation JSON object with a fresh UUIDv7 id.
pub fn mutation(kind: &str, payload: Value) -> Value {
    mutation_with_id(&Uuid::now_v7().to_string(), kind, 1, payload)
}

pub fn mutation_with_id(id: &str, kind: &str, schema: u16, payload: Value) -> Value {
    json!({
        "id": id,
        "type": kind,
        "schema": schema,
        "payload": payload,
        "client_time": "2026-10-03T18:31:00Z",
    })
}

pub fn push_body(mutations: Vec<Value>) -> Value {
    json!({ "mutations": mutations })
}

pub async fn count(db: &PgPool, sql: &'static str, user_id: Uuid) -> i64 {
    sqlx::query_scalar(sql)
        .bind(user_id)
        .fetch_one(db)
        .await
        .expect("count query")
}
