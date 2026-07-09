//! Financial authorization endpoints.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use tl_core::{
    AgenticPaymentReservation, ApprovalRequirement, CreateFinancialActionRequest,
    CreateFinancialMandateRequest, FinancialActionListResponse, FinancialActionOutcome,
    FinancialActionRecord, FinancialActionStatus, FinancialApprovalRequest,
    FinancialApprovalRequestListResponse, FinancialApprovalRequestStatus, FinancialMandate,
    FinancialMandateListResponse, FinancialOutcomeListResponse, FinancialReceipt, MoneyAmount,
};

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
    __path_create_action, __path_create_mandate, __path_create_policy, __path_deny_action,
    __path_execute_action, __path_get_action, __path_get_agentic_payment,
    __path_get_agentic_payment_receipt, __path_get_decision_receipt, __path_get_receipt,
    __path_list_action_outcomes, __path_list_actions, __path_list_approval_requests,
    __path_list_mandates, __path_list_policies, __path_record_action_outcome,
    __path_revoke_mandate, __path_rollback_agentic_payment, approve_action,
    authorize_agentic_payment, commit_agentic_payment, create_action, create_mandate,
    create_policy, deny_action, execute_action, get_action, get_agentic_payment,
    get_agentic_payment_receipt, get_decision_receipt, get_receipt, list_action_outcomes,
    list_actions, list_approval_requests, list_mandates, list_policies, record_action_outcome,
    revoke_mandate, rollback_agentic_payment,
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

#[async_trait]
pub trait FinancialStore: Send + Sync {
    async fn create_action(
        &self,
        workspace_id: &str,
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
}
