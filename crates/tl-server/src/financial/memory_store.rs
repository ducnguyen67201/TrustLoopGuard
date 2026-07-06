use std::collections::HashMap;

use async_trait::async_trait;
use tl_core::{
    ApprovalRequirement, CreateFinancialActionRequest, CreateFinancialMandateRequest,
    FinancialActionListResponse, FinancialActionRecord, FinancialActionStatus,
    FinancialApprovalRequest, FinancialApprovalRequestListResponse, FinancialApprovalRequestStatus,
    FinancialMandate, FinancialMandateListResponse, FinancialMandateStatus,
};
use tokio::sync::RwLock;

use super::{
    validation::{is_valid_transition, validate_create_action},
    FinancialStore, FinancialStoreError,
};

#[derive(Debug, Default)]
pub struct MemoryFinancialStore {
    actions: RwLock<HashMap<String, FinancialActionRecord>>,
    idempotency: RwLock<HashMap<String, String>>,
    approval_requests: RwLock<HashMap<String, FinancialApprovalRequest>>,
    mandates: RwLock<HashMap<String, FinancialMandate>>,
}

impl MemoryFinancialStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl FinancialStore for MemoryFinancialStore {
    async fn create_action(
        &self,
        workspace_id: &str,
        input: CreateFinancialActionRequest,
    ) -> Result<FinancialActionRecord, FinancialStoreError> {
        validate_create_action(&input)?;
        let idempotency_key = format!("{workspace_id}:{}", input.idempotency_key.trim());
        if let Some(action_id) = self.idempotency.read().await.get(&idempotency_key).cloned() {
            return self.get_action(workspace_id, &action_id).await;
        }

        let now = chrono::Utc::now().to_rfc3339();
        let id = input
            .action
            .id
            .clone()
            .unwrap_or_else(|| uuid::Uuid::now_v7().to_string());
        let record = FinancialActionRecord {
            id: id.clone(),
            workspace_id: workspace_id.to_string(),
            status: FinancialActionStatus::Proposed,
            action: tl_core::FinancialAction {
                id: Some(id.clone()),
                ..input.action
            },
            evidence: input.evidence,
            created_at: now.clone(),
            updated_at: now,
        };

        self.actions
            .write()
            .await
            .insert(key(workspace_id, &id), record.clone());
        self.idempotency.write().await.insert(idempotency_key, id);
        Ok(record)
    }

    async fn get_action(
        &self,
        workspace_id: &str,
        action_id: &str,
    ) -> Result<FinancialActionRecord, FinancialStoreError> {
        self.actions
            .read()
            .await
            .get(&key(workspace_id, action_id))
            .cloned()
            .ok_or(FinancialStoreError::NotFound)
    }

    async fn list_actions(
        &self,
        workspace_id: &str,
    ) -> Result<FinancialActionListResponse, FinancialStoreError> {
        let mut actions = self
            .actions
            .read()
            .await
            .values()
            .filter(|action| action.workspace_id == workspace_id)
            .cloned()
            .collect::<Vec<_>>();
        actions.sort_by(|a, b| {
            b.created_at
                .cmp(&a.created_at)
                .then_with(|| b.id.cmp(&a.id))
        });
        Ok(FinancialActionListResponse { actions })
    }

    async fn create_mandate(
        &self,
        workspace_id: &str,
        input: CreateFinancialMandateRequest,
    ) -> Result<FinancialMandate, FinancialStoreError> {
        let principal_id = clean_required("principal_id", &input.principal_id)?;
        let id = input
            .id
            .and_then(clean_optional)
            .unwrap_or_else(|| uuid::Uuid::now_v7().to_string());
        let version = input.version.unwrap_or(1);
        if version <= 0 {
            return Err(FinancialStoreError::Validation(
                "mandate version must be positive".into(),
            ));
        }
        let now = chrono::Utc::now().to_rfc3339();
        let mandate = FinancialMandate {
            id: id.clone(),
            workspace_id: workspace_id.to_string(),
            version,
            status: FinancialMandateStatus::Active,
            principal_id,
            scope: input.scope,
            metadata: input.metadata,
            starts_at: input.starts_at,
            expires_at: input.expires_at,
            created_at: now.clone(),
            updated_at: now,
        };
        self.mandates
            .write()
            .await
            .insert(mandate_key(workspace_id, &id, version), mandate.clone());
        Ok(mandate)
    }

    async fn list_mandates(
        &self,
        workspace_id: &str,
    ) -> Result<FinancialMandateListResponse, FinancialStoreError> {
        let mut mandates = self
            .mandates
            .read()
            .await
            .values()
            .filter(|mandate| mandate.workspace_id == workspace_id)
            .cloned()
            .collect::<Vec<_>>();
        mandates.sort_by(|a, b| {
            b.created_at
                .cmp(&a.created_at)
                .then_with(|| b.id.cmp(&a.id))
        });
        Ok(FinancialMandateListResponse { mandates })
    }

