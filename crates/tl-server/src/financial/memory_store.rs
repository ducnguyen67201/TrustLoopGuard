use std::collections::HashMap;

use async_trait::async_trait;
use tl_core::{
    CreateFinancialActionRequest, FinancialActionListResponse, FinancialActionRecord,
    FinancialActionStatus,
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
