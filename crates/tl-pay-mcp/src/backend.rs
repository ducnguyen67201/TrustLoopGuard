//! The storage seam the pay tools run against.
//!
//! Defined here — not in tl-server — so the tool handlers stay free of any
//! server or storage dependency. tl-server implements `PayBackend` over its
//! repos and mounts the tools; that inversion is what lets tl-server depend on
//! this crate without a cycle.

use async_trait::async_trait;
use tl_core::{PayAuditEntry, PayPolicy, PayStatus};

/// Failure from the durable backend. Handlers surface these to the caller; they
/// never panic on a backend error.
#[derive(Debug, thiserror::Error)]
pub enum PayError {
    #[error("payment backend error: {0}")]
    Backend(String),
    #[error("payment gate unavailable: no database configured")]
    Unavailable,
}

/// Durable operations the four pay tools need. All scoped to a single
/// workspace, which the implementor binds (so the tool surface stays
/// owner-keyed, like the original standalone gate).
#[async_trait]
pub trait PayBackend: Send + Sync {
    /// Insert or replace an owner's caps.
    async fn upsert_policy(&self, policy: &PayPolicy) -> Result<(), PayError>;

    /// Fetch an owner's caps, or `None` if none set (the gate fails closed).
    async fn get_policy(&self, owner: &str) -> Result<Option<PayPolicy>, PayError>;

    /// Spend counted toward caps today (allowed + approved holds).
    async fn spent_today(&self, owner: &str) -> Result<i64, PayError>;

    /// Spend counted toward caps this month.
    async fn spent_this_month(&self, owner: &str) -> Result<i64, PayError>;

    /// Append a decision to the audit log / hold registry.
    async fn record_decision(
        &self,
        owner: &str,
        decision_id: &str,
        amount_minor: i64,
        merchant: &str,
        category: &str,
        status: PayStatus,
    ) -> Result<(), PayError>;

    /// Approve/deny a held decision. Returns `true` if a matching hold existed.
    async fn resolve_hold(&self, decision_id: &str, approve: bool) -> Result<bool, PayError>;

    /// Every decision for an owner, oldest first.
    async fn list_audit(&self, owner: &str) -> Result<Vec<PayAuditEntry>, PayError>;
}
