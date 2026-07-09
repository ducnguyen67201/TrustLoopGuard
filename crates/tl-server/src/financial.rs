//! Financial authorization endpoints.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use tl_core::{
    AgenticPaymentReservation, ApprovalRequirement, CreateFinancialActionRequest,
    CreateFinancialMandateRequest, FinancialActionEvaluation, FinancialActionListResponse,
    FinancialActionOutcome, FinancialActionRecord, FinancialActionStatus, FinancialApprovalRequest,
    FinancialApprovalRequestListResponse, FinancialApprovalRequestStatus,
    FinancialExecutionBinding, FinancialExecutionConnector, FinancialExecutionGrant,
    FinancialMandate, FinancialMandateListResponse, FinancialObservationCurrencySummary,
    FinancialObservationReasonSummary, FinancialObservationReview,
    FinancialObservationReviewOutcome, FinancialOutcomeListResponse, FinancialRail,
    FinancialReceipt, MoneyAmount,
};

mod attestation;
mod canonical;
mod executor;
mod handlers;
mod memory_store;
mod response;
mod service;
mod validation;
mod x402;

pub use executor::{
    FinancialExecutionError, FinancialExecutionResult, FinancialExecutor,
    PaymentHttpFinancialExecutor,
};
pub use handlers::{
    __path_approve_action, __path_authorize_agentic_payment, __path_commit_agentic_payment,
    __path_commit_external_action, __path_create_action, __path_create_execution_connector,
    __path_create_mandate, __path_create_observation_review, __path_create_policy,
    __path_deny_action, __path_execute_action, __path_financial_observation_summary,
    __path_get_action, __path_get_agentic_payment, __path_get_agentic_payment_receipt,
    __path_get_decision_receipt, __path_get_receipt, __path_list_action_outcomes,
    __path_list_actions, __path_list_approval_requests, __path_list_execution_connectors,
    __path_list_mandates, __path_list_observation_reviews, __path_list_policies,
    __path_record_action_outcome, __path_revoke_execution_connector, __path_revoke_mandate,
    __path_rollback_agentic_payment, approve_action, authorize_agentic_payment,
    commit_agentic_payment, commit_external_action, create_action, create_execution_connector,
    create_mandate, create_observation_review, create_policy, deny_action, execute_action,
    financial_observation_summary, get_action, get_agentic_payment, get_agentic_payment_receipt,
    get_decision_receipt, get_receipt, list_action_outcomes, list_actions, list_approval_requests,
    list_execution_connectors, list_mandates, list_observation_reviews, list_policies,
    record_action_outcome, revoke_execution_connector, revoke_mandate, rollback_agentic_payment,
};
pub use memory_store::MemoryFinancialStore;
pub use service::{FinancialActionExecutionAttempt, FinancialAuthorizationService};

