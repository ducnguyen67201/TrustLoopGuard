//! Workspace-editable LLM model price repository.
//!
//! One `llm_model_prices` row per `(workspace, model)`, upserted from
//! `PUT /v1/llm-pricing/{model}`. Gateway metering reads `get_price`
//! (one indexed PK lookup per metered call) and falls back to the
//! built-in default table in `tl-server` when no row matches.

use diesel::prelude::*;
use diesel_async::RunQueryDsl;

use crate::models::{LlmModelPriceRecord, NewLlmModelPrice};
use crate::postgres::{DbConnection, DbPool};
use crate::schema::llm_model_prices;
use crate::StorageError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredLlmModelPrice {
    pub model: String,
    pub input_per_million_minor: i64,
    pub output_per_million_minor: i64,
    pub input_per_million_nanos: i64,
    pub output_per_million_nanos: i64,
    pub currency: String,
}

pub struct LlmPricingRepo {
    pool: DbPool,
}

impl LlmPricingRepo {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Insert or update one workspace model price.
    pub async fn upsert_price(
        &self,
        workspace_id: &str,
        model: &str,
        input_per_million_minor: i64,
        output_per_million_minor: i64,
        input_per_million_nanos: i64,
        output_per_million_nanos: i64,
    ) -> Result<(), StorageError> {
        let row = NewLlmModelPrice {
            workspace_id: workspace_id.to_string(),
            model: model.to_string(),
            input_per_million_minor,
            output_per_million_minor,
            input_per_million_nanos,
            output_per_million_nanos,
        };
        let mut conn = self.connection().await?;
        diesel::insert_into(llm_model_prices::table)
            .values(&row)
            .on_conflict((llm_model_prices::workspace_id, llm_model_prices::model))
            .do_update()
            .set((
                llm_model_prices::input_per_million_minor.eq(input_per_million_minor),
                llm_model_prices::output_per_million_minor.eq(output_per_million_minor),
                llm_model_prices::input_per_million_nanos.eq(input_per_million_nanos),
                llm_model_prices::output_per_million_nanos.eq(output_per_million_nanos),
                llm_model_prices::updated_at.eq(diesel::dsl::now),
            ))
            .execute(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("llm price upsert: {e}")))?;
        Ok(())
    }

    /// Delete one workspace model price. Returns whether a row existed.
    pub async fn delete_price(
        &self,
        workspace_id: &str,
        model: &str,
    ) -> Result<bool, StorageError> {
        let mut conn = self.connection().await?;
        let deleted = diesel::delete(
            llm_model_prices::table
                .filter(llm_model_prices::workspace_id.eq(workspace_id))
                .filter(llm_model_prices::model.eq(model)),
        )
        .execute(&mut conn)
        .await
        .map_err(|e| StorageError::Internal(format!("llm price delete: {e}")))?;
        Ok(deleted > 0)
    }

    /// All workspace price rows, model ascending.
    pub async fn list_prices(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<StoredLlmModelPrice>, StorageError> {
        let mut conn = self.connection().await?;
        let rows = llm_model_prices::table
            .filter(llm_model_prices::workspace_id.eq(workspace_id))
            .order(llm_model_prices::model.asc())
            .select(LlmModelPriceRecord::as_select())
            .load::<LlmModelPriceRecord>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("llm price list: {e}")))?;
        Ok(rows.into_iter().map(stored_price).collect())
    }

    /// Exact-match lookup — one indexed PK read.
    pub async fn get_price(
        &self,
        workspace_id: &str,
        model: &str,
    ) -> Result<Option<StoredLlmModelPrice>, StorageError> {
        let mut conn = self.connection().await?;
        let row = llm_model_prices::table
            .filter(llm_model_prices::workspace_id.eq(workspace_id))
            .filter(llm_model_prices::model.eq(model))
            .select(LlmModelPriceRecord::as_select())
            .first::<LlmModelPriceRecord>(&mut conn)
            .await
            .optional()
            .map_err(|e| StorageError::Internal(format!("llm price get: {e}")))?;
        Ok(row.map(stored_price))
    }

    async fn connection(&self) -> Result<DbConnection<'_>, StorageError> {
        self.pool
            .get()
            .await
            .map_err(|e| StorageError::Internal(format!("db pool: {e}")))
    }
}

impl std::fmt::Debug for LlmPricingRepo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LlmPricingRepo").finish_non_exhaustive()
    }
}

fn stored_price(row: LlmModelPriceRecord) -> StoredLlmModelPrice {
    StoredLlmModelPrice {
        model: row.model,
        input_per_million_minor: row.input_per_million_minor,
        output_per_million_minor: row.output_per_million_minor,
        input_per_million_nanos: row.input_per_million_nanos,
        output_per_million_nanos: row.output_per_million_nanos,
        currency: row.currency,
    }
}
