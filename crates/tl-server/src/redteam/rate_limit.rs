//! A small, dependency-free fixed-window rate limiter for the public report
//! endpoint.
//!
//! Keyed by the **resolved share token** (not client IP): the public read's only
//! direct caller is usually the dashboard's server-side fetch, so per-IP keying
//! would throttle all legitimate traffic into one bucket. Per-token keying caps
//! abuse of any single shared link and keeps the map bounded — the handler only
//! inserts a key *after* the token resolves, so invalid-token probe traffic
//! (cheap 404s) never grows the map. Expired entries are pruned on each check.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

pub struct ReportRateLimiter {
    window: Duration,
    max: u32,
    state: Mutex<HashMap<String, (Instant, u32)>>,
}

impl ReportRateLimiter {
    pub fn new(window: Duration, max: u32) -> Self {
        Self {
            window,
            max,
            state: Mutex::new(HashMap::new()),
        }
    }

    /// Record a hit for `key`; returns `true` when it is within the limit.
    /// Fails open if the lock is poisoned — a rate limiter must never take the
    /// endpoint down.
    pub fn check(&self, key: &str) -> bool {
        let now = Instant::now();
        let mut state = match self.state.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        // Drop entries whose window has elapsed so the map stays bounded.
        state.retain(|_, (start, _)| now.duration_since(*start) <= self.window);

        let entry = state.entry(key.to_string()).or_insert((now, 0));
        if now.duration_since(entry.0) > self.window {
            *entry = (now, 0);
        }
        entry.1 += 1;
        entry.1 <= self.max
    }
}

impl std::fmt::Debug for ReportRateLimiter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReportRateLimiter")
            .field("window", &self.window)
            .field("max", &self.max)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_up_to_max_then_blocks() {
        let limiter = ReportRateLimiter::new(Duration::from_secs(60), 2);
        assert!(limiter.check("rpt_a"));
        assert!(limiter.check("rpt_a"));
        assert!(!limiter.check("rpt_a")); // 3rd hit in the window is blocked
    }

    #[test]
    fn keys_are_independent() {
        let limiter = ReportRateLimiter::new(Duration::from_secs(60), 1);
        assert!(limiter.check("rpt_a"));
        assert!(!limiter.check("rpt_a"));
        // A different token has its own budget.
        assert!(limiter.check("rpt_b"));
    }

    #[test]
    fn resets_after_window() {
        let limiter = ReportRateLimiter::new(Duration::from_millis(1), 1);
        assert!(limiter.check("rpt_a"));
        std::thread::sleep(Duration::from_millis(3));
        // Window elapsed → fresh budget (and the stale entry is pruned).
        assert!(limiter.check("rpt_a"));
    }
}
