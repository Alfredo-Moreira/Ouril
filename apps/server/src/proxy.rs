//! The web app's proxy in front of `/v1` (ADR 0029).
//!
//! In production the browser calls `https://<web origin>/v1/…` and Vercel forwards the request
//! to this server, adding [`ORIGIN_SECRET_HEADER`] with the shared `ORIGIN_SECRET`. When that
//! secret is configured, every `/v1` request must carry it, so nobody can bypass the proxy and
//! hit the Fly address directly; requests that do carry it get [`ViaTrustedProxy`], which lets
//! the rate limiter trust the visitor IP Vercel puts in `x-real-ip`. Without `ORIGIN_SECRET`
//! (local development, tests) nothing is checked.

use axum::{
    extract::{Request, State},
    middleware::Next,
    response::{IntoResponse, Response},
};
use sha2::{Digest, Sha256};

use crate::{error::AppError, state::AppState};

/// Request header the proxy sets to `ORIGIN_SECRET` (Vercel `routes` transform, `vercel.json`).
pub const ORIGIN_SECRET_HEADER: &str = "x-ouril-origin-secret";

/// Request extension: the request came through the proxy (its secret matched).
#[derive(Debug, Clone, Copy)]
pub struct ViaTrustedProxy;

/// Constant-time comparison: both sides are hashed first, so neither the content nor the length
/// of the secret leaks through timing.
fn secret_matches(given: &[u8], expected: &[u8]) -> bool {
    let (a, b) = (Sha256::digest(given), Sha256::digest(expected));
    a.iter()
        .zip(b.iter())
        .fold(0u8, |acc, (x, y)| acc | (x ^ y))
        == 0
}

/// `/v1` middleware: with `ORIGIN_SECRET` set, reject requests without the proxy's secret
/// (403 `forbidden`) and mark the others [`ViaTrustedProxy`]. The header is removed before the
/// request goes further, so the secret can't reach handlers or logs.
pub async fn require_proxy(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Response {
    let Some(secret) = state.config.origin_secret.as_deref() else {
        return next.run(req).await;
    };
    let ok = req
        .headers()
        .get(ORIGIN_SECRET_HEADER)
        .is_some_and(|v| secret_matches(v.as_bytes(), secret.as_bytes()));
    if !ok {
        return AppError::forbidden("requests must come through the web app").into_response();
    }
    req.headers_mut().remove(ORIGIN_SECRET_HEADER);
    req.extensions_mut().insert(ViaTrustedProxy);
    next.run(req).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secret_comparison() {
        assert!(secret_matches(b"abc", b"abc"));
        assert!(!secret_matches(b"abc", b"abd"));
        assert!(!secret_matches(b"abc", b"abcd"));
        assert!(!secret_matches(b"", b"abc"));
    }
}
