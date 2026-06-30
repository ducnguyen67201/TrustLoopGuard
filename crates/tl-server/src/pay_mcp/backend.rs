//! `PayBackend` implemented over the durable repos. Workspace-bound: one
//! instance per resolved workspace, so the tool surface stays owner-keyed.

use std::sync::Arc;

use async_trait::async_trait;
use tl_core::{PayAuditEntry, PayPolicy, PayStatus};
use tl_pay_mcp::tools::status_str;
use tl_pay_mcp::{PayBackend, PayError};
use tl_storage::{PayDecisionRepo, PayDecisionRow, PayPolicyRepo, StorageError};

/// Durable [`PayBackend`] bound to a single workspace.
pub struct PayBackendImpl {
    workspace_id: String,
    policy: Arc<PayPolicyRepo>,
    decisions: Arc<PayDecisionRepo>,
}

impl PayBackendImpl {
    pub fn new(
        workspace_id: String,
        policy: Arc<PayPolicyRepo>,
        decisions: Arc<PayDecisionRepo>,
    ) -> Self {
        Self {
            workspace_id,
            policy,
            decisions,
        }
    }
}

fn map_err(e: StorageError) -> PayError {
    PayError::Backend(e.to_string())
}

fn parse_status(s: &str) -> PayStatus {
    match s {
        "allow" => PayStatus::Allow,
        "hold" => PayStatus::Hold,
        _ => PayStatus::Block,
    }
}

fn to_audit(row: PayDecisionRow) -> PayAuditEntry {
    PayAuditEntry {
        decision_id: row.decision_id,
        owner: row.owner,
        amount_minor: row.amount_minor,
        merchant: row.merchant,
        category: (!row.category.is_empty()).then_some(row.category),
        status: parse_status(&row.status),
        resolved: row.resolution,
        created_at: row.created_at.to_rfc3339(),
    }
}

#[async_trait]
impl PayBackend for PayBackendImpl {
    async fn upsert_policy(&self, policy: &PayPolicy) -> Result<(), PayError> {
        self.policy
            .upsert(&self.workspace_id, policy)
            .await
            .map_err(map_err)
    }

    async fn get_policy(&self, owner: &str) -> Result<Option<PayPolicy>, PayError> {
        self.policy
            .get(&self.workspace_id, owner)
            .await
            .map_err(map_err)
    }

    async fn spent_today(&self, owner: &str) -> Result<i64, PayError> {
        self.decisions
            .sum_today(&self.workspace_id, owner)
            .await
            .map_err(map_err)
    }

    async fn spent_this_month(&self, owner: &str) -> Result<i64, PayError> {
        self.decisions
            .sum_this_month(&self.workspace_id, owner)
            .await
            .map_err(map_err)
    }

    async fn record_decision(
        &self,
        owner: &str,
        decision_id: &str,
        amount_minor: i64,
        merchant: &str,
        category: &str,
        status: PayStatus,
    ) -> Result<(), PayError> {
        self.decisions
            .record(
                &self.workspace_id,
                owner,
                decision_id,
                amount_minor,
                merchant,
                category,
                status_str(status),
            )
            .await
            .map_err(map_err)
    }

    async fn resolve_hold(&self, decision_id: &str, approve: bool) -> Result<bool, PayError> {
        self.decisions
            .resolve_hold(&self.workspace_id, decision_id, approve)
            .await
            .map_err(map_err)
    }

    async fn list_audit(&self, owner: &str) -> Result<Vec<PayAuditEntry>, PayError> {
        let rows = self
            .decisions
            .list_for_owner(&self.workspace_id, owner)
            .await
            .map_err(map_err)?;
        Ok(rows.into_iter().map(to_audit).collect())
    }
}
