//! Relay configuration from the environment. Secrets are redacted from `Debug`.

use std::{fmt, time::Duration};

/// Shared secrets must be at least this long (`openssl rand -hex 32`).
pub const MIN_SECRET_BYTES: usize = 32;

#[derive(Clone)]
pub struct Config {
    pub port: u16,
    /// `Authorization: Bearer` key the server sends on `/publish` (`RELAY_API_KEY`).
    pub api_key: String,
    /// Verifies players' tokens (`RELAY_TOKEN_SECRET`).
    pub token_secret: String,
    /// Exact `Origin` values allowed to open a WebSocket (`ALLOWED_ORIGINS`).
    pub allowed_origins: Vec<String>,
    /// Take the client IP from `CF-Connecting-IP` (`TRUST_CF_CONNECTING_IP=true`). Only when
    /// the relay is reachable through Cloudflare alone, as on the homelab.
    pub trust_cf_connecting_ip: bool,
    pub max_connections: usize,
    pub max_connections_per_ip: usize,
    pub upgrades_per_minute_per_ip: u32,
    /// How long a new connection has to send its token.
    pub auth_timeout: Duration,
    /// How often the relay pings each connection.
    pub ping_interval: Duration,
    /// A connection that sends nothing (not even a pong) for this long is closed.
    pub idle_timeout: Duration,
}

impl fmt::Debug for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Config")
            .field("port", &self.port)
            .field("api_key", &"<redacted>")
            .field("token_secret", &"<redacted>")
            .field("allowed_origins", &self.allowed_origins)
            .field("trust_cf_connecting_ip", &self.trust_cf_connecting_ip)
            .field("max_connections", &self.max_connections)
            .field("max_connections_per_ip", &self.max_connections_per_ip)
            .field(
                "upgrades_per_minute_per_ip",
                &self.upgrades_per_minute_per_ip,
            )
            .finish()
    }
}

#[derive(Debug)]
pub struct ConfigError(pub String);

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ConfigError {}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::from_lookup(|k| std::env::var(k).ok())
    }

    pub fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Result<Self, ConfigError> {
        let opt = |name: &str| {
            lookup(name)
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
        };
        let secret = |name: &str| {
            let v = opt(name).ok_or_else(|| ConfigError(format!("{name} is required")))?;
            if v.len() < MIN_SECRET_BYTES {
                return Err(ConfigError(format!(
                    "{name} must be at least {MIN_SECRET_BYTES} bytes (openssl rand -hex 32)"
                )));
            }
            Ok(v)
        };
        fn num<T: std::str::FromStr>(
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
        let api_key = secret("RELAY_API_KEY")?;
        let token_secret = secret("RELAY_TOKEN_SECRET")?;
        if api_key == token_secret {
            return Err(ConfigError(
                "RELAY_API_KEY and RELAY_TOKEN_SECRET must differ".into(),
            ));
        }
        let allowed_origins: Vec<String> = opt("ALLOWED_ORIGINS")
            .ok_or_else(|| ConfigError("ALLOWED_ORIGINS is required".into()))?
            .split(',')
            .map(|o| o.trim().trim_end_matches('/').to_string())
            .filter(|o| !o.is_empty())
            .collect();
        if allowed_origins.is_empty() {
            return Err(ConfigError("ALLOWED_ORIGINS is empty".into()));
        }
        Ok(Config {
            port: num("PORT", opt("PORT"), 8090)?,
            api_key,
            token_secret,
            allowed_origins,
            trust_cf_connecting_ip: opt("TRUST_CF_CONNECTING_IP").as_deref() == Some("true"),
            max_connections: num("MAX_CONNECTIONS", opt("MAX_CONNECTIONS"), 2000)?,
            max_connections_per_ip: num(
                "MAX_CONNECTIONS_PER_IP",
                opt("MAX_CONNECTIONS_PER_IP"),
                20,
            )?,
            upgrades_per_minute_per_ip: num(
                "UPGRADES_PER_MINUTE_PER_IP",
                opt("UPGRADES_PER_MINUTE_PER_IP"),
                30,
            )?,
            auth_timeout: Duration::from_secs(5),
            // Under Cloudflare's 100 s idle cut-off for WebSockets.
            ping_interval: Duration::from_secs(30),
            idle_timeout: Duration::from_secs(75),
        })
    }

    pub fn origin_allowed(&self, origin: &str) -> bool {
        let origin = origin.trim_end_matches('/');
        self.allowed_origins.iter().any(|o| o == origin)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn load(m: &HashMap<&str, &str>) -> Result<Config, ConfigError> {
        Config::from_lookup(|k| m.get(k).map(|v| v.to_string()))
    }

    fn base() -> HashMap<&'static str, &'static str> {
        HashMap::from([
            ("RELAY_API_KEY", "a".repeat(32).leak() as &str),
            ("RELAY_TOKEN_SECRET", "b".repeat(32).leak() as &str),
            (
                "ALLOWED_ORIGINS",
                "https://ouril.vercel.app/, http://localhost:5173",
            ),
        ])
    }

    #[test]
    fn loads_with_defaults() {
        let c = load(&base()).unwrap();
        assert_eq!(c.port, 8090);
        assert!(c.origin_allowed("https://ouril.vercel.app"));
        assert!(c.origin_allowed("http://localhost:5173"));
        assert!(!c.origin_allowed("https://evil.example"));
        assert!(!c.trust_cf_connecting_ip);
        assert!(!format!("{c:?}").contains("aaaa"), "secrets are redacted");
    }

    #[test]
    fn refuses_weak_or_missing_secrets() {
        for (k, v) in [
            ("RELAY_API_KEY", "short"),
            ("RELAY_TOKEN_SECRET", ""),
            ("ALLOWED_ORIGINS", " , "),
        ] {
            let mut m = base();
            m.insert(k, v);
            assert!(load(&m).is_err(), "{k}={v:?}");
        }
        let mut same = base();
        same.insert("RELAY_TOKEN_SECRET", same["RELAY_API_KEY"]);
        assert!(load(&same).is_err());
    }
}
