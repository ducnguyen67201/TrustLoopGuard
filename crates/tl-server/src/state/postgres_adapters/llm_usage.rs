use std::sync::Arc;

use async_trait::async_trait;
use tl_core::{LlmUsageBucket, LlmUsageBucketsResponse, LlmUsageEvent, LlmUsageListResponse};

use crate::llm_usage::{
    LlmUsageFilter, LlmUsageGroupBy, LlmUsageStore, LlmUsageStoreError, RecordLlmUsageEvent,
};

pub struct PostgresLlmUsageAdapter(pub Arc<tl_storage::LlmUsageRepo>);

impl PostgresLlmUsageAdapter {
    pub fn new(repo: Arc<tl_storage::LlmUsageRepo>) -> Arc<Self> {
        Arc::new(Self(repo))
    }
}

#[async_trait]
impl LlmUsageStore for PostgresLlmUsageAdapter {
    async fn insert_event(
        &self,
        workspace_id: &str,
        event: RecordLlmUsageEvent,
    ) -> Result<(), LlmUsageStoreError> {
        self.0
            .insert_event(
                workspace_id,
                tl_storage::NewLlmUsageEventParams {
                    principal_id: event.principal_id,
                    api_key_id: event.api_key_id,
                    model: event.model,
                    prompt_tokens: event.prompt_tokens,
                    completion_tokens: event.completion_tokens,
                    cost_minor: event.cost_minor,
                    currency: event.currency,
                    request_id: event.request_id,
                    metadata: event.metadata,
                },
            )
            .await
            .map_err(llm_usage_store_error)
    }

    async fn net_llm_spend_minor(
        &self,
        workspace_id: &str,
        principal_id: &str,
        currency: &str,
        start: chrono::DateTime<chrono::Utc>,
        end: chrono::DateTime<chrono::Utc>,
    ) -> Result<i64, LlmUsageStoreError> {
        self.0
            .net_llm_spend_minor(workspace_id, principal_id, currency, start, end)
            .await
            .map_err(llm_usage_store_error)
    }

    async fn list_events(
        &self,
        workspace_id: &str,
        filter: &LlmUsageFilter,
    ) -> Result<LlmUsageListResponse, LlmUsageStoreError> {
        let events = self
            .0
            .list_events(workspace_id, &storage_filter(filter))
            .await
            .map_err(llm_usage_store_error)?
            .into_iter()
            .map(usage_event)
            .collect();
        Ok(LlmUsageListResponse { events })
    }

    async fn grouped_usage(
        &self,
        workspace_id: &str,
        group_by: LlmUsageGroupBy,
        filter: &LlmUsageFilter,
    ) -> Result<LlmUsageBucketsResponse, LlmUsageStoreError> {
        let group_by = match group_by {
            LlmUsageGroupBy::Day => tl_storage::LlmUsageGroupBy::Day,
            LlmUsageGroupBy::Principal => tl_storage::LlmUsageGroupBy::Principal,
            LlmUsageGroupBy::Model => tl_storage::LlmUsageGroupBy::Model,
        };
        let buckets = self
            .0
            .grouped_usage(workspace_id, group_by, &storage_filter(filter))
            .await
            .map_err(llm_usage_store_error)?
            .into_iter()
            .map(|row| LlmUsageBucket {
                key: row.key,
                prompt_tokens: row.prompt_tokens,
                completion_tokens: row.completion_tokens,
                cost_minor: row.cost_minor,
                calls: row.calls,
            })
            .collect();
        Ok(LlmUsageBucketsResponse { buckets })
    }
}

fn storage_filter(filter: &LlmUsageFilter) -> tl_storage::LlmUsageEventFilter {
    tl_storage::LlmUsageEventFilter {
        principal_id: filter.principal_id.clone(),
        model: filter.model.clone(),
        start: filter.start,
        end: filter.end,
    }
}

fn usage_event(row: tl_storage::StoredLlmUsageEvent) -> LlmUsageEvent {
    LlmUsageEvent {
        id: row.id,
        workspace_id: row.workspace_id,
        principal_id: row.principal_id,
        api_key_id: row.api_key_id,
        model: row.model,
        prompt_tokens: row.prompt_tokens,
        completion_tokens: row.completion_tokens,
        cost_minor: row.cost_minor,
        currency: row.currency,
        request_id: row.request_id,
        metadata: row.metadata,
        effective_at: row.effective_at.to_rfc3339(),
    }
}

fn llm_usage_store_error(error: tl_storage::StorageError) -> LlmUsageStoreError {
    match error {
        tl_storage::StorageError::Internal(message) if message.contains("must") => {
            LlmUsageStoreError::Validation(message)
        }
        other => LlmUsageStoreError::Internal(other.to_string()),
    }
}
