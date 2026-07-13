use std::sync::Arc;

use async_trait::async_trait;

use crate::financial::{
    FinancialBudgetReservationOutcome, FinancialBudgetReservationRequest, FinancialBudgetViolation,
    FinancialBudgetWindow, FinancialLedgerEntryKind, FinancialStore, FinancialStoreError,
};

pub struct PostgresFinancialAdapter(pub Arc<tl_storage::FinancialRepo>);

impl PostgresFinancialAdapter {
    pub fn new(repo: Arc<tl_storage::FinancialRepo>) -> Arc<Self> {
        Arc::new(Self(repo))
    }
}

#[async_trait]
impl FinancialStore for PostgresFinancialAdapter {
    async fn create_action(
        &self,
        workspace_id: &str,
        input: tl_core::CreateFinancialActionRequest,
    ) -> Result<tl_core::FinancialActionRecord, FinancialStoreError> {
        self.0
            .create_action(workspace_id, input)
            .await
            .map(stored_action_record)
            .map_err(financial_store_error)
    }

    async fn get_action(
        &self,
        workspace_id: &str,
        action_id: &str,
    ) -> Result<tl_core::FinancialActionRecord, FinancialStoreError> {
        self.0
            .get_action(workspace_id, action_id)
            .await
            .map(stored_action_record)
            .map_err(financial_store_error)
    }

    async fn list_actions(
        &self,
        workspace_id: &str,
    ) -> Result<tl_core::FinancialActionListResponse, FinancialStoreError> {
        let actions = self
            .0
            .list_actions(workspace_id)
            .await
            .map_err(financial_store_error)?
            .into_iter()
            .map(stored_action_record)
            .collect();
        Ok(tl_core::FinancialActionListResponse { actions })
    }

    async fn create_mandate(
        &self,
        workspace_id: &str,
        input: tl_core::CreateFinancialMandateRequest,
    ) -> Result<tl_core::FinancialMandate, FinancialStoreError> {
        self.0
            .create_mandate(workspace_id, input)
            .await
            .map(Into::into)
            .map_err(financial_store_error)
    }

    async fn list_mandates(
        &self,
        workspace_id: &str,
    ) -> Result<tl_core::FinancialMandateListResponse, FinancialStoreError> {
        let mandates = self
            .0
            .list_mandates(workspace_id)
            .await
            .map_err(financial_store_error)?
            .into_iter()
            .map(Into::into)
            .collect();
        Ok(tl_core::FinancialMandateListResponse { mandates })
    }

    async fn get_mandate(
        &self,
        workspace_id: &str,
        mandate_id: &str,
        version: Option<i32>,
    ) -> Result<tl_core::FinancialMandate, FinancialStoreError> {
        self.0
            .get_mandate(workspace_id, mandate_id, version)
            .await
            .map(Into::into)
            .map_err(financial_store_error)
    }

    async fn revoke_mandate(
        &self,
        workspace_id: &str,
        mandate_id: &str,
    ) -> Result<tl_core::FinancialMandate, FinancialStoreError> {
        self.0
            .revoke_mandate(workspace_id, mandate_id)
            .await
            .map(Into::into)
            .map_err(financial_store_error)
    }

    async fn create_receipt(
        &self,
        workspace_id: &str,
        action_id: &str,
        trace_id: Option<&str>,
        ledger_event_ids: Vec<String>,
        proof: serde_json::Value,
    ) -> Result<tl_core::FinancialReceipt, FinancialStoreError> {
        self.0
            .create_receipt(workspace_id, action_id, trace_id, ledger_event_ids, proof)
            .await
            .map(Into::into)
            .map_err(financial_store_error)
    }

    async fn get_receipt(
        &self,
        workspace_id: &str,
        receipt_id: &str,
    ) -> Result<tl_core::FinancialReceipt, FinancialStoreError> {
        self.0
            .get_receipt(workspace_id, receipt_id)
            .await
            .map(Into::into)
            .map_err(financial_store_error)
    }

