//! In-process decision cache backed by [`moka`].
//!
//! Two reasons the engine consults this cache:
//! 1. Latency — a hit returns in microseconds, skipping the entire
//!    Tier 1+2+3 fan-out.
//! 2. Cost — Tier 3 LLM judges are the dominant per-request cost; any
//!    hit avoids that token spend.
//!
//! `MokaCache::disabled()` returns a cache with capacity 0 — every
//! request is a miss. Used by `HandlerCtx::no_op()` and any deployment
//! where caching is undesirable (e.g. compliance reviews wanting fresh
//! verdicts every call).

use std::time::Duration;

use moka::future::Cache;
use tl_core::Decision;

/// Default cache TTL — five minutes is enough to absorb retry storms
/// and "user clicked twice" duplicates without serving stale verdicts
/// after a policy update on the next process.
pub const DEFAULT_TTL: Duration = Duration::from_secs(5 * 60);

/// Default per-process capacity. Each entry is a `Decision` (a few hundred
/// bytes), so 10K entries ≈ a few MB resident.
pub const DEFAULT_CAPACITY: u64 = 10_000;

#[derive(Clone)]
pub struct MokaCache {
    inner: Cache<String, Decision>,
}

impl MokaCache {
    /// Build with explicit `capacity` and `ttl`. Pass `capacity = 0` to
    /// disable caching entirely (every `get` returns `None`).
    pub fn new(capacity: u64, ttl: Duration) -> Self {
        let inner = Cache::builder()
            .max_capacity(capacity)
            .time_to_live(ttl)
            .build();
        Self { inner }
    }

    /// Sensible defaults for production: 10K entries, 5-minute TTL.
    pub fn with_defaults() -> Self {
        Self::new(DEFAULT_CAPACITY, DEFAULT_TTL)
    }

    /// A cache that never stores anything. Equivalent to running with
    /// caching turned off; useful in tests and as a `no_op()` placeholder.
    pub fn disabled() -> Self {
        Self::new(0, Duration::from_secs(1))
    }

    pub async fn get(&self, key: &str) -> Option<Decision> {
        self.inner.get(key).await
    }

    pub async fn put(&self, key: String, decision: Decision) {
        self.inner.insert(key, decision).await;
    }

    /// Approximate number of cached entries. Useful for tests and
    /// diagnostics; not a strong invariant under concurrent access.
    pub fn len(&self) -> u64 {
        self.inner.entry_count()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.entry_count() == 0
    }
}

impl Default for MokaCache {
    fn default() -> Self {
        Self::with_defaults()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tl_core::Decision;

    fn fake_decision(trace: &str) -> Decision {
        let mut d = Decision::allow(trace);
        d.latency_ms = 7;
        d
    }

    #[tokio::test]
    async fn put_then_get_returns_value() {
        let c = MokaCache::with_defaults();
        c.put("key1".into(), fake_decision("t-1")).await;
        let got = c.get("key1").await.expect("hit");
        assert_eq!(got.trace_id, "t-1");
        assert_eq!(got.latency_ms, 7);
    }

    #[tokio::test]
    async fn miss_returns_none() {
        let c = MokaCache::with_defaults();
        assert!(c.get("nope").await.is_none());
    }

    #[tokio::test]
    async fn disabled_cache_never_stores() {
        let c = MokaCache::disabled();
        c.put("key1".into(), fake_decision("t-1")).await;
        // moka's eviction is async; sync_all forces it.
        c.inner.run_pending_tasks().await;
        assert!(c.get("key1").await.is_none());
    }

    #[tokio::test]
    async fn ttl_expires_old_entries() {
        let c = MokaCache::new(100, Duration::from_millis(50));
        c.put("ephemeral".into(), fake_decision("t-1")).await;
        assert!(c.get("ephemeral").await.is_some());
        tokio::time::sleep(Duration::from_millis(80)).await;
        c.inner.run_pending_tasks().await;
        assert!(c.get("ephemeral").await.is_none());
    }

    #[tokio::test]
    async fn put_overwrites_existing_key() {
        let c = MokaCache::with_defaults();
        c.put("k".into(), fake_decision("first")).await;
        c.put("k".into(), fake_decision("second")).await;
        assert_eq!(c.get("k").await.unwrap().trace_id, "second");
    }
}
