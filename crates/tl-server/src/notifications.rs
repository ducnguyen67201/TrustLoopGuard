mod handlers;
mod worker;

use std::collections::HashMap;

use async_trait::async_trait;
use serde_json::Value;
use tl_core::{
    CreateNotificationRuleRequest, NotificationDeliveryStatus, NotificationDeliverySummary,
    NotificationEventKind, NotificationRule, UpdateNotificationRuleRequest,
};
use tokio::sync::RwLock;

pub use handlers::{
    __path_create_notification_rule, __path_delete_notification_rule,
    __path_list_notification_deliveries, __path_list_notification_rules,
    __path_notification_readiness, __path_patch_notification_rule, __path_test_notification,
    create_notification_rule, delete_notification_rule, list_notification_deliveries,
    list_notification_rules, notification_readiness, patch_notification_rule, test_notification,
    NotificationState,
};
pub use worker::{spawn_notification_worker, NotificationWorkerConfig};

pub(crate) fn valid_email(value: &str) -> bool {
    let value = value.trim();
    value.len() <= 320
        && value
            .split_once('@')
            .is_some_and(|(local, domain)| !local.is_empty() && domain.contains('.'))
}

#[derive(Debug, thiserror::Error)]
pub enum NotificationStoreError {
    #[error("not found")]
    NotFound,
    #[error("conflict")]
    Conflict,
    #[error("validation: {0}")]
    Validation(String),
    #[error("internal: {0}")]
    Internal(String),
}

#[derive(Debug, Clone)]
pub struct EnqueueNotification {
    pub workspace_id: String,
    pub environment_id: String,
    pub agent_id: Option<String>,
    pub rule_id: Option<String>,
    pub event_kind: NotificationEventKind,
    pub subject_id: String,
    pub subject_version: String,
    pub run_id: Option<String>,
    pub payload: Value,
}

#[derive(Debug, Clone)]
pub struct ClaimedNotification {
    pub workspace_id: String,
    pub delivery: NotificationDeliverySummary,
    pub email: String,
    pub payload: Value,
    pub run_id: Option<String>,
}

#[async_trait]
pub trait NotificationStore: Send + Sync {
    async fn create_rule(
        &self,
        workspace_id: &str,
        environment_id: &str,
        agent_id: Option<String>,
        input: CreateNotificationRuleRequest,
    ) -> Result<NotificationRule, NotificationStoreError>;
    async fn list_rules(
        &self,
        workspace_id: &str,
        environment_id: &str,
    ) -> Result<Vec<NotificationRule>, NotificationStoreError>;
    async fn update_rule(
        &self,
        workspace_id: &str,
        rule_id: &str,
        input: UpdateNotificationRuleRequest,
    ) -> Result<NotificationRule, NotificationStoreError>;
    async fn delete_rule(
        &self,
        workspace_id: &str,
        rule_id: &str,
    ) -> Result<(), NotificationStoreError>;
    async fn enqueue(&self, input: EnqueueNotification) -> Result<usize, NotificationStoreError>;
    async fn list_deliveries(
        &self,
        workspace_id: &str,
        environment_id: &str,
        limit: usize,
    ) -> Result<Vec<NotificationDeliverySummary>, NotificationStoreError>;
    async fn claim(
        &self,
        worker_id: &str,
        lease_seconds: i64,
    ) -> Result<Option<ClaimedNotification>, NotificationStoreError>;
    async fn mark_sent(
        &self,
        workspace_id: &str,
        delivery_id: &str,
    ) -> Result<(), NotificationStoreError>;
    async fn retry_or_fail(
        &self,
        workspace_id: &str,
        delivery_id: &str,
        max_attempts: i32,
        error_code: &str,
        error_message: &str,
    ) -> Result<(), NotificationStoreError>;
}

#[derive(Debug, Default)]
pub struct MemoryNotificationStore {
    rules: RwLock<HashMap<(String, String), Vec<NotificationRule>>>,
    deliveries: RwLock<Vec<MemoryDelivery>>,
}

#[derive(Debug, Clone)]
struct MemoryDelivery {
    workspace_id: String,
    environment_id: String,
    summary: NotificationDeliverySummary,
    email: String,
    payload: Value,
    run_id: Option<String>,
}

