use std::sync::Arc;

use async_trait::async_trait;
use tl_core::{
    BudgetAlertConfig, BudgetAlertFiring, CreateBudgetAlertConfigRequest,
    UpdateBudgetAlertConfigRequest,
};

use crate::budget_alerts::{
    threshold_type_from_str, window_from_str, BudgetAlertStore, BudgetAlertStoreError,
    RecordBudgetAlertFiring,
};

pub struct PostgresBudgetAlertAdapter(pub Arc<tl_storage::BudgetAlertRepo>);

impl PostgresBudgetAlertAdapter {
    pub fn new(repo: Arc<tl_storage::BudgetAlertRepo>) -> Arc<Self> {
        Arc::new(Self(repo))
    }
}

#[async_trait]
impl BudgetAlertStore for PostgresBudgetAlertAdapter {
    async fn create_config(
        &self,
        workspace_id: &str,
        input: CreateBudgetAlertConfigRequest,
    ) -> Result<BudgetAlertConfig, BudgetAlertStoreError> {
        let name = input.name.clone();
        self.0
            .create_config(
                workspace_id,
                tl_storage::NewBudgetAlertConfigParams {
                    name: input.name,
                    meter: crate::budget_alerts::meter_label(input.meter).to_string(),
                    window: crate::budget_alerts::window_label(input.window).to_string(),
                    principal_id: input.principal_id,
                    threshold_type: crate::budget_alerts::threshold_type_label(
                        input.threshold_type,
                    )
                    .to_string(),
                    threshold_value: input.threshold_value,
                    webhook_url: input.webhook_url,
                    enabled: input.enabled.unwrap_or(true),
                },
            )
            .await
            .map_err(|error| conflict_aware_error(error, &name))
            .and_then(config_from_stored)
    }

    async fn get_config(
        &self,
        workspace_id: &str,
        config_id: &str,
    ) -> Result<BudgetAlertConfig, BudgetAlertStoreError> {
        self.0
            .get_config(workspace_id, config_id)
            .await
            .map_err(budget_alert_store_error)
            .and_then(config_from_stored)
    }

