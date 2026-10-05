//! Web vs native clients, and the `ouril_refresh` httpOnly cookie (API.md "Authentication").
//!
//! A request with an `Origin` header in `ALLOWED_ORIGINS` is a **web** client: it gets the
//! refresh token as a cookie and never in the JSON body. A request without `Origin` is a
//! **native** client (or curl): refresh token in the body. Any other `Origin` is refused (403).

use axum::http::{header, HeaderMap, HeaderValue};

use crate::{config::Config, error::AppError};

pub use ouril_protocol::REFRESH_COOKIE;

pub const COOKIE_PATH: &str = "/v1/auth";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClientKind {
    /// Browser on an allowed origin. `secure` = set the `Secure` cookie attribute.
    Web {
        secure: bool,
    },
    Native,
}

impl ClientKind {
    pub fn is_web(&self) -> bool {
        matches!(self, ClientKind::Web { .. })
    }
}

/// Classify the caller from `Origin`. Disallowed origins are a 403 (CSRF defence).
pub fn client_kind(config: &Config, headers: &HeaderMap) -> Result<ClientKind, AppError> {
    let Some(origin) = headers.get(header::ORIGIN) else {
        return Ok(ClientKind::Native);
    };
    let origin = origin
        .to_str()
        .map_err(|_| AppError::forbidden("origin not allowed"))?;
    if !config.origin_allowed(origin) {
        return Err(AppError::forbidden("origin not allowed"));
    }
    Ok(ClientKind::Web {
        secure: !is_local_http(origin),
    })
}

/// `http://localhost[:port]`, `http://127.0.0.1[:port]` and `http://[::1][:port]` can't receive
/// `Secure` cookies.
fn is_local_http(origin: &str) -> bool {
    let Some(rest) = origin.strip_prefix("http://") else {
        return false;
    };
    // IPv6 literals are bracketed and contain ':' themselves, so take everything up to ']'.
    let host = match rest.strip_prefix('[') {
        Some(v6) => match v6.split_once(']') {
            Some((addr, after)) if after.is_empty() || after.starts_with(':') => addr,
            _ => return false,
        },
        None => rest.split(':').next().unwrap_or(rest),
    };
    host == "localhost" || host == "127.0.0.1" || host == "::1"
}

/// `Set-Cookie` that stores a refresh token.
pub fn set_refresh_cookie(token: &str, max_age_secs: u64, secure: bool) -> HeaderValue {
    let secure = if secure { "; Secure" } else { "" };
    let v = format!(
        "{REFRESH_COOKIE}={token}; HttpOnly{secure}; SameSite=Lax; Path={COOKIE_PATH}; Max-Age={max_age_secs}"
    );
    HeaderValue::from_str(&v).expect("cookie value is ASCII")
}

/// `Set-Cookie` that deletes the refresh cookie.
pub fn clear_refresh_cookie(secure: bool) -> HeaderValue {
    set_refresh_cookie("", 0, secure)
}

/// Read the refresh token from the `Cookie` header(s).
pub fn read_refresh_cookie(headers: &HeaderMap) -> Option<String> {
    headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(';'))
        .filter_map(|pair| pair.trim().split_once('='))
        .find(|(k, _)| *k == REFRESH_COOKIE)
        .map(|(_, v)| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_http_is_not_secure() {
        assert!(is_local_http("http://localhost:5173"));
        assert!(is_local_http("http://127.0.0.1"));
        assert!(!is_local_http("https://localhost:5173"));
        assert!(!is_local_http("http://ouril.example"));
        assert!(is_local_http("http://[::1]"));
        assert!(is_local_http("http://[::1]:5173"));
        assert!(!is_local_http("https://[::1]:5173"));
        assert!(!is_local_http("http://[::2]:5173"));
        assert!(!is_local_http("http://[::1]evil.example"));
        assert!(!is_local_http("http://[::1"));
    }

    #[test]
    fn cookie_attributes() {
        let v = set_refresh_cookie("abc", 60, true);
        let s = v.to_str().unwrap();
        assert!(s.starts_with("ouril_refresh=abc;"));
        for attr in [
            "HttpOnly",
            "Secure",
            "SameSite=Lax",
            "Path=/v1/auth",
            "Max-Age=60",
        ] {
            assert!(s.contains(attr), "{s} lacks {attr}");
        }
        assert!(!set_refresh_cookie("abc", 60, false)
            .to_str()
            .unwrap()
            .contains("Secure"));
        assert!(clear_refresh_cookie(true)
            .to_str()
            .unwrap()
            .contains("Max-Age=0"));
    }

    #[test]
    fn reads_cookie() {
        let mut h = HeaderMap::new();
        h.insert(
            header::COOKIE,
            HeaderValue::from_static("a=1; ouril_refresh=tok; b=2"),
        );
        assert_eq!(read_refresh_cookie(&h).as_deref(), Some("tok"));
        h.insert(header::COOKIE, HeaderValue::from_static("ouril_refresh="));
        assert_eq!(read_refresh_cookie(&h), None);
    }

    fn config() -> Config {
        Config::from_lookup(|k| match k {
            "DATABASE_URL" => Some("postgres://x".into()),
            "JWT_SECRET" => Some("0123456789abcdef0123456789abcdef".into()),
            "ALLOWED_ORIGINS" => Some("http://localhost:5173,https://ouril.example".into()),
            _ => None,
        })
        .unwrap()
    }

    fn with_origin(o: &'static str) -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert(header::ORIGIN, HeaderValue::from_static(o));
        h
    }

    #[test]
    fn classifies_clients_by_origin() {
        let c = config();
        assert_eq!(
            client_kind(&c, &HeaderMap::new()).unwrap(),
            ClientKind::Native
        );
        assert_eq!(
            client_kind(&c, &with_origin("http://localhost:5173")).unwrap(),
            ClientKind::Web { secure: false }
        );
        assert_eq!(
            client_kind(&c, &with_origin("https://ouril.example")).unwrap(),
            ClientKind::Web { secure: true }
        );
        for bad in [
            "https://evil.example",
            "null",
            "",
            "http://localhost:5173.evil.example",
        ] {
            let err = client_kind(&c, &with_origin(bad)).unwrap_err();
            assert_eq!(err.status, axum::http::StatusCode::FORBIDDEN, "{bad}");
        }
        let mut h = HeaderMap::new();
        h.insert(
            header::ORIGIN,
            HeaderValue::from_bytes(b"https://\xff").unwrap(),
        );
        assert!(client_kind(&c, &h).is_err(), "non-UTF-8 origin");
    }

    #[test]
    fn reads_cookie_across_headers_and_ignores_lookalikes() {
        let mut h = HeaderMap::new();
        h.append(
            header::COOKIE,
            HeaderValue::from_static("x_ouril_refresh=nope"),
        );
        h.append(
            header::COOKIE,
            HeaderValue::from_static("other=1;ouril_refresh=yes"),
        );
        assert_eq!(read_refresh_cookie(&h).as_deref(), Some("yes"));
        assert_eq!(read_refresh_cookie(&HeaderMap::new()), None);
    }
}