impl MemoryNotificationStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl NotificationStore for MemoryNotificationStore {
    async fn create_rule(
        &self,
        workspace_id: &str,
        environment_id: &str,
        agent_id: Option<String>,
        input: CreateNotificationRuleRequest,
    ) -> Result<NotificationRule, NotificationStoreError> {
        let now = chrono::Utc::now().to_rfc3339();
        let rule = NotificationRule {
            id: uuid::Uuid::now_v7().to_string(),
            workspace_id: workspace_id.to_string(),
            environment_id: environment_id.to_string(),
            agent_id,
            email: input.email,
            event_kinds: input.event_kinds,
            enabled: input.enabled,
            created_at: now.clone(),
            updated_at: now,
        };
        self.rules
            .write()
            .await
            .entry((workspace_id.to_string(), environment_id.to_string()))
            .or_default()
            .push(rule.clone());
        Ok(rule)
    }

    async fn list_rules(
        &self,
        workspace_id: &str,
        environment_id: &str,
    ) -> Result<Vec<NotificationRule>, NotificationStoreError> {
        Ok(self
            .rules
            .read()
            .await
            .get(&(workspace_id.to_string(), environment_id.to_string()))
            .cloned()
            .unwrap_or_default())
    }

    async fn update_rule(
        &self,
        workspace_id: &str,
        rule_id: &str,
        input: UpdateNotificationRuleRequest,
    ) -> Result<NotificationRule, NotificationStoreError> {
        let mut rules = self.rules.write().await;
        let rule = rules
            .iter_mut()
            .filter(|((workspace, _), _)| workspace == workspace_id)
            .find_map(|(_, rules)| rules.iter_mut().find(|rule| rule.id == rule_id))
            .ok_or(NotificationStoreError::NotFound)?;
        if let Some(email) = input.email {
            rule.email = email;
        }
        if let Some(event_kinds) = input.event_kinds {
            rule.event_kinds = event_kinds;
        }
        if let Some(enabled) = input.enabled {
            rule.enabled = enabled;
        }
        rule.updated_at = chrono::Utc::now().to_rfc3339();
        Ok(rule.clone())
    }

    async fn delete_rule(
        &self,
        workspace_id: &str,
        rule_id: &str,
    ) -> Result<(), NotificationStoreError> {
        let mut rules = self.rules.write().await;
        let rows = rules
            .iter_mut()
            .filter(|((workspace, _), _)| workspace == workspace_id)
            .find_map(|(_, rows)| rows.iter().any(|rule| rule.id == rule_id).then_some(rows))
            .ok_or(NotificationStoreError::NotFound)?;
        let before = rows.len();
        rows.retain(|rule| rule.id != rule_id);
        if rows.len() == before {
            return Err(NotificationStoreError::NotFound);
        }
        Ok(())
    }

    async fn enqueue(&self, input: EnqueueNotification) -> Result<usize, NotificationStoreError> {
        let rules = self
            .rules
            .read()
            .await
            .get(&(input.workspace_id.clone(), input.environment_id.clone()))
            .cloned()
            .unwrap_or_default();
        let mut deliveries = self.deliveries.write().await;
        let mut inserted = 0;
        for rule in rules.into_iter().filter(|rule| {
            rule.enabled
                && input.rule_id.as_deref().map_or_else(
                    || rule.event_kinds.contains(&input.event_kind),
                    |rule_id| rule.id == rule_id,
                )
        }) {
            if deliveries.iter().any(|delivery| {
                delivery.summary.rule_id == rule.id
                    && delivery.summary.event_kind == input.event_kind
                    && delivery.summary.subject_id == input.subject_id
                    && delivery.summary.subject_version == input.subject_version
            }) {
                continue;
            }
            let now = chrono::Utc::now().to_rfc3339();
            deliveries.push(MemoryDelivery {
                workspace_id: input.workspace_id.clone(),
                environment_id: input.environment_id.clone(),
                email: rule.email,
                payload: input.payload.clone(),
                run_id: input.run_id.clone(),
                summary: NotificationDeliverySummary {
                    id: uuid::Uuid::now_v7().to_string(),
                    rule_id: rule.id,
                    event_kind: input.event_kind,
                    subject_id: input.subject_id.clone(),
                    subject_version: input.subject_version.clone(),
                    status: NotificationDeliveryStatus::Pending,
                    attempt_count: 0,
                    last_error_code: None,
                    sent_at: None,
                    created_at: now.clone(),
                    updated_at: now,
                },
            });
            inserted += 1;
        }
        Ok(inserted)
    }

