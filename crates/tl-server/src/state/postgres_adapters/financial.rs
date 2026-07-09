use std::sync::Arc;

use async_trait::async_trait;

use crate::financial::{
    FinancialExecutionFinalization, FinancialLedgerEntryKind, FinancialStore, FinancialStoreError,
    StoredFinancialExecutionConnector,
};

pub struct PostgresFinancialAdapter(pub Arc<tl_storage::FinancialRepo>);

impl PostgresFinancialAdapter {
    pub fn new(repo: Arc<tl_storage::FinancialRepo>) -> Arc<Self> {
        Arc::new(Self(repo))
    }

    async fn enrich_action(
        &self,
        action: tl_storage::StoredFinancialAction,
    ) -> Result<tl_core::FinancialActionRecord, FinancialStoreError> {
        let workspace_id = action.workspace_id.clone();
        let action_id = action.id.clone();
        let mut record = stored_action_record(action);
        match self
            .0
            .get_action_evaluation(&workspace_id, &action_id)
            .await
        {
            Ok(evaluation) => {
                record.runtime_mode = Some(evaluation.runtime_mode);
                record.evaluation = Some(evaluation);
            }
            Err(tl_storage::StorageError::NotFound) => {}
            Err(error) => return Err(financial_store_error(error)),
        }
        match self.0.get_execution_grant(&workspace_id, &action_id).await {
            Ok(grant) => record.execution_grant = Some(grant),
            Err(tl_storage::StorageError::NotFound) => {}
            Err(error) => return Err(financial_store_error(error)),
        }
        Ok(record)
    }
}

#[async_trait]
impl FinancialStore for PostgresFinancialAdapter {
    async fn create_action(
        &self,
        workspace_id: &str,
        environment_id: &str,
        input: tl_core::CreateFinancialActionRequest,
    ) -> Result<tl_core::FinancialActionRecord, FinancialStoreError> {
        let action = self
            .0
            .create_action_in_environment(workspace_id, environment_id, input)
            .await
            .map_err(financial_store_error)?;
        self.enrich_action(action).await
    }

    async fn get_action(
        &self,
        workspace_id: &str,
        action_id: &str,
    ) -> Result<tl_core::FinancialActionRecord, FinancialStoreError> {
        let action = self
            .0
            .get_action(workspace_id, action_id)
            .await
            .map_err(financial_store_error)?;
        self.enrich_action(action).await
    }

    async fn list_actions(
        &self,
        workspace_id: &str,
    ) -> Result<tl_core::FinancialActionListResponse, FinancialStoreError> {
        let stored = self
            .0
            .list_actions(workspace_id)
            .await
            .map_err(financial_store_error)?;
        let mut actions = Vec::with_capacity(stored.len());
        for action in stored {
            actions.push(self.enrich_action(action).await?);
        }
        Ok(tl_core::FinancialActionListResponse { actions })
    }

    async fn persist_action_evaluation(
        &self,
        workspace_id: &str,
        evaluation: tl_core::FinancialActionEvaluation,
    ) -> Result<tl_core::FinancialActionEvaluation, FinancialStoreError> {
        self.0
            .persist_action_evaluation(workspace_id, evaluation)
            .await
            .map_err(financial_store_error)
    }

    async fn get_action_evaluation(
        &self,
        workspace_id: &str,
        action_id: &str,
    ) -> Result<tl_core::FinancialActionEvaluation, FinancialStoreError> {
        self.0
            .get_action_evaluation(workspace_id, action_id)
            .await
            .map_err(financial_store_error)
    }

