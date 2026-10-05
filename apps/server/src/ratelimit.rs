//! Rate limiting: an in-memory fixed-window counter per client network and route class,
//! applied as a tower layer (`axum::middleware`).
//!
//! - **Keys:** IPv4 addresses as-is; IPv6 by /64 prefix, so rotating addresses inside one
//!   allocation doesn't escape the limit.
//! - **Classes:** only the sign-in endpoints are in the strict `Auth` class. Refresh and logout
//!   need a valid 256-bit refresh token (they can't be brute-forced), so they share the
//!   `Default` budget. This keeps players behind carrier-grade NAT (one public IP for many
//!   phones, common on mobile networks) from locking each other out of their sessions.
//! - **Memory:** at most [`MAX_BUCKETS`] entries. Expired windows are pruned at most once per
//!   [`PRUNE_EVERY`] (never per request). If the map is still full (many fresh addresses within
//!   one window), new clients are allowed through untracked rather than locked out: per-IP
//!   limits can't stop an attacker with that many addresses anyway, and a warning is logged.
//!
//! Limits are per machine (not shared between Fly machines) and reset on restart. Configure
//! them with `RATE_LIMIT_AUTH_PER_MINUTE` / `RATE_LIMIT_DEFAULT_PER_MINUTE`. Move to
//! Valkey-backed counters when multiplayer brings Valkey (API.md open question).

use std::{
    collections::HashMap,
    net::{IpAddr, Ipv6Addr, SocketAddr},
    sync::Mutex,
    time::{Duration, Instant},
};

use axum::{
    extract::{ConnectInfo, Request, State},
    middleware::Next,
    response::{IntoResponse, Response},
};

use crate::{error::AppError, state::AppState};

const WINDOW: Duration = Duration::from_secs(60);
/// Default sign-in limit per client network per minute (`RATE_LIMIT_AUTH_PER_MINUTE`).
pub const AUTH_LIMIT_PER_MINUTE: u32 = 60;
/// Default limit for everything else under `/v1` (`RATE_LIMIT_DEFAULT_PER_MINUTE`).
pub const DEFAULT_LIMIT_PER_MINUTE: u32 = 600;
/// Hard cap on tracked (client, class) windows.
pub const MAX_BUCKETS: usize = 50_000;
/// Expired windows are pruned at most this often.
pub const PRUNE_EVERY: Duration = Duration::from_secs(10);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RouteClass {
    /// Sign-in: `/v1/auth/{dev,google,apple}`.
    Auth,
    Default,
}

impl RouteClass {
    pub fn of(path: &str) -> RouteClass {
        match path {
            "/v1/auth/google" | "/v1/auth/apple" => RouteClass::Auth,
            // Only named when the route exists: the release binary must not contain it (CI).
            #[cfg(feature = "dev-auth")]
            "/v1/auth/dev" => RouteClass::Auth,
            _ => RouteClass::Default,
        }
    }
}

/// Who a request counts against: an IPv4 address or an IPv6 /64 network.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ClientKey {
    V4([u8; 4]),
    V6Net([u8; 8]),
    Unknown,
}

impl ClientKey {
    pub fn of(ip: IpAddr) -> ClientKey {
        match ip {
            IpAddr::V4(v4) => ClientKey::V4(v4.octets()),
            IpAddr::V6(v6) => match v6.to_ipv4_mapped() {
                Some(v4) => ClientKey::V4(v4.octets()),
                None => ClientKey::V6Net(v6_prefix64(v6)),
            },
        }
    }
}

fn v6_prefix64(ip: Ipv6Addr) -> [u8; 8] {
    let o = ip.octets();
    [o[0], o[1], o[2], o[3], o[4], o[5], o[6], o[7]]
}

#[derive(Debug)]
struct Buckets {
    map: HashMap<(ClientKey, RouteClass), (Instant, u32)>,
    last_prune: Option<Instant>,
}

