use std::sync::Arc;

use async_trait::async_trait;
use tl_core::{
    CreateNotificationRuleRequest, NotificationDeliverySummary, NotificationRule,
    UpdateNotificationRuleRequest,
};

use crate::notifications::{
    ClaimedNotification, EnqueueNotification, NotificationStore, NotificationStoreError,
};

pub struct PostgresNotificationAdapter(Arc<tl_storage::NotificationRepo>);

impl PostgresNotificationAdapter {
    pub fn new(repo: Arc<tl_storage::NotificationRepo>) -> Self {
        Self(repo)
    }
}

#[async_trait]
impl NotificationStore for PostgresNotificationAdapter {
    async fn create_rule(
        &self,
        workspace_id: &str,
        environment_id: &str,
        agent_id: Option<String>,
        input: CreateNotificationRuleRequest,
    ) -> Result<NotificationRule, NotificationStoreError> {
        self.0
            .create_rule(
                workspace_id,
                environment_id,
                agent_id,
                input.email,
                input.event_kinds,
                input.enabled,
            )
            .await
            .map_err(map_error)
    }
    async fn list_rules(
        &self,
        workspace_id: &str,
        environment_id: &str,
    ) -> Result<Vec<NotificationRule>, NotificationStoreError> {
        self.0
            .list_rules(workspace_id, environment_id)
            .await
            .map_err(map_error)
    }
    async fn update_rule(
        &self,
        workspace_id: &str,
        rule_id: &str,
        input: UpdateNotificationRuleRequest,
    ) -> Result<NotificationRule, NotificationStoreError> {
        self.0
            .update_rule(
                workspace_id,
                rule_id,
                input.email,
                input.event_kinds,
                input.enabled,
            )
            .await
            .map_err(map_error)
    }
    async fn delete_rule(
        &self,
        workspace_id: &str,
        rule_id: &str,
    ) -> Result<(), NotificationStoreError> {
        self.0
            .delete_rule(workspace_id, rule_id)
            .await
            .map_err(map_error)
    }
    async fn enqueue(&self, input: EnqueueNotification) -> Result<usize, NotificationStoreError> {
        self.0
            .enqueue_matching(
                &input.workspace_id,
                &input.environment_id,
                input.agent_id.as_deref(),
                input.rule_id.as_deref(),
                input.event_kind,
                &input.subject_id,
                &input.subject_version,
                input.run_id.as_deref(),
                input.payload,
            )
            .await
            .map_err(map_error)
    }
    async fn list_deliveries(
        &self,
        workspace_id: &str,
        environment_id: &str,
        limit: usize,
    ) -> Result<Vec<NotificationDeliverySummary>, NotificationStoreError> {
        self.0
            .list_deliveries(workspace_id, environment_id, limit as i64)
            .await
            .map_err(map_error)
    }
    async fn claim(
        &self,
        worker_id: &str,
        lease_seconds: i64,
    ) -> Result<Option<ClaimedNotification>, NotificationStoreError> {
        self.0
            .claim_delivery(worker_id, lease_seconds)
            .await
            .map(|value| {
                value.map(|claimed| ClaimedNotification {
                    workspace_id: claimed.workspace_id,
                    delivery: claimed.delivery,
                    email: claimed.email,
                    payload: claimed.payload,
                    run_id: claimed.run_id,
                })
            })
            .map_err(map_error)
    }
    async fn mark_sent(
        &self,
        workspace_id: &str,
        delivery_id: &str,
    ) -> Result<(), NotificationStoreError> {
        self.0
            .mark_sent(workspace_id, delivery_id)
            .await
            .map_err(map_error)
    }
    async fn retry_or_fail(
        &self,
        workspace_id: &str,
        delivery_id: &str,
        max_attempts: i32,
        error_code: &str,
        error_message: &str,
    ) -> Result<(), NotificationStoreError> {
        self.0
            .retry_or_fail(
                workspace_id,
                delivery_id,
                max_attempts,
                error_code,
                error_message,
            )
            .await
            .map_err(map_error)
    }
}

fn map_error(error: tl_storage::StorageError) -> NotificationStoreError {
    match error {
        tl_storage::StorageError::NotFound => NotificationStoreError::NotFound,
        tl_storage::StorageError::Conflict => NotificationStoreError::Conflict,
        tl_storage::StorageError::Internal(message) => NotificationStoreError::Internal(message),
    }
}
