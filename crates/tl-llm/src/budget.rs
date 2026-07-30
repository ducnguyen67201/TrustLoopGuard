//! Per-tenant token budget tracking.
//!
//! In-memory in v0 — counters reset on process restart, which is fine
//! while we have one server replica per deployment. PR 11 / v1 will
//! persist these counters to Postgres so multi-instance deployments
//! and audits both see the same numbers.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use tokio::sync::{Mutex as AsyncMutex, OwnedMutexGuard};

/// Records prompt+completion tokens consumed per tenant and enforces
/// configured limits. Cheap reads/writes (HashMap behind a Mutex);
/// uncontended at expected rates (≤ a few thousand judge calls/sec).
pub struct TokenBudget {
    state: Mutex<BudgetState>,
}

struct BudgetState {
    spent: HashMap<String, u64>,
    limits: HashMap<String, u64>,
    admission: HashMap<String, Arc<AsyncMutex<()>>>,
    default_limit: u64,
}

/// Exclusive admission for one capped tenant evaluation.
///
/// Provider usage is only known after responses arrive, so a capped tenant may
/// have at most one budget session in flight. Multiple judges in the same Tier
/// 3 evaluation share a reservation and settle their combined usage when it
/// drops.
pub(crate) struct TokenBudgetReservation<'a> {
    budget: &'a TokenBudget,
    tenant: String,
    pending: AtomicU64,
    _admission: Option<OwnedMutexGuard<()>>,
}

impl TokenBudget {
    /// Build a budget where unknown tenants get `default_limit` tokens.
    /// `0` means "no limit" — useful for development and tests.
    pub fn new(default_limit: u64) -> Self {
        Self {
            state: Mutex::new(BudgetState {
                spent: HashMap::new(),
                limits: HashMap::new(),
                admission: HashMap::new(),
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

    /// Atomically admit one evaluation for `tenant`.
    ///
    /// Concurrent evaluations for a capped tenant queue behind the active
    /// reservation, then re-check committed usage before reaching a provider.
    /// Unlimited tenants are not serialized.
    pub(crate) async fn reserve(
        &self,
        tenant: &str,
    ) -> Result<TokenBudgetReservation<'_>, BudgetExceeded> {
        let admission = {
            let mut s = self.state.lock().expect("budget poisoned");
            let limit = *s.limits.get(tenant).unwrap_or(&s.default_limit);
            if limit == 0 {
                None
            } else {
                Some(
                    s.admission
                        .entry(tenant.to_string())
                        .or_insert_with(|| Arc::new(AsyncMutex::new(())))
                        .clone(),
                )
            }
        };
        let admission = match admission {
            Some(gate) => Some(gate.lock_owned().await),
            None => None,
        };

        let s = self.state.lock().expect("budget poisoned");
        let limit = *s.limits.get(tenant).unwrap_or(&s.default_limit);
        let used = *s.spent.get(tenant).unwrap_or(&0);
        if limit != 0 && used >= limit {
            return Err(BudgetExceeded { used, limit });
        }
        drop(s);

        Ok(TokenBudgetReservation {
            budget: self,
            tenant: tenant.to_string(),
            pending: AtomicU64::new(0),
            _admission: admission,
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
    pub(crate) fn record(&self, tokens: u64) {
        let _ = self
            .pending
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |current| {
                Some(current.saturating_add(tokens))
            });
    }
}

impl Drop for TokenBudgetReservation<'_> {
    fn drop(&mut self) {
        let tokens = self.pending.load(Ordering::Acquire);
        if tokens == 0 {
            return;
        }
        let mut s = self
            .budget
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let spent = s.spent.entry(self.tenant.clone()).or_insert(0);
        *spent = spent.saturating_add(tokens);
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

    #[tokio::test]
    async fn capped_tenant_queues_atomic_in_flight_reservations() {
        let b = TokenBudget::new(10);
        let reservation = b.reserve("acme").await.expect("first reservation");
        let second = b.reserve("acme");
        tokio::pin!(second);

        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(10), &mut second)
                .await
                .is_err(),
            "second reservation must wait for the first"
        );

        drop(reservation);
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(100), &mut second)
                .await
                .expect("second reservation should resume")
                .is_ok()
        );
    }

    #[tokio::test]
    async fn dropping_reservation_records_usage_and_releases_admission() {
        let b = TokenBudget::new(10);
        let reservation = b.reserve("acme").await.expect("reservation");
        reservation.record(4);
        drop(reservation);

        assert_eq!(b.used("acme"), 4);
        assert!(b.reserve("acme").await.is_ok());
    }
}
