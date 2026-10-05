//! `X-Ouril-Client: <platform>/<semver> (<build>); core/<semver>` (docs/architecture/versioning.md).
//!
//! Parsed on every request: logged in the request span, stored as a request extension, and
//! used to answer `426 upgrade_required` when the app is below `min_supported`.

use std::cmp::Ordering;

use axum::{
    extract::{Request, State},
    http::HeaderMap,
    middleware::Next,
    response::{IntoResponse, Response},
};

use crate::{error::AppError, state::AppState};

pub use ouril_protocol::CLIENT_HEADER;

/// Longest header value we look at (anything longer is ignored as garbage).
const MAX_HEADER_LEN: usize = 128;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientInfo {
    /// `ios` | `android` | `web` (unknown platforms are kept but never version-gated).
    pub platform: String,
    pub version: String,
    pub build: Option<String>,
    pub core_version: Option<String>,
}

impl ClientInfo {
    /// Parse the header value. Lenient: `web/0.1.0` alone is accepted.
    pub fn parse(raw: &str) -> Option<ClientInfo> {
        if raw.len() > MAX_HEADER_LEN || !raw.is_ascii() {
            return None;
        }
        let mut parts = raw.split(';').map(str::trim);
        let app = parts.next()?;
        let (platform_version, build) = match app.split_once('(') {
            Some((pv, rest)) => (
                pv.trim(),
                Some(rest.trim_end_matches(')').trim().to_string()),
            ),
            None => (app, None),
        };
        let (platform, version) = platform_version.split_once('/')?;
        let platform = platform.trim().to_ascii_lowercase();
        let version = version.trim().to_string();
        if platform.is_empty()
            || !platform.chars().all(|c| c.is_ascii_alphanumeric())
            || parse_semver(&version).is_none()
        {
            return None;
        }
        let core_version = parts
            .find_map(|p| p.strip_prefix("core/"))
            .map(|v| v.trim().to_string())
            .filter(|v| parse_semver(v).is_some());
        Some(ClientInfo {
            platform,
            version,
            build: build.filter(|b| !b.is_empty() && b.chars().all(|c| c.is_ascii_alphanumeric())),
            core_version,
        })
    }

    pub fn from_headers(headers: &HeaderMap) -> Option<ClientInfo> {
        headers
            .get(CLIENT_HEADER)
            .and_then(|v| v.to_str().ok())
            .and_then(ClientInfo::parse)
    }

    /// Short form for logs and the session's device name.
    pub fn label(&self) -> String {
        match &self.core_version {
            Some(core) => format!("{}/{}; core/{}", self.platform, self.version, core),
            None => format!("{}/{}", self.platform, self.version),
        }
    }
}

/// `major.minor.patch` with an optional `-pre`/`+build` suffix (ignored for ordering).
pub fn parse_semver(v: &str) -> Option<(u64, u64, u64)> {
    let core = v.split(['-', '+']).next()?;
    let mut it = core.split('.');
    let major = it.next()?.parse().ok()?;
    let minor = it.next()?.parse().ok()?;
    let patch = it.next()?.parse().ok()?;
    if it.next().is_some() {
        return None;
    }
    Some((major, minor, patch))
}

/// Compare two semver strings; `None` when either doesn't parse.
pub fn compare_versions(a: &str, b: &str) -> Option<Ordering> {
    Some(parse_semver(a)?.cmp(&parse_semver(b)?))
}

/// The request path as the client sent it (before `Router::nest` stripped the prefix).
pub fn full_path(req: &Request) -> &str {
    req.extensions()
        .get::<axum::extract::OriginalUri>()
        .map(|u| u.0.path())
        .unwrap_or_else(|| req.uri().path())
}

/// Middleware for `/v1`: store [`ClientInfo`] as an extension and refuse apps below
/// `min_supported` with 426. `/v1/meta` is exempt (clients need it to learn they're too old).
/// A missing or unparseable header is allowed (curl, health checks, future clients).
pub async fn client_version_gate(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Response {
    let info = ClientInfo::from_headers(req.headers());
    if let Some(info) = &info {
        // Inside the nested `/v1` router `req.uri()` has the prefix stripped; compare the
        // original URI so the exemption matches `/v1/meta` exactly.
        let exempt = full_path(&req) == "/v1/meta";
        if !exempt {
            if let Some(min) = state.config.min_supported.get(&info.platform) {
                if compare_versions(&info.version, min) == Some(Ordering::Less) {
                    return AppError::upgrade_required(format!(
                        "{} {} is below the minimum supported version {}",
                        info.platform, info.version, min
                    ))
                    .into_response();
                }
            }
        }
    }
    if let Some(info) = info {
        req.extensions_mut().insert(info);
    }
    next.run(req).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_full_header() {
        let c = ClientInfo::parse("web/0.1.0 (1); core/0.1.0").unwrap();
        assert_eq!(c.platform, "web");
        assert_eq!(c.version, "0.1.0");
        assert_eq!(c.build.as_deref(), Some("1"));
        assert_eq!(c.core_version.as_deref(), Some("0.1.0"));
        assert_eq!(c.label(), "web/0.1.0; core/0.1.0");
    }

    #[test]
    fn parses_minimal_and_rejects_garbage() {
        assert_eq!(ClientInfo::parse("ios/1.4.0").unwrap().platform, "ios");
        assert!(ClientInfo::parse("nonsense").is_none());
        assert!(ClientInfo::parse("web/abc").is_none());
        assert!(ClientInfo::parse("we b/1.0.0").is_none());
        assert!(ClientInfo::parse(&"x".repeat(500)).is_none());
    }

    #[test]
    fn semver_ordering() {
        assert_eq!(compare_versions("0.1.0", "0.2.0"), Some(Ordering::Less));
        assert_eq!(compare_versions("1.10.0", "1.9.9"), Some(Ordering::Greater));
        assert_eq!(
            compare_versions("1.0.0-beta", "1.0.0"),
            Some(Ordering::Equal)
        );
        assert_eq!(compare_versions("1.0", "1.0.0"), None);
    }
}
