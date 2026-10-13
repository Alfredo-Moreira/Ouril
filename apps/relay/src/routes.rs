//! HTTP: `GET /ws` (players), `POST /publish` (the server), `GET /healthz`. Everything else
//! is 404.

use std::{
    net::{IpAddr, SocketAddr},
    sync::Arc,
};

use axum::{
    extract::{ws::WebSocketUpgrade, ConnectInfo, DefaultBodyLimit, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use ouril_protocol::{is_valid_channel, RelayPublish};

use crate::{
    config::Config,
    hub::Hub,
    limits::{Limits, Refused},
    ws,
};

/// Largest `/publish` body: events are a few dozen bytes.
const MAX_PUBLISH_BYTES: usize = 4 * 1024;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub hub: Arc<Hub>,
    pub limits: Arc<Limits>,
}

/// The relay's router. Serve it with `into_make_service_with_connect_info::<SocketAddr>()`.
pub fn app(config: Config) -> Router {
    let limits = Limits::new(
        config.max_connections,
        config.max_connections_per_ip,
        config.upgrades_per_minute_per_ip,
    );
    let state = AppState {
        config: Arc::new(config),
        hub: Arc::new(Hub::default()),
        limits,
    };
    // Every minute: forget channels nobody listens to, and log how busy the relay is (counts
    // only, never who).
    let hub = Arc::clone(&state.hub);
    let limits = Arc::clone(&state.limits);
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(std::time::Duration::from_secs(60));
        loop {
            tick.tick().await;
            hub.prune();
            tracing::info!(
                connections = limits.connections(),
                channels = hub.channel_count(),
                "relay load"
            );
        }
    });
    Router::new()
        .route("/ws", get(upgrade))
        .route("/publish", post(publish))
        .route("/healthz", get(|| async { "ok" }))
        .fallback(|| async { StatusCode::NOT_FOUND })
        .layer(DefaultBodyLimit::max(MAX_PUBLISH_BYTES))
        .layer(middleware::from_fn(security_headers))
        .with_state(state)
}

async fn security_headers(req: axum::extract::Request, next: Next) -> Response {
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
    res
}

/// The client's IP: Cloudflare's header when trusted (homelab), else the TCP peer.
fn client_ip(config: &Config, headers: &HeaderMap, peer: SocketAddr) -> IpAddr {
    if config.trust_cf_connecting_ip {
        if let Some(ip) = headers
            .get("cf-connecting-ip")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.trim().parse().ok())
        {
            return ip;
        }
    }
    peer.ip()
}

async fn upgrade(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
) -> Response {
    let origin_ok = headers
        .get(header::ORIGIN)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|o| state.config.origin_allowed(o));
    if !origin_ok {
        return StatusCode::FORBIDDEN.into_response();
    }
    let ip = client_ip(&state.config, &headers, peer);
    let slot = match state.limits.admit(ip) {
        Ok(slot) => slot,
        Err(Refused::RateLimited) | Err(Refused::Full) => {
            return StatusCode::TOO_MANY_REQUESTS.into_response()
        }
    };
    ws.max_message_size(ws::MAX_MESSAGE_BYTES)
        .max_frame_size(ws::MAX_MESSAGE_BYTES)
        .on_upgrade(move |socket| ws::serve(socket, state, slot))
}

/// Constant-time comparison, so response timing reveals nothing about the key.
fn same_secret(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

async fn publish(State(state): State<AppState>, headers: HeaderMap, body: String) -> StatusCode {
    let presented = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .unwrap_or_default();
    if !same_secret(presented.as_bytes(), state.config.api_key.as_bytes()) {
        return StatusCode::UNAUTHORIZED;
    }
    let Ok(msg) = serde_json::from_str::<RelayPublish>(&body) else {
        return StatusCode::BAD_REQUEST;
    };
    if !is_valid_channel(&msg.channel) {
        return StatusCode::BAD_REQUEST;
    }
    let Ok(event) = serde_json::to_string(&msg.event) else {
        return StatusCode::BAD_REQUEST;
    };
    state.hub.publish(&msg.channel, event.into());
    StatusCode::NO_CONTENT
}

#[cfg(test)]
mod tests {
    use super::same_secret;

    #[test]
    fn secrets_compare_exactly() {
        assert!(same_secret(b"abc", b"abc"));
        assert!(!same_secret(b"abc", b"abd"));
        assert!(!same_secret(b"abc", b"abcd"));
        assert!(!same_secret(b"", b"a"));
    }
}
