use std::collections::HashSet;

use async_trait::async_trait;
use chrono::Utc;
use tl_core::{
    BudgetAlertConfig, BudgetAlertFiring, CreateBudgetAlertConfigRequest,
    UpdateBudgetAlertConfigRequest,
};
use tokio::sync::RwLock;
use uuid::Uuid;

use super::{BudgetAlertStore, BudgetAlertStoreError, RecordBudgetAlertFiring};

#[derive(Debug, Default)]
pub struct MemoryBudgetAlertStore {
    configs: RwLock<Vec<BudgetAlertConfig>>,
    firings: RwLock<Vec<BudgetAlertFiring>>,
    /// Dedup keys, mirroring the postgres UNIQUE
    /// `(config_id, principal_id, window_start)`.
    firing_keys: RwLock<HashSet<String>>,
}

impl MemoryBudgetAlertStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl BudgetAlertStore for MemoryBudgetAlertStore {
    async fn create_config(
        &self,
        workspace_id: &str,
        input: CreateBudgetAlertConfigRequest,
    ) -> Result<BudgetAlertConfig, BudgetAlertStoreError> {
        let mut configs = self.configs.write().await;
        if configs
            .iter()
            .any(|config| config.workspace_id == workspace_id && config.name == input.name)
        {
            return Err(BudgetAlertStoreError::Conflict(format!(
                "a budget alert named `{}` already exists",
                input.name
            )));
        }
        let now = Utc::now().to_rfc3339();
        let config = BudgetAlertConfig {
            id: Uuid::now_v7().to_string(),
            workspace_id: workspace_id.to_string(),
            name: input.name,
            window: input.window,
            principal_id: input.principal_id,
            threshold_type: input.threshold_type,
            threshold_value: input.threshold_value,
            webhook_url: input.webhook_url,
            enabled: input.enabled.unwrap_or(true),
            created_at: now.clone(),
            updated_at: now,
        };
        configs.push(config.clone());
        Ok(config)
    }

    async fn get_config(
        &self,
        workspace_id: &str,
        config_id: &str,
    ) -> Result<BudgetAlertConfig, BudgetAlertStoreError> {
        self.configs
            .read()
            .await
            .iter()
            .find(|config| config.workspace_id == workspace_id && config.id == config_id)
            .cloned()
            .ok_or(BudgetAlertStoreError::NotFound)
    }

