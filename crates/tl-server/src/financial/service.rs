use std::sync::Arc;

use tl_core::{
    ApprovalRequirement, CreateFinancialActionRequest, CreateFinancialMandateRequest,
    FinancialActionListResponse, FinancialActionRecord, FinancialActionStatus,
    FinancialApprovalRequestListResponse, FinancialApprovalRequestStatus, FinancialMandate,
    FinancialMandateListResponse,
};

use super::{validation::validate_create_action, FinancialStore, FinancialStoreError};

#[derive(Clone)]
pub struct FinancialAuthorizationService {
    store: Arc<dyn FinancialStore>,
}

impl FinancialAuthorizationService {
    pub fn new(store: Arc<dyn FinancialStore>) -> Self {
        Self { store }
    }

    pub async fn create_action(
        &self,
        workspace_id: &str,
        input: CreateFinancialActionRequest,
    ) -> Result<FinancialActionRecord, FinancialStoreError> {
        validate_create_action(&input)?;
        self.store.create_action(workspace_id, input).await
    }

    pub async fn get_action(
        &self,
        workspace_id: &str,
        action_id: &str,
    ) -> Result<FinancialActionRecord, FinancialStoreError> {
        self.store.get_action(workspace_id, action_id).await
    }

    pub async fn list_actions(
        &self,
        workspace_id: &str,
    ) -> Result<FinancialActionListResponse, FinancialStoreError> {
        self.store.list_actions(workspace_id).await
    }

    pub async fn create_mandate(
        &self,
        workspace_id: &str,
        input: CreateFinancialMandateRequest,
    ) -> Result<FinancialMandate, FinancialStoreError> {
        self.store.create_mandate(workspace_id, input).await
    }

    pub async fn list_mandates(
        &self,
        workspace_id: &str,
    ) -> Result<FinancialMandateListResponse, FinancialStoreError> {
        self.store.list_mandates(workspace_id).await
    }

    pub async fn revoke_mandate(
        &self,
        workspace_id: &str,
        mandate_id: &str,
    ) -> Result<FinancialMandate, FinancialStoreError> {
        self.store.revoke_mandate(workspace_id, mandate_id).await
    }

    pub async fn list_approval_requests(
        &self,
        workspace_id: &str,
    ) -> Result<FinancialApprovalRequestListResponse, FinancialStoreError> {
        self.store.list_approval_requests(workspace_id).await
    }

    pub async fn hold_action(
        &self,
        workspace_id: &str,
        action_id: &str,
        approval: ApprovalRequirement,
    ) -> Result<FinancialActionRecord, FinancialStoreError> {
        let held = self
            .transition_action(
                workspace_id,
                action_id,
                FinancialActionStatus::Held,
                "approval_required",
            )
            .await?;
        self.store
            .create_approval_request(workspace_id, action_id, approval)
            .await?;
        Ok(held)
    }

    pub async fn approve_action(
        &self,
        workspace_id: &str,
        action_id: &str,
    ) -> Result<FinancialActionRecord, FinancialStoreError> {
        let approved = self
            .transition_action(
                workspace_id,
                action_id,
                FinancialActionStatus::Authorized,
                "approved",
            )
            .await?;
        self.store
            .resolve_pending_approval_requests(
                workspace_id,
                action_id,
                FinancialApprovalRequestStatus::Approved,
            )
            .await?;
        Ok(approved)
    }

    pub async fn deny_action(
        &self,
        workspace_id: &str,
        action_id: &str,
    ) -> Result<FinancialActionRecord, FinancialStoreError> {
        let denied = self
            .transition_action(
                workspace_id,
                action_id,
                FinancialActionStatus::Denied,
                "denied",
            )
            .await?;
        self.store
            .resolve_pending_approval_requests(
                workspace_id,
                action_id,
                FinancialApprovalRequestStatus::Denied,
            )
            .await?;
        Ok(denied)
    }

    pub async fn execute_action(
        &self,
        workspace_id: &str,
        action_id: &str,
    ) -> Result<FinancialActionRecord, FinancialStoreError> {
        self.transition_action(
            workspace_id,
            action_id,
            FinancialActionStatus::Executed,
            "executed",
        )
        .await
    }

    async fn transition_action(
        &self,
        workspace_id: &str,
        action_id: &str,
        status: FinancialActionStatus,
        event_type: &str,
    ) -> Result<FinancialActionRecord, FinancialStoreError> {
        self.store
            .transition_action(workspace_id, action_id, status, event_type)
            .await
    }
}
