//! HTTP routes and middleware. See `apps/server/API.md` for the contract.

mod auth;
mod me;
mod meta;
mod sync;

use std::time::Duration;

use axum::{
    extract::{DefaultBodyLimit, Request},
    http::{header, HeaderName, HeaderValue, Method},
    middleware,
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use tower_http::{
    catch_panic::CatchPanicLayer,
    cors::{AllowOrigin, CorsLayer},
    trace::TraceLayer,
};

use crate::{client::ClientInfo, error::AppError, state::AppState};

/// Max request body (bytes). Sync push batches are the largest requests.
pub const MAX_BODY_BYTES: usize = 1024 * 1024;

/// Upper bound for one `/v1` request (slow database, stuck JWKS fetch). Dropping the handler
/// future releases its pool connection; an open transaction is rolled back.
pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);

/// `Strict-Transport-Security` sent by release builds (Fly terminates TLS in production).
/// Deliberately without `includeSubDomains`: browsers would keep forcing HTTPS on every
/// subdomain of the API's domain for a year, so add it (and consider `preload`) only once
/// every subdomain is confirmed to serve HTTPS (launch checklist in docs/review/human-review.md).
pub const HSTS: &str = "max-age=31536000";

pub fn router(state: AppState) -> Router {
    let v1 = Router::new()
        .route("/meta", get(meta::meta))
        .merge(auth::router())
        .merge(me::router())
        .merge(sync::router())
        .fallback(not_found)
        .layer(middleware::from_fn(request_timeout))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            crate::client::client_version_gate,
        ))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            crate::ratelimit::rate_limit,
        ));

    Router::new()
        .route("/healthz", get(healthz))
        .nest("/v1", v1)
        .fallback(not_found)
        .layer(DefaultBodyLimit::max(MAX_BODY_BYTES))
        .layer(cors(&state.config.allowed_origins))
        .layer(CatchPanicLayer::custom(|_| {
            tracing::error!("handler panicked");
            AppError::internal().into_response()
        }))
        .layer(TraceLayer::new_for_http().make_span_with(make_span))
        .layer(middleware::from_fn(security_headers))
        .with_state(state)
}

/// Request span: method, path (no query string), client version. Never headers or bodies,
/// so tokens and cookies can't reach the logs.
fn make_span(req: &Request) -> tracing::Span {
    let client = ClientInfo::from_headers(req.headers())
        .map(|c| c.label())
        .unwrap_or_else(|| "-".into());
    tracing::info_span!(
        "request",
        method = %req.method(),
        path = %req.uri().path(),
        client = %client,
        user_id = tracing::field::Empty,
    )
}

/// CORS: exact origins from `ALLOWED_ORIGINS`, credentials allowed (refresh cookie).
fn cors(origins: &[String]) -> CorsLayer {
    let origins: Vec<HeaderValue> = origins
        .iter()
        .filter_map(|o| HeaderValue::from_str(o).ok())
        .collect();
    CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_credentials(true)
        .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::DELETE])
        .allow_headers([
            header::AUTHORIZATION,
            header::CONTENT_TYPE,
            HeaderName::from_static("x-ouril-client"),
        ])
        .expose_headers([header::RETRY_AFTER])
        .max_age(Duration::from_secs(600))
}

/// Fail a `/v1` request that runs longer than [`REQUEST_TIMEOUT`] with the usual JSON error
/// body (500 `internal`; see API.md: a dedicated retryable code is an open contract question).
async fn request_timeout(req: Request, next: middleware::Next) -> Response {
    let path = req.uri().path().to_string();
    match tokio::time::timeout(REQUEST_TIMEOUT, next.run(req)).await {
        Ok(res) => res,
        Err(_) => {
            tracing::error!(%path, timeout_secs = REQUEST_TIMEOUT.as_secs(), "request timed out");
            AppError::internal().into_response()
        }
    }
}

/// Conservative headers for a JSON API. Release builds also pin HTTPS (HSTS); debug builds
/// don't, so a browser never pins plain-http localhost development.
async fn security_headers(req: Request, next: middleware::Next) -> Response {
    let mut res = next.run(req).await;
    let h = res.headers_mut();
    h.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    h.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    h.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("no-referrer"),
    );
    h.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static("default-src 'none'; frame-ancestors 'none'"),
    );
    if !cfg!(debug_assertions) {
        h.insert(
            header::STRICT_TRANSPORT_SECURITY,
            HeaderValue::from_static(HSTS),
        );
    }
    res
}

async fn healthz() -> &'static str {
    "ok"
}

async fn not_found() -> AppError {
    AppError::not_found()
}
