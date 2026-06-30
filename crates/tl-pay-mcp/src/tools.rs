//! The four pay tools, generic over a [`PayBackend`]. These hold the
//! orchestration; the pure cap logic is in [`crate::evaluate`] and persistence
//! is behind the trait.

use tl_core::{PayAuditEntry, PayDecision, PayPolicy, PaySpendRequest, PayStatus};

use crate::backend::{PayBackend, PayError};
use crate::evaluate::evaluate;

/// Canonical `PayStatus` → stored-string mapping. The tl-server backend impl
/// reuses this so the persisted strings always match the evaluator's output.
pub fn status_str(status: PayStatus) -> &'static str {
    match status {
        PayStatus::Allow => "allow",
        PayStatus::Block => "block",
        PayStatus::Hold => "hold",
    }
}

/// `set_policy`: write an owner's caps.
pub async fn set_policy<B: PayBackend>(backend: &B, policy: PayPolicy) -> Result<(), PayError> {
    backend.upsert_policy(&policy).await
}

/// `pay`: gate a spend. Fails closed (block) when no policy is set. Records
/// every decision for audit; the recorded hold is what `resolve_hold` acts on.
pub async fn pay<B: PayBackend>(
    backend: &B,
    req: PaySpendRequest,
) -> Result<PayDecision, PayError> {
    let decision_id = uuid::Uuid::now_v7().to_string();
    let category = req.category.as_deref().unwrap_or("");

    if req.amount_minor <= 0 {
        // Malformed request, not a gate decision — reject without recording.
        return Ok(PayDecision {
            status: PayStatus::Block,
            reason: "amount must be positive".to_string(),
            decision_id,
        });
    }

    let Some(policy) = backend.get_policy(&req.owner).await? else {
        let reason = "no policy set for owner (fails closed)".to_string();
        backend
            .record_decision(
                &req.owner,
                &decision_id,
                req.amount_minor,
                &req.merchant,
                category,
                PayStatus::Block,
            )
            .await?;
        return Ok(PayDecision {
            status: PayStatus::Block,
            reason,
            decision_id,
        });
    };

    let spent_today = backend.spent_today(&req.owner).await?;
    let spent_month = backend.spent_this_month(&req.owner).await?;
    let (status, reason) = evaluate(&policy, spent_today, spent_month, req.amount_minor);

    backend
        .record_decision(
            &req.owner,
            &decision_id,
            req.amount_minor,
            &req.merchant,
            category,
            status,
        )
        .await?;

    Ok(PayDecision {
        status,
        reason,
        decision_id,
    })
}

/// `resolve_hold`: approve/deny a held spend. Approved holds then count toward
/// spend. Returns `false` if no matching hold exists.
pub async fn resolve_hold<B: PayBackend>(
    backend: &B,
    decision_id: &str,
    approve: bool,
) -> Result<bool, PayError> {
    backend.resolve_hold(decision_id, approve).await
}

/// `export_audit`: every decision for an owner, oldest first.
pub async fn export_audit<B: PayBackend>(
    backend: &B,
    owner: &str,
) -> Result<Vec<PayAuditEntry>, PayError> {
    backend.list_audit(owner).await
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use async_trait::async_trait;

    use super::*;

    /// Minimal in-memory backend: holds one policy, records decisions, and
    /// reports zero prior spend (enough to exercise the orchestration).
    #[derive(Default)]
    struct MockBackend {
        policy: Mutex<Option<PayPolicy>>,
        recorded: Mutex<Vec<(String, PayStatus)>>,
    }

    #[async_trait]
    impl PayBackend for MockBackend {
        async fn upsert_policy(&self, policy: &PayPolicy) -> Result<(), PayError> {
            *self.policy.lock().unwrap() = Some(policy.clone());
            Ok(())
        }
        async fn get_policy(&self, _owner: &str) -> Result<Option<PayPolicy>, PayError> {
            Ok(self.policy.lock().unwrap().clone())
        }
        async fn spent_today(&self, _owner: &str) -> Result<i64, PayError> {
            Ok(0)
        }
        async fn spent_this_month(&self, _owner: &str) -> Result<i64, PayError> {
            Ok(0)
        }
        async fn record_decision(
            &self,
            _owner: &str,
            decision_id: &str,
            _amount_minor: i64,
            _merchant: &str,
            _category: &str,
            status: PayStatus,
        ) -> Result<(), PayError> {
            self.recorded
                .lock()
                .unwrap()
                .push((decision_id.to_string(), status));
            Ok(())
        }
        async fn resolve_hold(&self, _decision_id: &str, _approve: bool) -> Result<bool, PayError> {
            Ok(true)
        }
        async fn list_audit(&self, _owner: &str) -> Result<Vec<PayAuditEntry>, PayError> {
            Ok(vec![])
        }
    }

    fn req(amount_minor: i64) -> PaySpendRequest {
        PaySpendRequest {
            owner: "me".into(),
            amount_minor,
            merchant: "Coffee".into(),
            category: None,
            memo: None,
        }
    }

    fn demo_policy() -> PayPolicy {
        PayPolicy {
            owner: "me".into(),
            per_transaction_minor: Some(10_000),
            daily_minor: None,
            monthly_minor: None,
            hold_above_minor: Some(5_000),
        }
    }

    #[tokio::test]
    async fn pay_fails_closed_without_policy() {
        let backend = MockBackend::default();
        let decision = pay(&backend, req(4_000)).await.unwrap();
        assert_eq!(decision.status, PayStatus::Block);
        assert!(decision.reason.contains("no policy"));
        // The block is recorded for audit.
        assert_eq!(backend.recorded.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn pay_allows_blocks_and_holds_per_policy() {
        let backend = MockBackend::default();
        set_policy(&backend, demo_policy()).await.unwrap();

        assert_eq!(
            pay(&backend, req(4_000)).await.unwrap().status,
            PayStatus::Allow
        );
        assert_eq!(
            pay(&backend, req(80_000)).await.unwrap().status,
            PayStatus::Block
        );
        assert_eq!(
            pay(&backend, req(6_000)).await.unwrap().status,
            PayStatus::Hold
        );
    }

    #[tokio::test]
    async fn pay_rejects_non_positive_amount_without_recording() {
        let backend = MockBackend::default();
        set_policy(&backend, demo_policy()).await.unwrap();
        let decision = pay(&backend, req(0)).await.unwrap();
        assert_eq!(decision.status, PayStatus::Block);
        assert!(decision.reason.contains("positive"));
        assert_eq!(backend.recorded.lock().unwrap().len(), 0);
    }
}
