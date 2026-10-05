//! Error type for handlers: every non-2xx response is an `ApiError` JSON body.
//!
//! Internal errors are logged server-side with their cause; the client only sees a generic
//! `internal` message (no SQL, no stack traces, no secrets).

use axum::{
    body::Bytes,
    extract::{
        rejection::{JsonRejection, QueryRejection},
        FromRequest, FromRequestParts, OptionalFromRequest, Request,
    },
    http::{header, request::Parts, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use ouril_protocol::{ApiError, ErrorCode};
use serde::de::DeserializeOwned;

#[derive(Debug)]
pub struct AppError {
    pub status: StatusCode,
    pub body: ApiError,
    /// Seconds for a `Retry-After` header (429).
    pub retry_after: Option<u64>,
}

impl AppError {
    pub fn new(status: StatusCode, code: ErrorCode, message: impl Into<String>) -> Self {
        AppError {
            status,
            body: ApiError::new(code, message),
            retry_after: None,
        }
    }

    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, ErrorCode::BadRequest, message)
    }

    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self::new(StatusCode::UNAUTHORIZED, ErrorCode::Unauthorized, message)
    }

    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::new(StatusCode::FORBIDDEN, ErrorCode::Forbidden, message)
    }

    pub fn not_found() -> Self {
        Self::new(StatusCode::NOT_FOUND, ErrorCode::NotFound, "not found")
    }

    pub fn handle_taken() -> Self {
        Self::new(
            StatusCode::CONFLICT,
            ErrorCode::HandleTaken,
            "that handle is already taken",
        )
    }

    pub fn payload_too_large(message: impl Into<String>) -> Self {
        Self::new(
            StatusCode::PAYLOAD_TOO_LARGE,
            ErrorCode::PayloadTooLarge,
            message,
        )
    }

    pub fn upgrade_required(message: impl Into<String>) -> Self {
        Self::new(
            StatusCode::UPGRADE_REQUIRED,
            ErrorCode::UpgradeRequired,
            message,
        )
    }

    pub fn rate_limited(retry_after_secs: u64) -> Self {
        let mut e = Self::new(
            StatusCode::TOO_MANY_REQUESTS,
            ErrorCode::RateLimited,
            "too many requests, slow down",
        );
        e.retry_after = Some(retry_after_secs.max(1));
        e
    }

    /// 501 for providers whose credentials aren't configured (OAuth placeholders).
    pub fn not_configured(what: &str) -> Self {
        Self::new(
            StatusCode::NOT_IMPLEMENTED,
            ErrorCode::NotConfigured,
            format!("{what} is not configured on this server"),
        )
    }

    /// 503 when a provider we depend on (Google/Apple signing keys) can't be reached.
    pub fn provider_unavailable(what: &str) -> Self {
        Self::new(
            StatusCode::SERVICE_UNAVAILABLE,
            ErrorCode::ProviderUnavailable,
            format!("{what} is temporarily unavailable, try again later"),
        )
    }

    /// 500. Log the cause before calling this; the client sees a generic message.
    pub fn internal() -> Self {
        Self::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            ErrorCode::Internal,
            "internal server error",
        )
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let mut res = (self.status, Json(self.body)).into_response();
        if let Some(secs) = self.retry_after {
            if let Ok(v) = HeaderValue::from_str(&secs.to_string()) {
                res.headers_mut().insert(header::RETRY_AFTER, v);
            }
        }
        res
    }
}

impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        tracing::error!(error = %e, "database error");
        AppError::internal()
    }
}

pub type AppResult<T> = Result<T, AppError>;

/// `Json<T>` whose rejections are `ApiError` JSON (400 `bad_request`, 413
/// `payload_too_large`) instead of axum's plain-text bodies.
#[derive(Debug, Clone, Copy, Default)]
pub struct ApiJson<T>(pub T);

impl<T, S> FromRequest<S> for ApiJson<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        match <Json<T> as FromRequest<S>>::from_request(req, state).await {
            Ok(Json(v)) => Ok(ApiJson(v)),
            Err(rejection) => Err(json_rejection(rejection)),
        }
    }
}