#[derive(Debug, thiserror::Error)]
pub enum FinancialStoreError {
    #[error("not found")]
    NotFound,
    #[error("conflict")]
    Conflict,
    #[error("validation: {0}")]
    Validation(String),
    #[error("internal: {0}")]
    Internal(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FinancialLedgerEntryKind {
    Reserved,
    Released,
    Executed,
    Reversed,
}

#[derive(Debug, Clone)]
pub struct AgenticPaymentBudgetReservationRequest {
    pub workspace_id: String,
    pub session_id: String,
    pub principal_id: String,
    pub action_id: String,
    pub payment_requirement_hash: String,
    pub amount: MoneyAmount,
    pub session_limit_minor: i64,
    pub expires_at: DateTime<Utc>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct StoredFinancialExecutionConnector {
    pub connector: FinancialExecutionConnector,
    pub encrypted_secret: String,
}

#[derive(Debug, Clone)]
pub struct FinancialExecutionFinalization {
    pub provider: String,
    pub provider_status: String,
    pub provider_reference: Option<String>,
    pub provider_response: serde_json::Value,
    pub proof: serde_json::Value,
    pub commit_idempotency_key: Option<String>,
    pub attestation_hash: Option<String>,
}

#[async_trait]
pub trait FinancialStore: Send + Sync {
    async fn create_action(
        &self,
        workspace_id: &str,
        environment_id: &str,
        input: CreateFinancialActionRequest,
    ) -> Result<FinancialActionRecord, FinancialStoreError>;

    async fn get_action(
        &self,
        workspace_id: &str,
        action_id: &str,
    ) -> Result<FinancialActionRecord, FinancialStoreError>;

    async fn list_actions(
        &self,
        workspace_id: &str,
    ) -> Result<FinancialActionListResponse, FinancialStoreError>;

    async fn persist_action_evaluation(
        &self,
        workspace_id: &str,
        evaluation: FinancialActionEvaluation,
    ) -> Result<FinancialActionEvaluation, FinancialStoreError> {
        let _ = (workspace_id, evaluation);
        Err(unsupported_store_operation("persist action evaluation"))
    }

    async fn get_action_evaluation(
        &self,
        workspace_id: &str,
        action_id: &str,
    ) -> Result<FinancialActionEvaluation, FinancialStoreError> {
        let _ = (workspace_id, action_id);
        Err(unsupported_store_operation("get action evaluation"))
    }

    async fn issue_execution_grant(
        &self,
        workspace_id: &str,
        action_id: &str,
        action_hash: &str,
        binding: FinancialExecutionBinding,
        expires_at: DateTime<Utc>,
    ) -> Result<FinancialExecutionGrant, FinancialStoreError> {
        let _ = (workspace_id, action_id, action_hash, binding, expires_at);
        Err(unsupported_store_operation("issue execution grant"))
    }

    async fn get_execution_grant(
        &self,
        workspace_id: &str,
        action_id: &str,
    ) -> Result<FinancialExecutionGrant, FinancialStoreError> {
        let _ = (workspace_id, action_id);
        Err(unsupported_store_operation("get execution grant"))
    }

    async fn claim_execution_grant(
        &self,
        workspace_id: &str,
        action_id: &str,
        binding: FinancialExecutionBinding,
        claim_id: &str,
        stale_before: DateTime<Utc>,
    ) -> Result<FinancialExecutionGrant, FinancialStoreError> {
        let _ = (workspace_id, action_id, binding, claim_id, stale_before);
        Err(unsupported_store_operation("claim execution grant"))
    }

    async fn finalize_execution(
        &self,
        workspace_id: &str,
        action_id: &str,
        grant_id: &str,
        finalization: FinancialExecutionFinalization,
    ) -> Result<
        (
            FinancialActionRecord,
            FinancialExecutionGrant,
            FinancialReceipt,
        ),
        FinancialStoreError,
    > {
        let _ = (workspace_id, action_id, grant_id, finalization);
        Err(unsupported_store_operation("finalize execution"))
    }

    async fn fail_execution_grant(
        &self,
        workspace_id: &str,
        action_id: &str,
        grant_id: &str,
        reason: &str,
    ) -> Result<FinancialActionRecord, FinancialStoreError> {
        let _ = (workspace_id, action_id, grant_id, reason);
        Err(unsupported_store_operation("fail execution grant"))
    }

    async fn create_execution_connector(
        &self,
        workspace_id: &str,
        display_name: &str,
        encrypted_secret: &str,
        allowed_rails: Vec<FinancialRail>,
        allowed_operations: Vec<String>,
    ) -> Result<StoredFinancialExecutionConnector, FinancialStoreError> {
        let _ = (
            workspace_id,
            display_name,
            encrypted_secret,
            allowed_rails,
            allowed_operations,
        );
        Err(unsupported_store_operation("create execution connector"))
    }

    async fn list_execution_connectors(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<FinancialExecutionConnector>, FinancialStoreError> {
        let _ = workspace_id;
        Err(unsupported_store_operation("list execution connectors"))
    }

    async fn get_execution_connector(
        &self,
        workspace_id: &str,
        connector_id: &str,
    ) -> Result<StoredFinancialExecutionConnector, FinancialStoreError> {
        let _ = (workspace_id, connector_id);
        Err(unsupported_store_operation("get execution connector"))
    }

    async fn revoke_execution_connector(
        &self,
        workspace_id: &str,
        connector_id: &str,
    ) -> Result<FinancialExecutionConnector, FinancialStoreError> {
        let _ = (workspace_id, connector_id);
        Err(unsupported_store_operation("revoke execution connector"))
    }

    async fn create_observation_review(
        &self,
        workspace_id: &str,
        action_id: &str,
        outcome: FinancialObservationReviewOutcome,
        note: Option<String>,
        reviewed_by: &str,
    ) -> Result<FinancialObservationReview, FinancialStoreError> {
        let _ = (workspace_id, action_id, outcome, note, reviewed_by);
        Err(unsupported_store_operation("create observation review"))
    }

    async fn list_observation_reviews(
        &self,
        workspace_id: &str,
        action_id: &str,
    ) -> Result<Vec<FinancialObservationReview>, FinancialStoreError> {
        let _ = (workspace_id, action_id);
        Err(unsupported_store_operation("list observation reviews"))
    }

    async fn observation_summary(
        &self,
        workspace_id: &str,
        environment_id: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<
        (
            Vec<FinancialObservationCurrencySummary>,
            Vec<FinancialObservationReasonSummary>,
        ),
        FinancialStoreError,
    > {
        let _ = (workspace_id, environment_id, start, end);
        Err(unsupported_store_operation("observation summary"))
    }

    async fn create_mandate(
        &self,
        workspace_id: &str,
        input: CreateFinancialMandateRequest,
    ) -> Result<FinancialMandate, FinancialStoreError>;

    async fn list_mandates(
        &self,
        workspace_id: &str,
    ) -> Result<FinancialMandateListResponse, FinancialStoreError>;

    async fn get_mandate(
        &self,
        workspace_id: &str,
        mandate_id: &str,
        version: Option<i32>,
    ) -> Result<FinancialMandate, FinancialStoreError>;

    async fn revoke_mandate(
        &self,
        workspace_id: &str,
        mandate_id: &str,
    ) -> Result<FinancialMandate, FinancialStoreError>;

    async fn create_receipt(
        &self,
        workspace_id: &str,
        action_id: &str,
        trace_id: Option<&str>,
        ledger_event_ids: Vec<String>,
        proof: serde_json::Value,
    ) -> Result<FinancialReceipt, FinancialStoreError>;

    async fn get_receipt(
        &self,
        workspace_id: &str,
        receipt_id: &str,
    ) -> Result<FinancialReceipt, FinancialStoreError>;

    async fn record_action_outcome(
        &self,
        workspace_id: &str,
        action_id: &str,
        outcome: FinancialActionOutcome,
    ) -> Result<FinancialActionOutcome, FinancialStoreError>;

    async fn list_action_outcomes(
        &self,
        workspace_id: &str,
        action_id: &str,
    ) -> Result<FinancialOutcomeListResponse, FinancialStoreError>;

    async fn create_approval_request(
        &self,
        workspace_id: &str,
        action_id: &str,
        approval: ApprovalRequirement,
    ) -> Result<FinancialApprovalRequest, FinancialStoreError>;

    async fn list_approval_requests(
        &self,
        workspace_id: &str,
    ) -> Result<FinancialApprovalRequestListResponse, FinancialStoreError>;

    async fn resolve_pending_approval_requests(
        &self,
        workspace_id: &str,
        action_id: &str,
        status: FinancialApprovalRequestStatus,
        decided_by: Option<&str>,
    ) -> Result<(), FinancialStoreError>;

    async fn transition_action(
        &self,
        workspace_id: &str,
        action_id: &str,
        status: FinancialActionStatus,
        event_type: &str,
    ) -> Result<FinancialActionRecord, FinancialStoreError>;

    async fn transition_action_with_reason(
        &self,
        workspace_id: &str,
        action_id: &str,
        status: FinancialActionStatus,
        event_type: &str,
        reason: &str,
    ) -> Result<FinancialActionRecord, FinancialStoreError> {
        let _ = reason;
        self.transition_action(workspace_id, action_id, status, event_type)
            .await
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
    ) -> Result<String, FinancialStoreError>;

    async fn ledger_entry_exists(
        &self,
        workspace_id: &str,
        idempotency_key: &str,
    ) -> Result<bool, FinancialStoreError>;

    async fn net_spend_minor(
        &self,
        _workspace_id: &str,
        _principal_id: &str,
        _currency: &str,
        _start: DateTime<Utc>,
        _end: DateTime<Utc>,
    ) -> Result<i64, FinancialStoreError> {
        Ok(0)
    }

    async fn try_reserve_agentic_payment_budget(
        &self,
        request: AgenticPaymentBudgetReservationRequest,
    ) -> Result<AgenticPaymentReservation, FinancialStoreError>;

    async fn get_agentic_payment_reservation(
        &self,
        workspace_id: &str,
        action_id: &str,
    ) -> Result<AgenticPaymentReservation, FinancialStoreError>;

    async fn commit_agentic_payment_reservation(
        &self,
        workspace_id: &str,
        action_id: &str,
        proof: serde_json::Value,
    ) -> Result<AgenticPaymentReservation, FinancialStoreError>;

    async fn release_agentic_payment_reservation(
        &self,
        workspace_id: &str,
        action_id: &str,
        reason: &str,
        metadata: serde_json::Value,
    ) -> Result<AgenticPaymentReservation, FinancialStoreError>;
}

#[derive(Clone)]
pub struct FinancialState {
    pub service: FinancialAuthorizationService,
    pub settings_store: Arc<dyn crate::SettingsStore>,
    pub team_store: Arc<dyn crate::team::TeamStore>,
}

fn unsupported_store_operation(operation: &str) -> FinancialStoreError {
    FinancialStoreError::Internal(format!("financial store does not support {operation}"))
}