    async fn revoke_mandate(
        &self,
        workspace_id: &str,
        mandate_id: &str,
    ) -> Result<FinancialMandate, FinancialStoreError> {
        let mut mandates = self.mandates.write().await;
        let mut latest_key: Option<String> = None;
        let mut latest_version = i32::MIN;
        for (key, mandate) in mandates.iter() {
            if mandate.workspace_id == workspace_id
                && mandate.id == mandate_id
                && mandate.version > latest_version
            {
                latest_version = mandate.version;
                latest_key = Some(key.clone());
            }
        }
        let latest_key = latest_key.ok_or(FinancialStoreError::NotFound)?;
        for mandate in mandates.values_mut() {
            if mandate.workspace_id == workspace_id && mandate.id == mandate_id {
                mandate.status = FinancialMandateStatus::Revoked;
                mandate.updated_at = chrono::Utc::now().to_rfc3339();
            }
        }
        mandates
            .get(&latest_key)
            .cloned()
            .ok_or(FinancialStoreError::NotFound)
    }

    async fn create_approval_request(
        &self,
        workspace_id: &str,
        action_id: &str,
        approval: ApprovalRequirement,
    ) -> Result<FinancialApprovalRequest, FinancialStoreError> {
        self.get_action(workspace_id, action_id).await?;
        let now = chrono::Utc::now().to_rfc3339();
        let id = uuid::Uuid::now_v7().to_string();
        let request = FinancialApprovalRequest {
            id: id.clone(),
            workspace_id: workspace_id.to_string(),
            action_id: action_id.to_string(),
            status: FinancialApprovalRequestStatus::Pending,
            reason: approval.reason,
            approver_roles: approval.approver_roles,
            decided_by: None,
            decided_at: None,
            expires_at: approval.expires_at,
            metadata: serde_json::json!({}),
            created_at: now.clone(),
            updated_at: now,
        };
        self.approval_requests
            .write()
            .await
            .insert(key(workspace_id, &id), request.clone());
        Ok(request)
    }

    async fn list_approval_requests(
        &self,
        workspace_id: &str,
    ) -> Result<FinancialApprovalRequestListResponse, FinancialStoreError> {
        let mut approval_requests = self
            .approval_requests
            .read()
            .await
            .values()
            .filter(|request| request.workspace_id == workspace_id)
            .cloned()
            .collect::<Vec<_>>();
        approval_requests.sort_by(|a, b| {
            b.created_at
                .cmp(&a.created_at)
                .then_with(|| b.id.cmp(&a.id))
        });
        Ok(FinancialApprovalRequestListResponse { approval_requests })
    }

    async fn resolve_pending_approval_requests(
        &self,
        workspace_id: &str,
        action_id: &str,
        status: FinancialApprovalRequestStatus,
    ) -> Result<(), FinancialStoreError> {
        if !matches!(
            status,
            FinancialApprovalRequestStatus::Approved | FinancialApprovalRequestStatus::Denied
        ) {
            return Err(FinancialStoreError::Validation(
                "approval request resolution must be approved or denied".into(),
            ));
        }
        self.get_action(workspace_id, action_id).await?;
        let now = chrono::Utc::now().to_rfc3339();
        for request in self.approval_requests.write().await.values_mut() {
            if request.workspace_id == workspace_id
                && request.action_id == action_id
                && request.status == FinancialApprovalRequestStatus::Pending
            {
                request.status = status;
                request.decided_at = Some(now.clone());
                request.updated_at = now.clone();
            }
        }
        Ok(())
    }

    async fn transition_action(
        &self,
        workspace_id: &str,
        action_id: &str,
        status: FinancialActionStatus,
        _event_type: &str,
    ) -> Result<FinancialActionRecord, FinancialStoreError> {
        let mut actions = self.actions.write().await;
        let record = actions
            .get_mut(&key(workspace_id, action_id))
            .ok_or(FinancialStoreError::NotFound)?;
        if !is_valid_transition(record.status, status) {
            return Err(FinancialStoreError::Conflict);
        }
        record.status = status;
        record.updated_at = chrono::Utc::now().to_rfc3339();
        Ok(record.clone())
    }
}

fn key(workspace_id: &str, action_id: &str) -> String {
    format!("{workspace_id}:{action_id}")
}

fn mandate_key(workspace_id: &str, mandate_id: &str, version: i32) -> String {
    format!("{workspace_id}:{mandate_id}:{version}")
}

fn clean_required(name: &str, value: &str) -> Result<String, FinancialStoreError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(FinancialStoreError::Validation(format!(
            "{name} must not be empty"
        )));
    }
    Ok(trimmed.to_string())
}

fn clean_optional(value: String) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}