    async fn record_action_outcome(
        &self,
        workspace_id: &str,
        action_id: &str,
        outcome: tl_core::FinancialActionOutcome,
    ) -> Result<tl_core::FinancialActionOutcome, FinancialStoreError> {
        self.0
            .record_action_outcome(workspace_id, action_id, outcome)
            .await
            .map(Into::into)
            .map_err(financial_store_error)
    }

    async fn list_action_outcomes(
        &self,
        workspace_id: &str,
        action_id: &str,
    ) -> Result<tl_core::FinancialOutcomeListResponse, FinancialStoreError> {
        let outcomes = self
            .0
            .list_action_outcomes(workspace_id, action_id)
            .await
            .map_err(financial_store_error)?
            .into_iter()
            .map(Into::into)
            .collect();
        Ok(tl_core::FinancialOutcomeListResponse { outcomes })
    }

    async fn create_approval_request(
        &self,
        workspace_id: &str,
        action_id: &str,
        approval: tl_core::ApprovalRequirement,
    ) -> Result<tl_core::FinancialApprovalRequest, FinancialStoreError> {
        let expires_at = approval
            .expires_at
            .as_deref()
            .map(chrono::DateTime::parse_from_rfc3339)
            .transpose()
            .map_err(|e| FinancialStoreError::Validation(format!("expires_at: {e}")))?
            .map(|dt| dt.with_timezone(&chrono::Utc));
        self.0
            .create_approval_request(
                workspace_id,
                action_id,
                &approval.reason,
                approval.approver_roles,
                expires_at,
                serde_json::json!({}),
            )
            .await
            .map(stored_approval_request)
            .map_err(financial_store_error)
    }

    async fn list_approval_requests(
        &self,
        workspace_id: &str,
    ) -> Result<tl_core::FinancialApprovalRequestListResponse, FinancialStoreError> {
        let approval_requests = self
            .0
            .list_approval_requests(workspace_id)
            .await
            .map_err(financial_store_error)?
            .into_iter()
            .map(stored_approval_request)
            .collect();
        Ok(tl_core::FinancialApprovalRequestListResponse { approval_requests })
    }

    async fn has_current_approved_request(
        &self,
        workspace_id: &str,
        action_id: &str,
    ) -> Result<bool, FinancialStoreError> {
        self.0
            .has_current_approved_request(workspace_id, action_id)
            .await
            .map_err(financial_store_error)
    }

    async fn resolve_pending_approval_requests(
        &self,
        workspace_id: &str,
        action_id: &str,
        status: tl_core::FinancialApprovalRequestStatus,
        decided_by: Option<&str>,
    ) -> Result<(), FinancialStoreError> {
        self.0
            .resolve_pending_approval_requests(workspace_id, action_id, status, decided_by)
            .await
            .map_err(financial_store_error)
    }

    async fn transition_action(
        &self,
        workspace_id: &str,
        action_id: &str,
        status: tl_core::FinancialActionStatus,
        event_type: &str,
    ) -> Result<tl_core::FinancialActionRecord, FinancialStoreError> {
        self.0
            .transition_status(
                workspace_id,
                action_id,
                status,
                event_type,
                serde_json::json!({}),
            )
            .await
            .map(stored_action_record)
            .map_err(financial_store_error)
    }

    async fn transition_action_with_reason(
        &self,
        workspace_id: &str,
        action_id: &str,
        status: tl_core::FinancialActionStatus,
        event_type: &str,
        reason: &str,
    ) -> Result<tl_core::FinancialActionRecord, FinancialStoreError> {
        self.0
            .transition_status(
                workspace_id,
                action_id,
                status,
                event_type,
                serde_json::json!({ "reason": reason }),
            )
            .await
            .map(stored_action_record)
            .map_err(financial_store_error)
    }