    async fn list_configs(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<BudgetAlertConfig>, BudgetAlertStoreError> {
        self.0
            .list_configs(workspace_id)
            .await
            .map_err(budget_alert_store_error)?
            .into_iter()
            .map(config_from_stored)
            .collect()
    }

    async fn list_enabled_configs(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<BudgetAlertConfig>, BudgetAlertStoreError> {
        self.0
            .list_enabled_configs(workspace_id)
            .await
            .map_err(budget_alert_store_error)?
            .into_iter()
            .map(config_from_stored)
            .collect()
    }

    async fn update_config(
        &self,
        workspace_id: &str,
        config_id: &str,
        update: UpdateBudgetAlertConfigRequest,
    ) -> Result<BudgetAlertConfig, BudgetAlertStoreError> {
        let name = update.name.clone().unwrap_or_default();
        self.0
            .update_config(
                workspace_id,
                config_id,
                tl_storage::UpdateBudgetAlertConfigParams {
                    name: update.name,
                    meter: update
                        .meter
                        .map(|meter| crate::budget_alerts::meter_label(meter).to_string()),
                    window: update
                        .window
                        .map(|window| crate::budget_alerts::window_label(window).to_string()),
                    principal_id: update.principal_id,
                    threshold_type: update.threshold_type.map(|threshold_type| {
                        crate::budget_alerts::threshold_type_label(threshold_type).to_string()
                    }),
                    threshold_value: update.threshold_value,
                    webhook_url: update.webhook_url,
                    enabled: update.enabled,
                },
            )
            .await
            .map_err(|error| conflict_aware_error(error, &name))
            .and_then(config_from_stored)
    }

    async fn delete_config(
        &self,
        workspace_id: &str,
        config_id: &str,
    ) -> Result<(), BudgetAlertStoreError> {
        self.0
            .delete_config(workspace_id, config_id)
            .await
            .map_err(budget_alert_store_error)
    }

    async fn try_record_firing(
        &self,
        workspace_id: &str,
        firing: RecordBudgetAlertFiring,
    ) -> Result<bool, BudgetAlertStoreError> {
        self.0
            .try_record_firing(
                workspace_id,
                tl_storage::NewBudgetAlertFiringParams {
                    config_id: firing.config_id,
                    meter: crate::budget_alerts::meter_label(firing.meter).to_string(),
                    principal_id: firing.principal_id,
                    window_start: firing.window_start,
                    cap_minor: firing.cap_minor,
                    spent_minor: firing.spent_minor,
                    currency: firing.currency,
                    payload: firing.payload,
                },
            )
            .await
            .map_err(budget_alert_store_error)
    }

    async fn list_firings(
        &self,
        workspace_id: &str,
        config_id: &str,
    ) -> Result<Vec<BudgetAlertFiring>, BudgetAlertStoreError> {
        Ok(self
            .0
            .list_firings(workspace_id, config_id)
            .await
            .map_err(budget_alert_store_error)?
            .into_iter()
            .map(firing_from_stored)
            .collect())
    }
}

fn config_from_stored(
    stored: tl_storage::StoredBudgetAlertConfig,
) -> Result<BudgetAlertConfig, BudgetAlertStoreError> {
    Ok(BudgetAlertConfig {
        meter: crate::budget_alerts::meter_from_str(&stored.meter).ok_or_else(|| {
            BudgetAlertStoreError::Internal(format!("unknown spend meter `{}`", stored.meter))
        })?,
        window: window_from_str(&stored.window).ok_or_else(|| {
            BudgetAlertStoreError::Internal(format!("unknown alert window `{}`", stored.window))
        })?,
        threshold_type: threshold_type_from_str(&stored.threshold_type).ok_or_else(|| {
            BudgetAlertStoreError::Internal(format!(
                "unknown threshold type `{}`",
                stored.threshold_type
            ))
        })?,
        id: stored.id,
        workspace_id: stored.workspace_id,
        name: stored.name,
        principal_id: stored.principal_id,
        threshold_value: stored.threshold_value,
        webhook_url: stored.webhook_url,
        enabled: stored.enabled,
        created_at: stored.created_at.to_rfc3339(),
        updated_at: stored.updated_at.to_rfc3339(),
    })
}

fn firing_from_stored(stored: tl_storage::StoredBudgetAlertFiring) -> BudgetAlertFiring {
    BudgetAlertFiring {
        id: stored.id,
        workspace_id: stored.workspace_id,
        config_id: stored.config_id,
        meter: crate::budget_alerts::meter_from_str(&stored.meter)
            .unwrap_or(tl_core::SpendMeter::Actions),
        principal_id: stored.principal_id,
        window_start: stored.window_start.to_rfc3339(),
        cap_minor: stored.cap_minor,
        spent_minor: stored.spent_minor,
        currency: stored.currency,
        payload: stored.payload,
        fired_at: stored.fired_at.to_rfc3339(),
    }
}

/// The `(workspace_id, meter, name)` UNIQUE key is the only conflict source
/// on config writes, so map it to a named error.
fn conflict_aware_error(error: tl_storage::StorageError, name: &str) -> BudgetAlertStoreError {
    match error {
        tl_storage::StorageError::Conflict => {
            BudgetAlertStoreError::Conflict(format!("a budget alert named `{name}` already exists"))
        }
        other => budget_alert_store_error(other),
    }
}

fn budget_alert_store_error(error: tl_storage::StorageError) -> BudgetAlertStoreError {
    match error {
        tl_storage::StorageError::NotFound => BudgetAlertStoreError::NotFound,
        tl_storage::StorageError::Conflict => {
            BudgetAlertStoreError::Conflict("budget alert conflict".into())
        }
        other => BudgetAlertStoreError::Internal(other.to_string()),
    }
}