#[derive(Debug)]
pub struct RateLimiter {
    auth_limit: u32,
    default_limit: u32,
    buckets: Mutex<Buckets>,
}

impl Default for RateLimiter {
    fn default() -> Self {
        RateLimiter::new(AUTH_LIMIT_PER_MINUTE, DEFAULT_LIMIT_PER_MINUTE)
    }
}

impl RateLimiter {
    pub fn new(auth_limit: u32, default_limit: u32) -> Self {
        RateLimiter {
            auth_limit,
            default_limit,
            buckets: Mutex::new(Buckets {
                map: HashMap::new(),
                last_prune: None,
            }),
        }
    }

    pub fn limit(&self, class: RouteClass) -> u32 {
        match class {
            RouteClass::Auth => self.auth_limit,
            RouteClass::Default => self.default_limit,
        }
    }

    /// Count one request. `Err(retry_after_secs)` when over the limit.
    pub fn check(&self, key: ClientKey, class: RouteClass, now: Instant) -> Result<(), u64> {
        let limit = self.limit(class);
        let mut b = self.buckets.lock().unwrap_or_else(|p| p.into_inner());
        let due = b
            .last_prune
            .is_none_or(|t| now.duration_since(t) >= PRUNE_EVERY);
        if due && b.map.len() >= MAX_BUCKETS / 2 {
            b.map
                .retain(|_, (start, _)| now.duration_since(*start) < WINDOW);
            b.last_prune = Some(now);
        }
        let k = (key, class);
        if !b.map.contains_key(&k) && b.map.len() >= MAX_BUCKETS {
            // Full of live windows: let new clients through untracked (see module docs).
            tracing::warn!(
                tracked = b.map.len(),
                "rate limiter full; new client not tracked"
            );
            return Ok(());
        }
        let entry = b.map.entry(k).or_insert((now, 0));
        if now.duration_since(entry.0) >= WINDOW {
            *entry = (now, 0);
        }
        if entry.1 >= limit {
            let retry = WINDOW.saturating_sub(now.duration_since(entry.0));
            return Err(retry.as_secs().max(1));
        }
        entry.1 += 1;
        Ok(())
    }

    #[cfg(test)]
    fn tracked(&self) -> usize {
        self.buckets.lock().unwrap().map.len()
    }
}

/// The client address. With `trust_fly_client_ip` (`Config`, set from `FLY_APP_NAME`) the Fly
/// proxy's `Fly-Client-IP` header is trusted; otherwise only the TCP peer address is used, so
/// the header can't be spoofed.
fn client_ip(req: &Request, trust_fly_client_ip: bool) -> Option<IpAddr> {
    if trust_fly_client_ip {
        if let Some(ip) = req
            .headers()
            .get("fly-client-ip")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.trim().parse::<IpAddr>().ok())
        {
            return Some(ip);
        }
    }
    req.extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|ci| ci.0.ip())
}