    async fn list_configs(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<BudgetAlertConfig>, BudgetAlertStoreError> {
        Ok(self
            .configs
            .read()
            .await
            .iter()
            .filter(|config| config.workspace_id == workspace_id)
            .cloned()
            .collect())
    }

    async fn list_enabled_configs(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<BudgetAlertConfig>, BudgetAlertStoreError> {
        Ok(self
            .configs
            .read()
            .await
            .iter()
            .filter(|config| config.workspace_id == workspace_id && config.enabled)
            .cloned()
            .collect())
    }

    async fn update_config(
        &self,
        workspace_id: &str,
        config_id: &str,
        update: UpdateBudgetAlertConfigRequest,
    ) -> Result<BudgetAlertConfig, BudgetAlertStoreError> {
        let mut configs = self.configs.write().await;
        if let Some(name) = &update.name {
            if configs.iter().any(|config| {
                config.workspace_id == workspace_id
                    && config.id != config_id
                    && &config.name == name
            }) {
                return Err(BudgetAlertStoreError::Conflict(format!(
                    "a budget alert named `{name}` already exists"
                )));
            }
        }
        let config = configs
            .iter_mut()
            .find(|config| config.workspace_id == workspace_id && config.id == config_id)
            .ok_or(BudgetAlertStoreError::NotFound)?;
        if let Some(name) = update.name {
            config.name = name;
        }
        if let Some(window) = update.window {
            config.window = window;
        }
        if let Some(principal_id) = update.principal_id {
            config.principal_id = Some(principal_id);
        }
        if let Some(threshold_type) = update.threshold_type {
            config.threshold_type = threshold_type;
        }
        if let Some(threshold_value) = update.threshold_value {
            config.threshold_value = threshold_value;
        }
        if let Some(webhook_url) = update.webhook_url {
            config.webhook_url = Some(webhook_url);
        }
        if let Some(enabled) = update.enabled {
            config.enabled = enabled;
        }
        config.updated_at = Utc::now().to_rfc3339();
        Ok(config.clone())
    }

    async fn delete_config(
        &self,
        workspace_id: &str,
        config_id: &str,
    ) -> Result<(), BudgetAlertStoreError> {
        let mut configs = self.configs.write().await;
        let before = configs.len();
        configs.retain(|config| !(config.workspace_id == workspace_id && config.id == config_id));
        if configs.len() == before {
            return Err(BudgetAlertStoreError::NotFound);
        }
        // Postgres cascades firings on config delete; mirror that.
        self.firings
            .write()
            .await
            .retain(|firing| firing.config_id != config_id);
        Ok(())
    }

    async fn try_record_firing(
        &self,
        workspace_id: &str,
        firing: RecordBudgetAlertFiring,
    ) -> Result<bool, BudgetAlertStoreError> {
        let key = format!(
            "{}:{}:{}",
            firing.config_id,
            firing.principal_id,
            firing.window_start.to_rfc3339()
        );
        let mut keys = self.firing_keys.write().await;
        if keys.contains(&key) {
            // Another spend in this window won the race — mirrors the
            // postgres ON CONFLICT DO NOTHING.
            return Ok(false);
        }
        self.firings.write().await.push(BudgetAlertFiring {
            id: Uuid::now_v7().to_string(),
            workspace_id: workspace_id.to_string(),
            config_id: firing.config_id,
            principal_id: firing.principal_id,
            window_start: firing.window_start.to_rfc3339(),
            cap_minor: firing.cap_minor,
            spent_minor: firing.spent_minor,
            currency: firing.currency,
            payload: firing.payload,
            fired_at: Utc::now().to_rfc3339(),
        });
        keys.insert(key);
        Ok(true)
    }

    async fn list_firings(
        &self,
        workspace_id: &str,
        config_id: &str,
    ) -> Result<Vec<BudgetAlertFiring>, BudgetAlertStoreError> {
        let mut firings = self
            .firings
            .read()
            .await
            .iter()
            .filter(|firing| firing.workspace_id == workspace_id && firing.config_id == config_id)
            .cloned()
            .collect::<Vec<_>>();
        // RFC 3339 strings sort chronologically.
        firings.sort_by(|a, b| b.fired_at.cmp(&a.fired_at).then_with(|| b.id.cmp(&a.id)));
        Ok(firings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tl_core::{BudgetAlertThresholdType, BudgetAlertWindow};

    fn config(name: &str) -> CreateBudgetAlertConfigRequest {
        CreateBudgetAlertConfigRequest {
            name: name.into(),
            window: BudgetAlertWindow::Week,
            principal_id: None,
            threshold_type: BudgetAlertThresholdType::Percent,
            threshold_value: 80,
            webhook_url: None,
            enabled: Some(true),
        }
    }

    fn firing(config_id: &str, principal: &str) -> RecordBudgetAlertFiring {
        RecordBudgetAlertFiring {
            config_id: config_id.into(),
            principal_id: principal.into(),
            window_start: Utc::now(),
            cap_minor: 5000,
            spent_minor: 4000,
            currency: "USD".into(),
            payload: serde_json::json!({}),
        }
    }

    #[tokio::test]
    async fn config_round_trip_and_name_conflict() {
        let store = MemoryBudgetAlertStore::new();
        let created = store
            .create_config("ws", config("weekly-80"))
            .await
            .unwrap();
        assert!(store
            .create_config("ws", config("weekly-80"))
            .await
            .is_err());
        // Same name in another workspace is fine.
        store
            .create_config("ws_other", config("weekly-80"))
            .await
            .unwrap();

        let listed = store.list_configs("ws").await.unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, created.id);

        let updated = store
            .update_config(
                "ws",
                &created.id,
                UpdateBudgetAlertConfigRequest {
                    enabled: Some(false),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        assert!(!updated.enabled);
        assert!(store.list_enabled_configs("ws").await.unwrap().is_empty());

        store.delete_config("ws", &created.id).await.unwrap();
        assert!(store.list_configs("ws").await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn firing_dedup_is_per_config_principal_window() {
        let store = MemoryBudgetAlertStore::new();
        let created = store
            .create_config("ws", config("weekly-80"))
            .await
            .unwrap();
        let first = firing(&created.id, "user:a");

        assert!(store.try_record_firing("ws", first.clone()).await.unwrap());
        // Same key: deduped.
        assert!(!store.try_record_firing("ws", first.clone()).await.unwrap());
        // Different principal, same window: fires.
        assert!(store
            .try_record_firing(
                "ws",
                RecordBudgetAlertFiring {
                    principal_id: "user:b".into(),
                    ..first.clone()
                }
            )
            .await
            .unwrap());
        // New window: fires again.
        assert!(store
            .try_record_firing(
                "ws",
                RecordBudgetAlertFiring {
                    window_start: first.window_start + chrono::Duration::days(7),
                    ..first
                }
            )
            .await
            .unwrap());

        assert_eq!(
            store.list_firings("ws", &created.id).await.unwrap().len(),
            3
        );
    }
}
