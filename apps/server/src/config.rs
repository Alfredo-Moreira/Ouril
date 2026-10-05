//! Server configuration from the environment (12-factor). Template: `.env.example`.
//!
//! Secrets (`JWT_SECRET`, `DATABASE_URL`, Apple key path) are redacted from `Debug` so they
//! can never reach the logs.

use std::{env, fmt};

#[derive(Clone)]
pub struct Config {
    pub database_url: String,
    pub db_max_connections: u32,
    pub host: String,
    pub port: u16,
    pub jwt_secret: String,
    pub access_token_ttl_secs: u32,
    pub refresh_token_ttl_days: u32,
    /// Exact origins allowed for CORS and cookie-based (web) auth.
    pub allowed_origins: Vec<String>,
    pub min_supported: Platforms,
    pub recommended: Platforms,
    /// `None` = Google sign-in not configured (501 `not_configured`).
    pub google_client_id: Option<String>,
    /// `None` = Apple sign-in not configured (501 `not_configured`).
    pub apple: Option<AppleConfig>,
    /// Server-side error reporting. `None` = no-op (ADR 0014). Not wired yet.
    pub sentry_dsn: Option<String>,
    /// Trust the `Fly-Client-IP` header for the client address (rate limiting). Set when
    /// `FLY_APP_NAME` is present, i.e. only behind Fly's proxy, so the header can't be spoofed
    /// anywhere else.
    pub trust_fly_client_ip: bool,
    /// Sign-in requests per client network per minute (`RATE_LIMIT_AUTH_PER_MINUTE`).
    pub rate_limit_auth_per_minute: u32,
    /// Other `/v1` requests per client network per minute (`RATE_LIMIT_DEFAULT_PER_MINUTE`).
    pub rate_limit_default_per_minute: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Platforms {
    pub ios: String,
    pub android: String,
    pub web: String,
}

impl Platforms {
    /// The version for a platform name from `X-Ouril-Client` (`ios`, `android`, `web`).
    pub fn get(&self, platform: &str) -> Option<&str> {
        match platform {
            "ios" => Some(&self.ios),
            "android" => Some(&self.android),
            "web" => Some(&self.web),
            _ => None,
        }
    }
}

#[derive(Clone)]
pub struct AppleConfig {
    /// Audience of web identity tokens (Services ID). iOS tokens use the app's bundle ID.
    pub service_id: String,
    pub team_id: String,
    pub key_id: String,
    /// Path to the `.p8` key inside the container (needed for token revocation).
    pub private_key_path: String,
    /// iOS app bundle ID (`APPLE_BUNDLE_ID`): the audience of native iOS identity tokens.
    pub bundle_id: Option<String>,
    /// Redirect URI registered for the web flow (`APPLE_REDIRECT_URI`), sent with code
    /// exchanges from the web.
    pub redirect_uri: Option<String>,
}

impl fmt::Debug for AppleConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AppleConfig")
            .field("service_id", &self.service_id)
            .field("team_id", &self.team_id)
            .field("key_id", &self.key_id)
            .field("private_key_path", &"<redacted>")
            .field("bundle_id", &self.bundle_id)
            .field("redirect_uri", &self.redirect_uri)
            .finish()
    }
}

impl fmt::Debug for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Config")
            .field("database_url", &"<redacted>")
            .field("db_max_connections", &self.db_max_connections)
            .field("host", &self.host)
            .field("port", &self.port)
            .field("jwt_secret", &"<redacted>")
            .field("access_token_ttl_secs", &self.access_token_ttl_secs)
            .field("refresh_token_ttl_days", &self.refresh_token_ttl_days)
            .field("allowed_origins", &self.allowed_origins)
            .field("min_supported", &self.min_supported)
            .field("recommended", &self.recommended)
            .field("google_client_id", &self.google_client_id)
            .field("apple", &self.apple)
            .field(
                "sentry_dsn",
                &self.sentry_dsn.as_ref().map(|_| "<redacted>"),
            )
            .field("trust_fly_client_ip", &self.trust_fly_client_ip)
            .field(
                "rate_limit_auth_per_minute",
                &self.rate_limit_auth_per_minute,
            )
            .field(
                "rate_limit_default_per_minute",
                &self.rate_limit_default_per_minute,
            )
            .finish()
    }
}

#[derive(Debug)]
pub struct ConfigError(pub String);

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "configuration error: {}", self.0)
    }
}

