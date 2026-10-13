//! Connection limits: total and per client IP (held for a connection's lifetime by a
//! [`Slot`]), and WebSocket upgrades per IP per minute.

use std::{
    collections::HashMap,
    net::IpAddr,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

const WINDOW: Duration = Duration::from_secs(60);
/// Most IPs whose upgrade counts are tracked; past it, the oldest windows are dropped.
const MAX_TRACKED_IPS: usize = 50_000;

pub struct Limits {
    max_total: usize,
    max_per_ip: usize,
    upgrades_per_minute: u32,
    state: Mutex<State>,
}

#[derive(Default)]
struct State {
    total: usize,
    per_ip: HashMap<IpAddr, usize>,
    upgrades: HashMap<IpAddr, (Instant, u32)>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Refused {
    /// Too many upgrades from this IP in the last minute.
    RateLimited,
    /// The relay, or this IP, is at its connection limit.
    Full,
}

/// A held connection slot; dropping it frees the slot.
pub struct Slot {
    limits: Arc<Limits>,
    ip: IpAddr,
}

impl Drop for Slot {
    fn drop(&mut self) {
        let mut s = self.limits.state.lock().expect("limits lock");
        s.total = s.total.saturating_sub(1);
        if let Some(n) = s.per_ip.get_mut(&self.ip) {
            *n -= 1;
            if *n == 0 {
                s.per_ip.remove(&self.ip);
            }
        }
    }
}

impl Limits {
    pub fn new(max_total: usize, max_per_ip: usize, upgrades_per_minute: u32) -> Arc<Self> {
        Arc::new(Limits {
            max_total,
            max_per_ip,
            upgrades_per_minute,
            state: Mutex::default(),
        })
    }

    /// Count an upgrade attempt from `ip` and, if allowed, take a connection slot.
    pub fn admit(self: &Arc<Self>, ip: IpAddr) -> Result<Slot, Refused> {
        let mut s = self.state.lock().expect("limits lock");
        let now = Instant::now();
        if s.upgrades.len() >= MAX_TRACKED_IPS {
            s.upgrades
                .retain(|_, (start, _)| now.duration_since(*start) < WINDOW);
        }
        let window = s.upgrades.entry(ip).or_insert((now, 0));
        if now.duration_since(window.0) >= WINDOW {
            *window = (now, 0);
        }
        window.1 += 1;
        if window.1 > self.upgrades_per_minute {
            return Err(Refused::RateLimited);
        }
        let here = s.per_ip.get(&ip).copied().unwrap_or(0);
        if s.total >= self.max_total || here >= self.max_per_ip {
            return Err(Refused::Full);
        }
        s.total += 1;
        *s.per_ip.entry(ip).or_insert(0) += 1;
        Ok(Slot {
            limits: Arc::clone(self),
            ip,
        })
    }

    pub fn connections(&self) -> usize {
        self.state.lock().expect("limits lock").total
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ip(n: u8) -> IpAddr {
        IpAddr::from([10, 0, 0, n])
    }

    #[test]
    fn slots_are_counted_per_ip_and_freed_on_drop() {
        let l = Limits::new(3, 2, 100);
        let a = l.admit(ip(1)).unwrap();
        let _b = l.admit(ip(1)).unwrap();
        assert_eq!(l.admit(ip(1)).err(), Some(Refused::Full), "two per IP");
        let _c = l.admit(ip(2)).unwrap();
        assert_eq!(l.admit(ip(3)).err(), Some(Refused::Full), "three in total");
        drop(a);
        assert!(l.admit(ip(1)).is_ok());
    }

    #[test]
    fn upgrades_are_rate_limited_per_ip() {
        let l = Limits::new(100, 100, 2);
        let _a = l.admit(ip(1)).unwrap();
        let _b = l.admit(ip(1)).unwrap();
        assert_eq!(l.admit(ip(1)).err(), Some(Refused::RateLimited));
        assert!(l.admit(ip(2)).is_ok(), "another IP has its own window");
    }
}
