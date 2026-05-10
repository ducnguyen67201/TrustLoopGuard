//! Per-tenant token budget tracking.
//!
//! In-memory in v0 — counters reset on process restart, which is fine
//! while we have one server replica per deployment. PR 11 / v1 will
//! persist these counters to Postgres so multi-instance deployments
//! and audits both see the same numbers.

use std::collections::HashMap;
use std::sync::Mutex;

/// Records prompt+completion tokens consumed per tenant and enforces
/// configured limits. Cheap reads/writes (HashMap behind a Mutex);
/// uncontended at expected rates (≤ a few thousand judge calls/sec).
pub struct TokenBudget {
    state: Mutex<BudgetState>,
}

struct BudgetState {
    spent: HashMap<String, u64>,
    limits: HashMap<String, u64>,
    default_limit: u64,
}

impl TokenBudget {
    /// Build a budget where unknown tenants get `default_limit` tokens.
    /// `0` means "no limit" — useful for development and tests.
    pub fn new(default_limit: u64) -> Self {
        Self {
            state: Mutex::new(BudgetState {
                spent: HashMap::new(),
                limits: HashMap::new(),
                default_limit,
            }),
        }
    }

    /// Override the limit for a specific tenant. Set to `0` to lift the cap.
    pub fn set_tenant_limit(&self, tenant: impl Into<String>, limit: u64) {
        let mut s = self.state.lock().expect("budget poisoned");
        s.limits.insert(tenant.into(), limit);
    }

    /// Returns `Ok(())` if the tenant has room; `Err(BudgetExceeded)`
    /// otherwise. `0` is treated as "no limit" — never exceeded.
    pub fn check(&self, tenant: &str) -> Result<(), BudgetExceeded> {
        let s = self.state.lock().expect("budget poisoned");
        let limit = *s.limits.get(tenant).unwrap_or(&s.default_limit);
        if limit == 0 {
            return Ok(());
        }
        let used = *s.spent.get(tenant).unwrap_or(&0);
        if used >= limit {
            Err(BudgetExceeded { used, limit })
        } else {
            Ok(())
        }
    }

    /// Record that `tokens` were consumed for `tenant`. Does not enforce
    /// the limit (the call already happened); call `check` *before* the
    /// LLM request to gate it.
    pub fn record(&self, tenant: impl Into<String>, tokens: u64) {
        let mut s = self.state.lock().expect("budget poisoned");
        *s.spent.entry(tenant.into()).or_insert(0) += tokens;
    }

    /// Total tokens spent by `tenant` in this process lifetime.
    pub fn used(&self, tenant: &str) -> u64 {
        let s = self.state.lock().expect("budget poisoned");
        *s.spent.get(tenant).unwrap_or(&0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BudgetExceeded {
    pub used: u64,
    pub limit: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_tenant_uses_default_limit() {
        let b = TokenBudget::new(100);
        assert!(b.check("acme").is_ok());
    }

    #[test]
    fn exceeding_default_limit_errors() {
        let b = TokenBudget::new(10);
        b.record("acme", 11);
        let err = b.check("acme").unwrap_err();
        assert_eq!(err.limit, 10);
        assert_eq!(err.used, 11);
    }

    #[test]
    fn tenant_limit_overrides_default() {
        let b = TokenBudget::new(100);
        b.set_tenant_limit("acme", 5);
        b.record("acme", 6);
        assert!(b.check("acme").is_err());
        // Other tenant still uses default and has plenty of room.
        assert!(b.check("other").is_ok());
    }

    #[test]
    fn zero_limit_means_unlimited() {
        let b = TokenBudget::new(0);
        b.record("acme", 10_000_000);
        assert!(b.check("acme").is_ok());
    }

    #[test]
    fn used_returns_running_total() {
        let b = TokenBudget::new(0);
        b.record("acme", 7);
        b.record("acme", 3);
        assert_eq!(b.used("acme"), 10);
    }
}