/// `Option<ApiJson<T>>`: an empty (or whitespace-only) body is `None`, anything else must be
/// valid JSON for `T`. Used where the body is optional (`/v1/auth/refresh` with `{}` or nothing).
impl<T, S> OptionalFromRequest<S> for ApiJson<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request(req: Request, state: &S) -> Result<Option<Self>, Self::Rejection> {
        let bytes = Bytes::from_request(req, state).await.map_err(|r| {
            if r.status() == StatusCode::PAYLOAD_TOO_LARGE {
                AppError::payload_too_large("request body is larger than 1 MiB")
            } else {
                AppError::bad_request("invalid request body")
            }
        })?;
        if bytes.iter().all(u8::is_ascii_whitespace) {
            return Ok(None);
        }
        serde_json::from_slice(&bytes)
            .map(|v| Some(ApiJson(v)))
            .map_err(|_| AppError::bad_request("malformed JSON body"))
    }
}

fn json_rejection(r: JsonRejection) -> AppError {
    if r.status() == StatusCode::PAYLOAD_TOO_LARGE {
        return AppError::payload_too_large("request body is larger than 1 MiB");
    }
    match r {
        JsonRejection::MissingJsonContentType(_) => {
            AppError::bad_request("expected Content-Type: application/json")
        }
        // serde's message names the field and position, never echoes secrets back.
        JsonRejection::JsonDataError(e) => AppError::bad_request(e.body_text()),
        JsonRejection::JsonSyntaxError(_) => AppError::bad_request("malformed JSON body"),
        _ => AppError::bad_request("invalid request body"),
    }
}

/// `Query<T>` with `ApiError` rejections.
#[derive(Debug, Clone, Copy, Default)]
pub struct ApiQuery<T>(pub T);

impl<T, S> FromRequestParts<S> for ApiQuery<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        match axum::extract::Query::<T>::from_request_parts(parts, state).await {
            Ok(axum::extract::Query(v)) => Ok(ApiQuery(v)),
            Err(QueryRejection::FailedToDeserializeQueryString(e)) => {
                Err(AppError::bad_request(e.body_text()))
            }
            Err(_) => Err(AppError::bad_request("invalid query string")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, routing::post, Router};
    use http_body_util::BodyExt;
    use serde_json::Value;
    use tower::ServiceExt;

    async fn parts(res: Response) -> (StatusCode, Option<String>, Value) {
        let status = res.status();
        let retry = res
            .headers()
            .get(header::RETRY_AFTER)
            .map(|v| v.to_str().unwrap().to_string());
        let bytes = res.into_body().collect().await.unwrap().to_bytes();
        (status, retry, serde_json::from_slice(&bytes).unwrap())
    }

    #[tokio::test]
    async fn constructors_map_to_status_and_code() {
        let cases = [
            (AppError::bad_request("x"), 400, "bad_request"),
            (AppError::unauthorized("x"), 401, "unauthorized"),
            (AppError::forbidden("x"), 403, "forbidden"),
            (AppError::not_found(), 404, "not_found"),
            (AppError::handle_taken(), 409, "handle_taken"),
            (AppError::payload_too_large("x"), 413, "payload_too_large"),
            (AppError::upgrade_required("x"), 426, "upgrade_required"),
            (AppError::rate_limited(5), 429, "rate_limited"),
            (
                AppError::not_configured("Google sign-in"),
                501,
                "not_configured",
            ),
            (AppError::internal(), 500, "internal"),
        ];
        for (err, status, code) in cases {
            let (s, retry, body) = parts(err.into_response()).await;
            assert_eq!(s.as_u16(), status, "{code}");
            assert_eq!(body["code"], code);
            assert!(body["message"].as_str().is_some_and(|m| !m.is_empty()));
            assert_eq!(retry.is_some(), code == "rate_limited", "{code}");
        }
    }

    #[tokio::test]
    async fn retry_after_is_at_least_one_second() {
        let (_, retry, _) = parts(AppError::rate_limited(0).into_response()).await;
        assert_eq!(retry.as_deref(), Some("1"));
        let (_, retry, _) = parts(AppError::rate_limited(42).into_response()).await;
        assert_eq!(retry.as_deref(), Some("42"));
    }

    #[tokio::test]
    async fn not_configured_names_the_provider() {
        let (_, _, body) = parts(AppError::not_configured("Apple sign-in").into_response()).await;
        assert_eq!(
            body["message"],
            "Apple sign-in is not configured on this server"
        );
    }