    async fn issue_execution_grant(
        &self,
        workspace_id: &str,
        action_id: &str,
        action_hash: &str,
        binding: tl_core::FinancialExecutionBinding,
        expires_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<tl_core::FinancialExecutionGrant, FinancialStoreError> {
        self.0
            .issue_execution_grant(workspace_id, action_id, action_hash, binding, expires_at)
            .await
            .map_err(financial_store_error)
    }

    async fn get_execution_grant(
        &self,
        workspace_id: &str,
        action_id: &str,
    ) -> Result<tl_core::FinancialExecutionGrant, FinancialStoreError> {
        self.0
            .get_execution_grant(workspace_id, action_id)
            .await
            .map_err(financial_store_error)
    }

    async fn claim_execution_grant(
        &self,
        workspace_id: &str,
        action_id: &str,
        binding: tl_core::FinancialExecutionBinding,
        claim_id: &str,
        stale_before: chrono::DateTime<chrono::Utc>,
    ) -> Result<tl_core::FinancialExecutionGrant, FinancialStoreError> {
        self.0
            .claim_execution_grant(workspace_id, action_id, binding, claim_id, stale_before)
            .await
            .map_err(financial_store_error)
    }

    async fn finalize_execution(
        &self,
        workspace_id: &str,
        action_id: &str,
        grant_id: &str,
        finalization: FinancialExecutionFinalization,
    ) -> Result<
        (
            tl_core::FinancialActionRecord,
            tl_core::FinancialExecutionGrant,
            tl_core::FinancialReceipt,
        ),
        FinancialStoreError,
    > {
        let (action, grant, receipt) = self
            .0
            .finalize_execution(
                workspace_id,
                action_id,
                grant_id,
                tl_storage::FinalizeFinancialExecutionParams {
                    provider: finalization.provider,
                    provider_status: finalization.provider_status,
                    provider_reference: finalization.provider_reference,
                    provider_response: finalization.provider_response,
                    proof: finalization.proof,
                    commit_idempotency_key: finalization.commit_idempotency_key,
                    attestation_hash: finalization.attestation_hash,
                },
            )
            .await
            .map_err(financial_store_error)?;
        Ok((self.enrich_action(action).await?, grant, receipt.into()))
    }

    async fn fail_execution_grant(
        &self,
        workspace_id: &str,
        action_id: &str,
        grant_id: &str,
        reason: &str,
    ) -> Result<tl_core::FinancialActionRecord, FinancialStoreError> {
        let action = self
            .0
            .fail_execution_grant(workspace_id, action_id, grant_id, reason)
            .await
            .map_err(financial_store_error)?;
        self.enrich_action(action).await
    }

    async fn create_execution_connector(
        &self,
        workspace_id: &str,
        display_name: &str,
        encrypted_secret: &str,
        allowed_rails: Vec<tl_core::FinancialRail>,
        allowed_operations: Vec<String>,
    ) -> Result<StoredFinancialExecutionConnector, FinancialStoreError> {
        self.0
            .create_execution_connector(
                workspace_id,
                display_name,
                encrypted_secret,
                allowed_rails,
                allowed_operations,
            )
            .await
            .map(stored_execution_connector)
            .map_err(financial_store_error)
    }

    async fn list_execution_connectors(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<tl_core::FinancialExecutionConnector>, FinancialStoreError> {
        self.0
            .list_execution_connectors(workspace_id)
            .await
            .map_err(financial_store_error)
    }

    async fn get_execution_connector(
        &self,
        workspace_id: &str,
        connector_id: &str,
    ) -> Result<StoredFinancialExecutionConnector, FinancialStoreError> {
        self.0
            .get_execution_connector(workspace_id, connector_id)
            .await
            .map(stored_execution_connector)
            .map_err(financial_store_error)
    }

    async fn revoke_execution_connector(
        &self,
        workspace_id: &str,
        connector_id: &str,
    ) -> Result<tl_core::FinancialExecutionConnector, FinancialStoreError> {
        self.0
            .revoke_execution_connector(workspace_id, connector_id)
            .await
            .map_err(financial_store_error)
    }

    async fn create_observation_review(
        &self,
        workspace_id: &str,
        action_id: &str,
        outcome: tl_core::FinancialObservationReviewOutcome,
        note: Option<String>,
        reviewed_by: &str,
    ) -> Result<tl_core::FinancialObservationReview, FinancialStoreError> {
        self.0
            .create_observation_review(workspace_id, action_id, outcome, note, reviewed_by)
            .await
            .map_err(financial_store_error)
    }

    async fn list_observation_reviews(
        &self,
        workspace_id: &str,
        action_id: &str,
    ) -> Result<Vec<tl_core::FinancialObservationReview>, FinancialStoreError> {
        self.0
            .list_observation_reviews(workspace_id, action_id)
            .await
            .map_err(financial_store_error)
    }

    async fn observation_summary(
        &self,
        workspace_id: &str,
        environment_id: &str,
        start: chrono::DateTime<chrono::Utc>,
        end: chrono::DateTime<chrono::Utc>,
    ) -> Result<
        (
            Vec<tl_core::FinancialObservationCurrencySummary>,
            Vec<tl_core::FinancialObservationReasonSummary>,
        ),
        FinancialStoreError,
    > {
        self.0
            .observation_summary(workspace_id, environment_id, start, end)
            .await
            .map_err(financial_store_error)
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
        environment_id: Some(row.environment_id),
        runtime_mode: None,
        evaluation: None,
        execution_grant: None,
        action: row.action,
        evidence: row.evidence,
        created_at: row.created_at.to_rfc3339(),
        updated_at: row.updated_at.to_rfc3339(),
    }
}

fn stored_execution_connector(
    stored: tl_storage::StoredFinancialExecutionConnector,
) -> StoredFinancialExecutionConnector {
    StoredFinancialExecutionConnector {
        connector: stored.connector,
        encrypted_secret: stored.encrypted_secret,
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
