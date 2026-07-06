use std::sync::Arc;

use async_trait::async_trait;

use crate::financial::{FinancialStore, FinancialStoreError};

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
    ) -> Result<(), FinancialStoreError> {
        self.0
            .resolve_pending_approval_requests(workspace_id, action_id, status)
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
