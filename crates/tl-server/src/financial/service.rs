use std::sync::Arc;

use tl_core::{
    CreateFinancialActionRequest, FinancialActionListResponse, FinancialActionRecord,
    FinancialActionStatus,
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

    pub async fn approve_action(
        &self,
        workspace_id: &str,
        action_id: &str,
    ) -> Result<FinancialActionRecord, FinancialStoreError> {
        self.transition_action(
            workspace_id,
            action_id,
            FinancialActionStatus::Authorized,
            "approved",
        )
        .await
    }

    pub async fn deny_action(
        &self,
        workspace_id: &str,
        action_id: &str,
    ) -> Result<FinancialActionRecord, FinancialStoreError> {
        self.transition_action(
            workspace_id,
            action_id,
            FinancialActionStatus::Denied,
            "denied",
        )
        .await
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