pub async fn rate_limit(State(state): State<AppState>, req: Request, next: Next) -> Response {
    // Original path: inside the nested `/v1` router `req.uri()` lacks the `/v1` prefix.
    let class = RouteClass::of(crate::client::full_path(&req));
    let key =
        client_ip(&req, state.config.trust_fly_client_ip).map_or(ClientKey::Unknown, ClientKey::of);
    if let Err(retry_after) = state.rate_limiter.check(key, class, Instant::now()) {
        tracing::warn!(?class, "rate limited");
        return AppError::rate_limited(retry_after).into_response();
    }
    next.run(req).await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v4(s: &str) -> ClientKey {
        ClientKey::of(s.parse().unwrap())
    }

    #[test]
    fn limits_then_resets() {
        let rl = RateLimiter::new(3, 5);
        let t0 = Instant::now();
        for _ in 0..3 {
            assert!(rl.check(v4("1.2.3.4"), RouteClass::Auth, t0).is_ok());
        }
        let retry = rl.check(v4("1.2.3.4"), RouteClass::Auth, t0).unwrap_err();
        assert!((1..=60).contains(&retry));
        // Other clients and other classes are independent.
        assert!(rl.check(v4("5.6.7.8"), RouteClass::Auth, t0).is_ok());
        assert!(rl.check(v4("1.2.3.4"), RouteClass::Default, t0).is_ok());
        // A new window starts after a minute.
        assert!(rl
            .check(v4("1.2.3.4"), RouteClass::Auth, t0 + WINDOW)
            .is_ok());
    }

    #[test]
    fn ipv6_is_keyed_by_64_prefix() {
        let a = ClientKey::of("2001:db8:1:2::1".parse().unwrap());
        let b = ClientKey::of("2001:db8:1:2:ffff:ffff:ffff:ffff".parse().unwrap());
        let other_net = ClientKey::of("2001:db8:1:3::1".parse().unwrap());
        assert_eq!(a, b);
        assert_ne!(a, other_net);
        // IPv4-mapped IPv6 counts as the IPv4 address.
        assert_eq!(
            ClientKey::of("::ffff:1.2.3.4".parse().unwrap()),
            v4("1.2.3.4")
        );

        let rl = RateLimiter::new(2, 100);
        let t0 = Instant::now();
        assert!(rl.check(a, RouteClass::Auth, t0).is_ok());
        assert!(rl.check(b, RouteClass::Auth, t0).is_ok());
        assert!(
            rl.check(a, RouteClass::Auth, t0).is_err(),
            "same /64 shares the limit"
        );
        assert!(rl.check(other_net, RouteClass::Auth, t0).is_ok());
    }

    #[test]
    fn memory_is_bounded_and_new_clients_are_not_locked_out() {
        let rl = RateLimiter::new(1, 1);
        let t0 = Instant::now();
        let key = |i: u32| ClientKey::V4(i.to_be_bytes());
        for i in 0..(MAX_BUCKETS as u32 + 1_000) {
            assert!(rl.check(key(i), RouteClass::Default, t0).is_ok());
        }
        assert!(rl.tracked() <= MAX_BUCKETS);
        // Tracked clients are still limited while full.
        assert!(rl.check(key(0), RouteClass::Default, t0).is_err());
        // After the windows expire, the next prune frees the map.
        let later = t0 + WINDOW + PRUNE_EVERY;
        assert!(rl.check(key(u32::MAX), RouteClass::Default, later).is_ok());
        assert!(rl.tracked() < 10);
    }

    #[test]
    fn fly_client_ip_only_when_trusted() {
        let mut req = Request::builder()
            .header("fly-client-ip", "203.0.113.7")
            .body(axum::body::Body::empty())
            .unwrap();
        req.extensions_mut()
            .insert(ConnectInfo(SocketAddr::from(([10, 0, 0, 1], 4000))));
        assert_eq!(client_ip(&req, true), Some("203.0.113.7".parse().unwrap()));
        assert_eq!(client_ip(&req, false), Some("10.0.0.1".parse().unwrap()));
        // An unparsable header falls back to the peer address.
        req.headers_mut()
            .insert("fly-client-ip", "not-an-ip".parse().unwrap());
        assert_eq!(client_ip(&req, true), Some("10.0.0.1".parse().unwrap()));
    }

    #[test]
    fn only_sign_in_is_in_the_auth_class() {
        assert_eq!(RouteClass::of("/v1/auth/google"), RouteClass::Auth);
        assert_eq!(RouteClass::of("/v1/auth/apple"), RouteClass::Auth);
        #[cfg(feature = "dev-auth")]
        assert_eq!(RouteClass::of("/v1/auth/dev"), RouteClass::Auth);
        assert_eq!(RouteClass::of("/v1/auth/refresh"), RouteClass::Default);
        assert_eq!(RouteClass::of("/v1/auth/logout"), RouteClass::Default);
        assert_eq!(RouteClass::of("/v1/sync/push"), RouteClass::Default);
    }
}
