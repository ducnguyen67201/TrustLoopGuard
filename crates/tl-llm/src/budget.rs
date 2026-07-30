//! Per-tenant token budget tracking.
//!
//! In-memory in v0 — counters reset on process restart, which is fine
//! while we have one server replica per deployment. PR 11 / v1 will
//! persist these counters to Postgres so multi-instance deployments
//! and audits both see the same numbers.

use std::collections::{HashMap, HashSet};
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
    in_flight: HashSet<String>,
    default_limit: u64,
}

/// Exclusive admission for one capped tenant call.
///
/// Provider usage is only known after a response, so a capped tenant may
/// have at most one judge call in flight. Dropping an unsettled reservation
/// releases admission without charging tokens.
pub(crate) struct TokenBudgetReservation<'a> {
    budget: &'a TokenBudget,
    tenant: String,
    exclusive: bool,
    settled: bool,
}

impl TokenBudget {
    /// Build a budget where unknown tenants get `default_limit` tokens.
    /// `0` means "no limit" — useful for development and tests.
    pub fn new(default_limit: u64) -> Self {
        Self {
            state: Mutex::new(BudgetState {
                spent: HashMap::new(),
                limits: HashMap::new(),
                in_flight: HashSet::new(),
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
        if used >= limit || s.in_flight.contains(tenant) {
            Err(BudgetExceeded { used, limit })
        } else {
            Ok(())
        }
    }

    /// Atomically admit one provider call for `tenant`.
    ///
    /// Since the provider reports token usage only after completion, capped
    /// tenants use an exclusive in-flight reservation. Unlimited tenants are
    /// not serialized.
    pub(crate) fn reserve(
        &self,
        tenant: &str,
    ) -> Result<TokenBudgetReservation<'_>, BudgetExceeded> {
        let mut s = self.state.lock().expect("budget poisoned");
        let limit = *s.limits.get(tenant).unwrap_or(&s.default_limit);
        let used = *s.spent.get(tenant).unwrap_or(&0);
        let exclusive = limit != 0;

        if exclusive && (used >= limit || s.in_flight.contains(tenant)) {
            return Err(BudgetExceeded { used, limit });
        }
        if exclusive {
            s.in_flight.insert(tenant.to_string());
        }

        Ok(TokenBudgetReservation {
            budget: self,
            tenant: tenant.to_string(),
            exclusive,
            settled: false,
        })
    }

    /// Record externally accounted usage for `tenant` without admission.
    /// Router calls must use `reserve` and settle their reservation instead.
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

impl TokenBudgetReservation<'_> {
    pub(crate) fn settle(mut self, tokens: u64) {
        let mut s = self.budget.state.lock().expect("budget poisoned");
        if self.exclusive {
            s.in_flight.remove(&self.tenant);
        }
        let spent = s.spent.entry(self.tenant.clone()).or_insert(0);
        *spent = spent.saturating_add(tokens);
        self.settled = true;
    }
}

impl Drop for TokenBudgetReservation<'_> {
    fn drop(&mut self) {
        if self.settled || !self.exclusive {
            return;
        }
        let mut s = self
            .budget
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        s.in_flight.remove(&self.tenant);
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

    #[test]
    fn capped_tenant_has_one_atomic_in_flight_reservation() {
        let b = TokenBudget::new(10);
        let reservation = b.reserve("acme").expect("first reservation");

        assert!(matches!(
            b.reserve("acme"),
            Err(BudgetExceeded { used: 0, limit: 10 })
        ));

        drop(reservation);
        assert!(b.reserve("acme").is_ok());
    }

    #[test]
    fn settling_reservation_records_usage_and_releases_admission() {
        let b = TokenBudget::new(10);
        b.reserve("acme").expect("reservation").settle(4);

        assert_eq!(b.used("acme"), 4);
        assert!(b.reserve("acme").is_ok());
    }
}