impl std::error::Error for ConfigError {}

/// Minimum `JWT_SECRET` length in bytes.
pub const MIN_JWT_SECRET_BYTES: usize = 32;

/// The public `JWT_SECRET` from `.env.example`. Never valid in a release build.
pub const EXAMPLE_JWT_SECRET: &str = "dev-only-insecure-jwt-secret-change-me-0123456789";

/// Whether this is a release build (staging/production image). Debug builds keep the
/// convenient local-development defaults.
const RELEASE_BUILD: bool = !cfg!(debug_assertions);

/// `JWT_SECRET` checks. In a release build the secret must not be a known placeholder: anyone
/// who knows it could mint access tokens.
fn check_jwt_secret(secret: &str, release: bool) -> Result<(), ConfigError> {
    if secret.len() < MIN_JWT_SECRET_BYTES {
        return Err(ConfigError(format!(
            "JWT_SECRET must be at least {MIN_JWT_SECRET_BYTES} bytes"
        )));
    }
    let lower = secret.to_ascii_lowercase();
    if release
        && (secret == EXAMPLE_JWT_SECRET
            || lower.contains("dev-only")
            || lower.contains("change-me")
            || lower.contains("changeme"))
    {
        return Err(ConfigError(
            "JWT_SECRET is a development placeholder; set a real random secret \
             (e.g. `openssl rand -base64 48`)"
                .into(),
        ));
    }
    Ok(())
}

/// `http://localhost[:port]`, `http://127.0.0.1[:port]` or `http://[::1][:port]`.
pub fn is_loopback_http_origin(origin: &str) -> bool {
    let Some(rest) = origin.strip_prefix("http://") else {
        return false;
    };
    let host = if rest.starts_with("[::1]") {
        "[::1]"
    } else {
        rest.split(':').next().unwrap_or(rest)
    };
    let tail = &rest[host.len().min(rest.len())..];
    (host == "localhost" || host == "127.0.0.1" || host == "[::1]")
        && (tail.is_empty()
            || tail
                .strip_prefix(':')
                .is_some_and(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit())))
}

/// One `ALLOWED_ORIGINS` entry. Release builds only accept `https://` (plus loopback
/// `http://` for local testing of the production image), because the refresh cookie's
/// `Secure` attribute follows the origin's scheme.
fn check_origin(o: &str, release: bool) -> Result<(), ConfigError> {
    if o == "*" || !(o.starts_with("https://") || o.starts_with("http://")) {
        return Err(ConfigError(format!(
            "ALLOWED_ORIGINS must list exact http(s) origins, got {o:?}"
        )));
    }
    if release && o.starts_with("http://") && !is_loopback_http_origin(o) {
        return Err(ConfigError(format!(
            "ALLOWED_ORIGINS must use https:// in a release build (only loopback may use \
             http://), got {o:?}"
        )));
    }
    Ok(())
}