    async fn record_ledger_entry(
        &self,
        workspace_id: &str,
        action_id: &str,
        kind: FinancialLedgerEntryKind,
        amount_minor: i64,
        currency: &str,
        idempotency_key: &str,
        metadata: serde_json::Value,
    ) -> Result<String, FinancialStoreError> {
        self.0
            .record_ledger_entry(
                workspace_id,
                action_id,
                storage_ledger_kind(kind),
                amount_minor,
                currency,
                idempotency_key,
                metadata,
            )
            .await
            .map(|entry| entry.id)
            .map_err(financial_store_error)
    }

    async fn ledger_entry_exists(
        &self,
        workspace_id: &str,
        idempotency_key: &str,
    ) -> Result<bool, FinancialStoreError> {
        self.0
            .ledger_entry_exists(workspace_id, idempotency_key)
            .await
            .map_err(financial_store_error)
    }

    async fn net_spend_minor(
        &self,
        workspace_id: &str,
        principal_id: &str,
        currency: &str,
        start: chrono::DateTime<chrono::Utc>,
        end: chrono::DateTime<chrono::Utc>,
    ) -> Result<i64, FinancialStoreError> {
        self.0
            .net_spend_minor(workspace_id, principal_id, currency, start, end)
            .await
            .map_err(financial_store_error)
    }

    async fn try_reserve_action_budget(
        &self,
        request: FinancialBudgetReservationRequest,
    ) -> Result<FinancialBudgetReservationOutcome, FinancialStoreError> {
        self.0
            .reserve_action_budget(tl_storage::ReserveFinancialActionBudgetRequest {
                workspace_id: request.workspace_id,
                action_id: request.action_id,
                principal_id: request.principal_id,
                amount: request.amount,
                idempotency_key: request.idempotency_key,
                day_start: request.day_start,
                week_start: request.week_start,
                month_start: request.month_start,
                now: request.now,
                constraints: request
                    .constraints
                    .into_iter()
                    .map(|constraint| tl_storage::FinancialBudgetConstraint {
                        policy_id: constraint.policy_id,
                        window: storage_budget_window(constraint.window),
                        cap_minor: constraint.cap_minor,
                        block_on_breach: constraint.block_on_breach,
                    })
                    .collect(),
                metadata: request.metadata,
            })
            .await
            .map(|outcome| match outcome {
                tl_storage::ReserveFinancialActionBudgetResult::Reserved {
                    ledger_entry_id,
                    violations,
                } => FinancialBudgetReservationOutcome::Reserved {
                    ledger_entry_id,
                    violations: violations
                        .into_iter()
                        .map(server_budget_violation)
                        .collect(),
                },
                tl_storage::ReserveFinancialActionBudgetResult::Denied { violations } => {
                    FinancialBudgetReservationOutcome::Denied {
                        violations: violations
                            .into_iter()
                            .map(server_budget_violation)
                            .collect(),
                    }
                }
            })
            .map_err(financial_store_error)
    }

    async fn try_reserve_agentic_payment_budget(
        &self,
        request: crate::financial::AgenticPaymentBudgetReservationRequest,
    ) -> Result<tl_core::AgenticPaymentReservation, FinancialStoreError> {
        self.0
            .try_reserve_agentic_payment_budget(tl_storage::ReserveAgenticPaymentBudgetRequest {
                workspace_id: request.workspace_id,
                session_id: request.session_id,
                principal_id: request.principal_id,
                action_id: request.action_id,
                payment_requirement_hash: request.payment_requirement_hash,
                amount: request.amount,
                session_limit_minor: request.session_limit_minor,
                expires_at: request.expires_at,
                metadata: request.metadata,
            })
            .await
            .map_err(financial_store_error)
    }

    async fn get_agentic_payment_reservation(
        &self,
        workspace_id: &str,
        action_id: &str,
    ) -> Result<tl_core::AgenticPaymentReservation, FinancialStoreError> {
        self.0
            .get_agentic_payment_reservation(workspace_id, action_id)
            .await
            .map_err(financial_store_error)
    }

    async fn commit_agentic_payment_reservation(
        &self,
        workspace_id: &str,
        action_id: &str,
        proof: serde_json::Value,
    ) -> Result<tl_core::AgenticPaymentReservation, FinancialStoreError> {
        self.0
            .commit_agentic_payment_reservation(workspace_id, action_id, proof)
            .await
            .map_err(financial_store_error)
    }

