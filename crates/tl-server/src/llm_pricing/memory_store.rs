use std::collections::BTreeMap;

use async_trait::async_trait;
use tokio::sync::RwLock;

use super::{LlmPricingStore, LlmPricingStoreError, ModelPrice, WorkspaceModelPrice};

/// In-memory `LlmPricingStore`. Keys are `(workspace_id, model)` in a
/// `BTreeMap` so listings come out model-ascending, matching the
/// postgres repo's `ORDER BY model`.
#[derive(Debug, Default)]
pub struct MemoryLlmPricingStore {
    prices: RwLock<BTreeMap<(String, String), ModelPrice>>,
}

impl MemoryLlmPricingStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl LlmPricingStore for MemoryLlmPricingStore {
    async fn upsert_price(
        &self,
        workspace_id: &str,
        model: &str,
        input_per_million_minor: i64,
        output_per_million_minor: i64,
    ) -> Result<(), LlmPricingStoreError> {
        self.prices.write().await.insert(
            (workspace_id.to_string(), model.to_string()),
            ModelPrice {
                input_per_million_minor,
                output_per_million_minor,
            },
        );
        Ok(())
    }

    async fn delete_price(
        &self,
        workspace_id: &str,
        model: &str,
    ) -> Result<bool, LlmPricingStoreError> {
        Ok(self
            .prices
            .write()
            .await
            .remove(&(workspace_id.to_string(), model.to_string()))
            .is_some())
    }

    async fn list_prices(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<WorkspaceModelPrice>, LlmPricingStoreError> {
        Ok(self
            .prices
            .read()
            .await
            .iter()
            .filter(|((workspace, _), _)| workspace == workspace_id)
            .map(|((_, model), price)| WorkspaceModelPrice {
                model: model.clone(),
                price: *price,
            })
            .collect())
    }

    async fn get_price(
        &self,
        workspace_id: &str,
        model: &str,
    ) -> Result<Option<ModelPrice>, LlmPricingStoreError> {
        Ok(self
            .prices
            .read()
            .await
            .get(&(workspace_id.to_string(), model.to_string()))
            .copied())
    }
}