    async fn list_deliveries(
        &self,
        workspace_id: &str,
        environment_id: &str,
        limit: usize,
    ) -> Result<Vec<NotificationDeliverySummary>, NotificationStoreError> {
        Ok(self
            .deliveries
            .read()
            .await
            .iter()
            .filter(|delivery| {
                delivery.workspace_id == workspace_id && delivery.environment_id == environment_id
            })
            .rev()
            .take(limit)
            .map(|delivery| delivery.summary.clone())
            .collect())
    }

    async fn claim(
        &self,
        _worker_id: &str,
        _lease_seconds: i64,
    ) -> Result<Option<ClaimedNotification>, NotificationStoreError> {
        let mut deliveries = self.deliveries.write().await;
        let Some(delivery) = deliveries
            .iter_mut()
            .find(|delivery| delivery.summary.status == NotificationDeliveryStatus::Pending)
        else {
            return Ok(None);
        };
        delivery.summary.status = NotificationDeliveryStatus::Sending;
        delivery.summary.attempt_count += 1;
        Ok(Some(ClaimedNotification {
            workspace_id: delivery.workspace_id.clone(),
            delivery: delivery.summary.clone(),
            email: delivery.email.clone(),
            payload: delivery.payload.clone(),
            run_id: delivery.run_id.clone(),
        }))
    }

    async fn mark_sent(
        &self,
        workspace_id: &str,
        delivery_id: &str,
    ) -> Result<(), NotificationStoreError> {
        let mut deliveries = self.deliveries.write().await;
        let delivery = deliveries
            .iter_mut()
            .find(|delivery| {
                delivery.workspace_id == workspace_id && delivery.summary.id == delivery_id
            })
            .ok_or(NotificationStoreError::NotFound)?;
        delivery.summary.status = NotificationDeliveryStatus::Sent;
        delivery.summary.sent_at = Some(chrono::Utc::now().to_rfc3339());
        Ok(())
    }

    async fn retry_or_fail(
        &self,
        workspace_id: &str,
        delivery_id: &str,
        max_attempts: i32,
        error_code: &str,
        _error_message: &str,
    ) -> Result<(), NotificationStoreError> {
        let mut deliveries = self.deliveries.write().await;
        let delivery = deliveries
            .iter_mut()
            .find(|delivery| {
                delivery.workspace_id == workspace_id && delivery.summary.id == delivery_id
            })
            .ok_or(NotificationStoreError::NotFound)?;
        delivery.summary.status = if delivery.summary.attempt_count >= max_attempts {
            NotificationDeliveryStatus::Failed
        } else {
            NotificationDeliveryStatus::Pending
        };
        delivery.summary.last_error_code = Some(error_code.to_string());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use tl_core::{CreateNotificationRuleRequest, NotificationEventKind};

    use super::{EnqueueNotification, MemoryNotificationStore, NotificationStore};

    #[tokio::test]
    async fn memory_outbox_is_environment_scoped_and_deduplicated() {
        let store = MemoryNotificationStore::new();
        store
            .create_rule(
                "workspace-1",
                "production",
                None,
                CreateNotificationRuleRequest {
                    email: "ops@example.com".into(),
                    event_kinds: vec![NotificationEventKind::EvaluationFailed],
                    enabled: true,
                },
            )
            .await
            .unwrap();

        let input = EnqueueNotification {
            workspace_id: "workspace-1".into(),
            environment_id: "production".into(),
            agent_id: None,
            rule_id: None,
            event_kind: NotificationEventKind::EvaluationFailed,
            subject_id: "run-1".into(),
            subject_version: "v1".into(),
            run_id: Some("run-1".into()),
            payload: json!({"title": "failed"}),
        };
        assert_eq!(store.enqueue(input.clone()).await.unwrap(), 1);
        assert_eq!(store.enqueue(input).await.unwrap(), 0);
        assert_eq!(
            store
                .list_deliveries("workspace-1", "production", 10)
                .await
                .unwrap()
                .len(),
            1
        );
        assert!(store
            .list_deliveries("workspace-1", "staging", 10)
            .await
            .unwrap()
            .is_empty());
    }
}