    async fn release_agentic_payment_reservation(
        &self,
        workspace_id: &str,
        action_id: &str,
        reason: &str,
        metadata: serde_json::Value,
    ) -> Result<tl_core::AgenticPaymentReservation, FinancialStoreError> {
        self.0
            .release_agentic_payment_reservation(workspace_id, action_id, reason, metadata)
            .await
            .map_err(financial_store_error)
    }
}

fn storage_ledger_kind(kind: FinancialLedgerEntryKind) -> tl_storage::FinancialLedgerEntryKind {
    match kind {
        FinancialLedgerEntryKind::Reserved => tl_storage::FinancialLedgerEntryKind::Reserved,
        FinancialLedgerEntryKind::Released => tl_storage::FinancialLedgerEntryKind::Released,
        FinancialLedgerEntryKind::Executed => tl_storage::FinancialLedgerEntryKind::Executed,
        FinancialLedgerEntryKind::Reversed => tl_storage::FinancialLedgerEntryKind::Reversed,
    }
}

fn storage_budget_window(window: FinancialBudgetWindow) -> tl_storage::FinancialBudgetWindow {
    match window {
        FinancialBudgetWindow::Day => tl_storage::FinancialBudgetWindow::Day,
        FinancialBudgetWindow::Week => tl_storage::FinancialBudgetWindow::Week,
        FinancialBudgetWindow::Month => tl_storage::FinancialBudgetWindow::Month,
    }
}

fn server_budget_violation(
    violation: tl_storage::FinancialBudgetViolation,
) -> FinancialBudgetViolation {
    FinancialBudgetViolation {
        policy_id: violation.policy_id,
        window: match violation.window {
            tl_storage::FinancialBudgetWindow::Day => FinancialBudgetWindow::Day,
            tl_storage::FinancialBudgetWindow::Week => FinancialBudgetWindow::Week,
            tl_storage::FinancialBudgetWindow::Month => FinancialBudgetWindow::Month,
        },
        cap_minor: violation.cap_minor,
        committed_minor: violation.committed_minor,
        requested_minor: violation.requested_minor,
        block_on_breach: violation.block_on_breach,
    }
}

fn stored_approval_request(
    row: tl_storage::StoredFinancialApprovalRequest,
) -> tl_core::FinancialApprovalRequest {
    tl_core::FinancialApprovalRequest {
        id: row.id,
        workspace_id: row.workspace_id,
        action_id: row.action_id,
        status: row.status,
        reason: row.reason,
        approver_roles: row.approver_roles,
        decided_by: row.decided_by,
        decided_at: row.decided_at.map(|value| value.to_rfc3339()),
        expires_at: row.expires_at.map(|value| value.to_rfc3339()),
        metadata: row.metadata,
        created_at: row.created_at.to_rfc3339(),
        updated_at: row.updated_at.to_rfc3339(),
    }
}

fn stored_action_record(row: tl_storage::StoredFinancialAction) -> tl_core::FinancialActionRecord {
    tl_core::FinancialActionRecord {
        id: row.id,
        workspace_id: row.workspace_id,
        status: row.status,
        status_reason: row.status_reason,
        action: row.action,
        evidence: row.evidence,
        created_at: row.created_at.to_rfc3339(),
        updated_at: row.updated_at.to_rfc3339(),
    }
}

fn financial_store_error(error: tl_storage::StorageError) -> FinancialStoreError {
    match error {
        tl_storage::StorageError::NotFound => FinancialStoreError::NotFound,
        tl_storage::StorageError::Conflict => FinancialStoreError::Conflict,
        tl_storage::StorageError::Internal(message)
            if message.contains("must") || message.contains("invalid") =>
        {
            FinancialStoreError::Validation(message)
        }
        tl_storage::StorageError::Internal(message) => FinancialStoreError::Internal(message),
    }
}