    #[tokio::test]
    async fn database_errors_become_generic_500() {
        let err: AppError =
            sqlx::Error::Protocol("secret detail: SELECT * FROM users".into()).into();
        let (s, _, body) = parts(err.into_response()).await;
        assert_eq!(s, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body["code"], "internal");
        assert!(!body.to_string().contains("secret detail"), "{body}");
    }

    #[derive(serde::Deserialize)]
    #[allow(dead_code)]
    struct Body1 {
        name: String,
    }

    fn app() -> Router {
        Router::new()
            .route(
                "/json",
                post(|ApiJson(b): ApiJson<Body1>| async move { b.name }),
            )
            .route(
                "/optional",
                post(|b: Option<ApiJson<Body1>>| async move {
                    b.map(|ApiJson(b)| b.name).unwrap_or_else(|| "none".into())
                }),
            )
            .layer(axum::extract::DefaultBodyLimit::max(64))
    }

    async fn send(path: &str, ct: Option<&str>, body: &str) -> (StatusCode, String) {
        let mut req = Request::builder().method("POST").uri(path);
        if let Some(ct) = ct {
            req = req.header(header::CONTENT_TYPE, ct);
        }
        let res = app()
            .oneshot(req.body(Body::from(body.to_string())).unwrap())
            .await
            .unwrap();
        let status = res.status();
        let bytes = res.into_body().collect().await.unwrap().to_bytes();
        (status, String::from_utf8_lossy(&bytes).into_owned())
    }

    fn code(body: &str) -> String {
        serde_json::from_str::<Value>(body).unwrap()["code"]
            .as_str()
            .unwrap()
            .to_string()
    }

    #[tokio::test]
    async fn api_json_rejections_are_api_errors() {
        const JSON: Option<&str> = Some("application/json");
        assert_eq!(
            send("/json", JSON, r#"{"name":"ok"}"#).await,
            (StatusCode::OK, "ok".into())
        );
        for (ct, body) in [
            (JSON, "{nope"),
            (JSON, r#"{"name":1}"#),
            (JSON, "{}"),
            (None, r#"{"name":"ok"}"#),
            (Some("text/plain"), r#"{"name":"ok"}"#),
        ] {
            let (s, b) = send("/json", ct, body).await;
            assert_eq!(s, StatusCode::BAD_REQUEST, "{ct:?} {body}: {b}");
            assert_eq!(code(&b), "bad_request");
        }
        let big = format!(r#"{{"name":"{}"}}"#, "x".repeat(100));
        let (s, b) = send("/json", JSON, &big).await;
        assert_eq!(s, StatusCode::PAYLOAD_TOO_LARGE);
        assert_eq!(code(&b), "payload_too_large");
    }

    #[tokio::test]
    async fn optional_api_json() {
        assert_eq!(
            send("/optional", None, "").await,
            (StatusCode::OK, "none".into())
        );
        assert_eq!(
            send("/optional", None, "  \n").await,
            (StatusCode::OK, "none".into())
        );
        assert_eq!(
            send("/optional", None, r#"{"name":"a"}"#).await,
            (StatusCode::OK, "a".into())
        );
        let (s, b) = send("/optional", None, "{bad").await;
        assert_eq!(
            (s, code(&b).as_str()),
            (StatusCode::BAD_REQUEST, "bad_request")
        );
        let big = format!(r#"{{"name":"{}"}}"#, "x".repeat(100));
        let (s, b) = send("/optional", None, &big).await;
        assert_eq!(
            (s, code(&b).as_str()),
            (StatusCode::PAYLOAD_TOO_LARGE, "payload_too_large")
        );
    }

    #[tokio::test]
    async fn api_query_rejections_are_api_errors() {
        #[derive(serde::Deserialize)]
        struct Q {
            n: u32,
        }
        let app = Router::new().route(
            "/q",
            axum::routing::get(|ApiQuery(q): ApiQuery<Q>| async move { q.n.to_string() }),
        );
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/q?n=7")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        for uri in ["/q?n=x", "/q", "/q?n=-1"] {
            let res = app
                .clone()
                .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
                .await
                .unwrap();
            let (s, _, body) = parts(res).await;
            assert_eq!(s, StatusCode::BAD_REQUEST, "{uri}");
            assert_eq!(body["code"], "bad_request");
        }
    }
}
