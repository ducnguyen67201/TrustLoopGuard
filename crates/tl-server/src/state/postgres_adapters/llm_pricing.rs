use std::sync::Arc;

use async_trait::async_trait;

use crate::llm_pricing::{LlmPricingStore, LlmPricingStoreError, ModelPrice, WorkspaceModelPrice};

pub struct PostgresLlmPricingAdapter(pub Arc<tl_storage::LlmPricingRepo>);

impl PostgresLlmPricingAdapter {
    pub fn new(repo: Arc<tl_storage::LlmPricingRepo>) -> Arc<Self> {
        Arc::new(Self(repo))
    }
}

#[async_trait]
impl LlmPricingStore for PostgresLlmPricingAdapter {
    async fn upsert_price(
        &self,
        workspace_id: &str,
        model: &str,
        input_per_million_minor: i64,
        output_per_million_minor: i64,
        input_per_million_nanos: i64,
        output_per_million_nanos: i64,
    ) -> Result<(), LlmPricingStoreError> {
        self.0
            .upsert_price(
                workspace_id,
                model,
                input_per_million_minor,
                output_per_million_minor,
                input_per_million_nanos,
                output_per_million_nanos,
            )
            .await
            .map_err(store_error)
    }

    async fn delete_price(
        &self,
        workspace_id: &str,
        model: &str,
    ) -> Result<bool, LlmPricingStoreError> {
        self.0
            .delete_price(workspace_id, model)
            .await
            .map_err(store_error)
    }

    async fn list_prices(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<WorkspaceModelPrice>, LlmPricingStoreError> {
        Ok(self
            .0
            .list_prices(workspace_id)
            .await
            .map_err(store_error)?
            .into_iter()
            .map(|row| WorkspaceModelPrice {
                model: row.model,
                price: ModelPrice {
                    input_per_million_minor: row.input_per_million_minor,
                    output_per_million_minor: row.output_per_million_minor,
                    input_per_million_nanos: row.input_per_million_nanos,
                    output_per_million_nanos: row.output_per_million_nanos,
                },
            })
            .collect())
    }

    async fn get_price(
        &self,
        workspace_id: &str,
        model: &str,
    ) -> Result<Option<ModelPrice>, LlmPricingStoreError> {
        Ok(self
            .0
            .get_price(workspace_id, model)
            .await
            .map_err(store_error)?
            .map(|row| ModelPrice {
                input_per_million_minor: row.input_per_million_minor,
                output_per_million_minor: row.output_per_million_minor,
                input_per_million_nanos: row.input_per_million_nanos,
                output_per_million_nanos: row.output_per_million_nanos,
            }))
    }
}

fn store_error(error: tl_storage::StorageError) -> LlmPricingStoreError {
    LlmPricingStoreError::Internal(error.to_string())
}
