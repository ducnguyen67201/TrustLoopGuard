//! Financial authorization endpoints.

use async_trait::async_trait;
use tl_core::{
    ApprovalRequirement, CreateFinancialActionRequest, CreateFinancialMandateRequest,
    FinancialActionListResponse, FinancialActionRecord, FinancialActionStatus,
    FinancialApprovalRequest, FinancialApprovalRequestListResponse, FinancialApprovalRequestStatus,
    FinancialMandate, FinancialMandateListResponse,
};

mod handlers;
mod memory_store;
mod response;
mod service;
mod validation;

pub use handlers::{
    __path_approve_action, __path_create_action, __path_create_mandate, __path_deny_action,
    __path_execute_action, __path_get_action, __path_list_actions, __path_list_approval_requests,
    __path_list_mandates, __path_revoke_mandate, approve_action, create_action, create_mandate,
    deny_action, execute_action, get_action, list_actions, list_approval_requests, list_mandates,
    revoke_mandate,
};
pub use memory_store::MemoryFinancialStore;
pub use service::FinancialAuthorizationService;

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

    async fn revoke_mandate(
        &self,
        workspace_id: &str,
        mandate_id: &str,
    ) -> Result<FinancialMandate, FinancialStoreError>;

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
    ) -> Result<(), FinancialStoreError>;

    async fn transition_action(
        &self,
        workspace_id: &str,
        action_id: &str,
        status: FinancialActionStatus,
        event_type: &str,
    ) -> Result<FinancialActionRecord, FinancialStoreError>;
}

#[derive(Clone)]
pub struct FinancialState {
    pub service: FinancialAuthorizationService,
}