fn positive(name: &str, v: u32) -> Result<u32, ConfigError> {
    if v == 0 {
        return Err(ConfigError(format!("{name} must be at least 1")));
    }
    Ok(v)
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::from_lookup(|name| env::var(name).ok())
    }

    /// Build the config from any key lookup (tests pass a map instead of the environment).
    pub fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Result<Self, ConfigError> {
        // A non-empty variable, or `None` (empty placeholders count as unset).
        let opt = |name: &str| {
            lookup(name)
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
        };
        let req = |name: &str| opt(name).ok_or_else(|| ConfigError(format!("{name} is required")));
        fn parse<T: std::str::FromStr>(
            name: &str,
            v: Option<String>,
            default: T,
        ) -> Result<T, ConfigError> {
            match v {
                None => Ok(default),
                Some(v) => v
                    .parse()
                    .map_err(|_| ConfigError(format!("{name} is invalid: {v:?}"))),
            }
        }

        let jwt_secret = req("JWT_SECRET")?;
        check_jwt_secret(&jwt_secret, RELEASE_BUILD)?;
        // Apple needs every value; a partial set is "not configured" (and logged as a warning).
        let apple_parts = [
            opt("APPLE_SERVICE_ID"),
            opt("APPLE_TEAM_ID"),
            opt("APPLE_KEY_ID"),
            opt("APPLE_PRIVATE_KEY_PATH"),
        ];
        let apple = match apple_parts {
            [Some(service_id), Some(team_id), Some(key_id), Some(private_key_path)] => {
                Some(AppleConfig {
                    service_id,
                    team_id,
                    key_id,
                    private_key_path,
                    bundle_id: opt("APPLE_BUNDLE_ID"),
                    redirect_uri: opt("APPLE_REDIRECT_URI"),
                })
            }
            ref parts => {
                if parts.iter().any(Option::is_some) {
                    tracing::warn!(
                        "some APPLE_* variables are set but not all: Apple sign-in stays disabled"
                    );
                }
                None
            }
        };
        let access_token_ttl_secs =
            parse("ACCESS_TOKEN_TTL_SECS", opt("ACCESS_TOKEN_TTL_SECS"), 900)?;
        if !(60..=86_400).contains(&access_token_ttl_secs) {
            return Err(ConfigError(
                "ACCESS_TOKEN_TTL_SECS must be 60..=86400".into(),
            ));
        }
        let refresh_token_ttl_days =
            parse("REFRESH_TOKEN_TTL_DAYS", opt("REFRESH_TOKEN_TTL_DAYS"), 30)?;
        if !(1..=365).contains(&refresh_token_ttl_days) {
            return Err(ConfigError("REFRESH_TOKEN_TTL_DAYS must be 1..=365".into()));
        }
        let allowed_origins: Vec<String> = opt("ALLOWED_ORIGINS")
            .map(|v| {
                v.split(',')
                    .map(|s| s.trim().trim_end_matches('/').to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            })
            .unwrap_or_default();
        for o in &allowed_origins {
            check_origin(o, RELEASE_BUILD)?;
        }
        let version = |name: &str| opt(name).unwrap_or_else(|| "0.1.0".into());
        Ok(Config {
            database_url: req("DATABASE_URL")?,
            db_max_connections: parse("DB_MAX_CONNECTIONS", opt("DB_MAX_CONNECTIONS"), 10)?,
            host: opt("HOST").unwrap_or_else(|| "0.0.0.0".into()),
            port: parse("PORT", opt("PORT"), 8080)?,
            jwt_secret,
            access_token_ttl_secs,
            refresh_token_ttl_days,
            allowed_origins,
            min_supported: Platforms {
                ios: version("MIN_SUPPORTED_IOS"),
                android: version("MIN_SUPPORTED_ANDROID"),
                web: version("MIN_SUPPORTED_WEB"),
            },
            recommended: Platforms {
                ios: version("RECOMMENDED_IOS"),
                android: version("RECOMMENDED_ANDROID"),
                web: version("RECOMMENDED_WEB"),
            },
            google_client_id: opt("GOOGLE_CLIENT_ID"),
            apple,
            sentry_dsn: opt("SENTRY_DSN"),
            trust_fly_client_ip: opt("FLY_APP_NAME").is_some(),
            rate_limit_auth_per_minute: positive(
                "RATE_LIMIT_AUTH_PER_MINUTE",
                parse(
                    "RATE_LIMIT_AUTH_PER_MINUTE",
                    opt("RATE_LIMIT_AUTH_PER_MINUTE"),
                    crate::ratelimit::AUTH_LIMIT_PER_MINUTE,
                )?,
            )?,
            rate_limit_default_per_minute: positive(
                "RATE_LIMIT_DEFAULT_PER_MINUTE",
                parse(
                    "RATE_LIMIT_DEFAULT_PER_MINUTE",
                    opt("RATE_LIMIT_DEFAULT_PER_MINUTE"),
                    crate::ratelimit::DEFAULT_LIMIT_PER_MINUTE,
                )?,
            )?,
        })
    }

    /// Whether `origin` is one of `ALLOWED_ORIGINS` (exact match).
    pub fn origin_allowed(&self, origin: &str) -> bool {
        let origin = origin.trim_end_matches('/');
        self.allowed_origins.iter().any(|o| o == origin)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn base() -> HashMap<&'static str, &'static str> {
        HashMap::from([
            ("DATABASE_URL", "postgres://u:p@db/x"),
            ("JWT_SECRET", "0123456789abcdef0123456789abcdef"),
            (
                "ALLOWED_ORIGINS",
                "http://localhost:5173, https://ouril.example/",
            ),
            ("GOOGLE_CLIENT_ID", ""),
            ("APPLE_SERVICE_ID", "x"),
        ])
    }

    fn load(m: &HashMap<&'static str, &'static str>) -> Result<Config, ConfigError> {
        Config::from_lookup(|k| m.get(k).map(|v| v.to_string()))
    }

    #[test]
    fn defaults_and_placeholders() {
        let c = load(&base()).unwrap();
        assert_eq!(c.port, 8080);
        assert_eq!(c.access_token_ttl_secs, 900);
        assert!(c.google_client_id.is_none(), "empty = not configured");
        assert!(c.apple.is_none(), "partial Apple config = not configured");
        assert!(c.origin_allowed("https://ouril.example"));
        assert!(c.origin_allowed("http://localhost:5173"));
        assert!(!c.origin_allowed("http://evil.example"));
    }

    #[test]
    fn short_secret_rejected() {
        let mut m = base();
        m.insert("JWT_SECRET", "short");
        assert!(load(&m).is_err());
    }

    #[test]
    fn wildcard_origin_rejected() {
        let mut m = base();
        m.insert("ALLOWED_ORIGINS", "*");
        assert!(load(&m).is_err());
    }

    #[test]
    fn debug_redacts_secrets() {
        let c = load(&base()).unwrap();
        let s = format!("{c:?}");
        assert!(!s.contains("0123456789abcdef"));
        assert!(!s.contains("postgres://"));
    }

    #[test]
    fn required_variables() {
        for missing in ["DATABASE_URL", "JWT_SECRET"] {
            let mut m = base();
            m.remove(missing);
            let err = load(&m).unwrap_err().to_string();
            assert!(err.contains(missing), "{err}");
            // Whitespace-only counts as missing.
            m.insert(missing, "   ");
            assert!(load(&m).is_err());
        }
    }

    #[test]
    fn numeric_ranges_and_parse_errors() {
        for (k, v) in [
            ("ACCESS_TOKEN_TTL_SECS", "59"),
            ("ACCESS_TOKEN_TTL_SECS", "86401"),
            ("ACCESS_TOKEN_TTL_SECS", "abc"),
            ("REFRESH_TOKEN_TTL_DAYS", "0"),
            ("REFRESH_TOKEN_TTL_DAYS", "366"),
            ("PORT", "70000"),
            ("PORT", "-1"),
            ("DB_MAX_CONNECTIONS", "many"),
            ("RATE_LIMIT_AUTH_PER_MINUTE", "0"),
            ("RATE_LIMIT_DEFAULT_PER_MINUTE", "lots"),
        ] {
            let mut m = base();
            m.insert(k, v);
            assert!(load(&m).is_err(), "{k}={v} should be rejected");
        }
        let mut m = base();
        m.insert("ACCESS_TOKEN_TTL_SECS", "60");
        m.insert("REFRESH_TOKEN_TTL_DAYS", "365");
        m.insert("PORT", " 9000 ");
        m.insert("DB_MAX_CONNECTIONS", "3");
        let c = load(&m).unwrap();
        assert_eq!(
            (
                c.access_token_ttl_secs,
                c.refresh_token_ttl_days,
                c.port,
                c.db_max_connections
            ),
            (60, 365, 9000, 3)
        );
        let c = load(&base()).unwrap();
        assert_eq!((c.refresh_token_ttl_days, c.db_max_connections), (30, 10));
        assert_eq!(c.host, "0.0.0.0");
    }

    #[test]
    fn origins_must_be_exact_http_urls() {
        for bad in ["localhost:5173", "ftp://x.example", "https://ok.example,*"] {
            let mut m = base();
            m.insert("ALLOWED_ORIGINS", bad);
            assert!(load(&m).is_err(), "{bad}");
        }
        let mut m = base();
        m.remove("ALLOWED_ORIGINS");
        let c = load(&m).unwrap();
        assert!(c.allowed_origins.is_empty(), "no origins = no web clients");
        assert!(!c.origin_allowed("http://localhost:5173"));
        // Trailing slashes are normalized on both sides; anything else must match exactly.
        let c = load(&base()).unwrap();
        assert!(c.origin_allowed("https://ouril.example/"));
        assert!(!c.origin_allowed("https://ouril.example.evil"));
        assert!(!c.origin_allowed("https://OURIL.example"));
        assert!(!c.origin_allowed("http://localhost:5174"));
        assert!(!c.origin_allowed(""));
    }

    #[test]
    fn oauth_placeholders() {
        let mut m = base();
        m.insert("GOOGLE_CLIENT_ID", "  ");
        assert!(load(&m).unwrap().google_client_id.is_none());
        m.insert("GOOGLE_CLIENT_ID", "abc.apps.googleusercontent.com");
        assert_eq!(
            load(&m).unwrap().google_client_id.as_deref(),
            Some("abc.apps.googleusercontent.com")
        );
        // Apple needs all four values.
        let mut m = base();
        m.insert("APPLE_SERVICE_ID", "com.example.web");
        m.insert("APPLE_TEAM_ID", "TEAM");
        m.insert("APPLE_KEY_ID", "KEY");
        assert!(load(&m).unwrap().apple.is_none());
        m.insert("APPLE_PRIVATE_KEY_PATH", "/secrets/apple.p8");
        let apple = load(&m).unwrap().apple.expect("configured");
        assert_eq!(apple.service_id, "com.example.web");
        assert!(!format!("{apple:?}").contains("/secrets/apple.p8"));
    }

    #[test]
    fn versions_default_and_override() {
        let mut m = base();
        m.insert("MIN_SUPPORTED_WEB", "1.2.3");
        m.insert("RECOMMENDED_IOS", "2.0.0");
        let c = load(&m).unwrap();
        assert_eq!(c.min_supported.get("web"), Some("1.2.3"));
        assert_eq!(c.min_supported.get("ios"), Some("0.1.0"));
        assert_eq!(c.recommended.get("ios"), Some("2.0.0"));
        assert_eq!(c.recommended.get("android"), Some("0.1.0"));
        assert_eq!(c.min_supported.get("tvos"), None);
    }

    #[test]
    fn placeholder_jwt_secret_rejected_in_release() {
        // Debug builds (local dev with the copied .env) accept the example secret.
        assert!(check_jwt_secret(EXAMPLE_JWT_SECRET, false).is_ok());
        for bad in [
            EXAMPLE_JWT_SECRET,
            "some-DEV-ONLY-secret-that-is-long-enough-123",
            "please-change-me-please-change-me-0123456789",
            "CHANGEME-CHANGEME-CHANGEME-CHANGEME-01234",
        ] {
            assert!(check_jwt_secret(bad, true).is_err(), "{bad}");
        }
        assert!(check_jwt_secret("Zq3v9TnJ0wq2Lr8yXk1PbH6sGm4dCe7uFa5", true).is_ok());
        assert!(check_jwt_secret("short", false).is_err());
        assert!(check_jwt_secret("short", true).is_err());
        // In a debug build (how tests run) the .env.example value loads.
        if !RELEASE_BUILD {
            let mut m = base();
            m.insert("JWT_SECRET", EXAMPLE_JWT_SECRET);
            assert!(load(&m).is_ok());
        }
    }

    #[test]
    fn release_requires_https_origins() {
        for ok in [
            "https://ouril.example",
            "http://localhost",
            "http://localhost:5173",
            "http://127.0.0.1:8080",
            "http://[::1]:5173",
        ] {
            assert!(check_origin(ok, true).is_ok(), "{ok}");
        }
        for bad in [
            "http://ouril.example",
            "http://localhost.evil.example",
            "http://localhost:5173.evil.example",
            "http://127.0.0.1.evil.example",
            "http://localhost:",
            "http://[::1]x",
        ] {
            assert!(check_origin(bad, true).is_err(), "{bad}");
        }
        // Debug builds still accept plain http for LAN testing.
        assert!(check_origin("http://192.168.1.10:5173", false).is_ok());
    }

    #[test]
    fn fly_client_ip_trust_follows_fly_app_name() {
        assert!(!load(&base()).unwrap().trust_fly_client_ip);
        let mut m = base();
        m.insert("FLY_APP_NAME", "ouril-staging");
        assert!(load(&m).unwrap().trust_fly_client_ip);
        m.insert("FLY_APP_NAME", " ");
        assert!(!load(&m).unwrap().trust_fly_client_ip);
    }

    #[test]
    fn debug_redacts_sentry_dsn() {
        let mut m = base();
        m.insert("SENTRY_DSN", "https://publickey@o0.ingest.example/1");
        let c = load(&m).unwrap();
        assert!(c.sentry_dsn.is_some());
        assert!(!format!("{c:?}").contains("publickey"));
    }
}
