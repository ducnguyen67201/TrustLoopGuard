use std::sync::Arc;

use async_trait::async_trait;
use tl_core::{LlmUsageBucket, LlmUsageBucketsResponse, LlmUsageEvent, LlmUsageListResponse};

use crate::llm_usage::{
    LlmBudgetWindow, LlmUsageFilter, LlmUsageGroupBy, LlmUsageStore, LlmUsageStoreError,
    RecordLlmUsageEvent, ReserveLlmBudget, ReserveLlmBudgetOutcome,
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
                    cost_nanos: event.cost_nanos,
                    currency: event.currency,
                    request_id: event.request_id,
                    metadata: event.metadata,
                },
            )
            .await
            .map_err(llm_usage_store_error)
    }

    async fn reserve_budget(
        &self,
        workspace_id: &str,
        reservation: ReserveLlmBudget,
    ) -> Result<ReserveLlmBudgetOutcome, LlmUsageStoreError> {
        let outcome = self
            .0
            .reserve_budget(
                workspace_id,
                tl_storage::NewLlmBudgetReservationParams {
                    request_id: reservation.request_id,
                    principal_id: reservation.principal_id,
                    api_key_id: reservation.api_key_id,
                    currency: reservation.currency,
                    reserved_nanos: reservation.reserved_nanos,
                    caps: tl_storage::LlmBudgetCapsNanos {
                        daily: reservation.caps.daily,
                        weekly: reservation.caps.weekly,
                        monthly: reservation.caps.monthly,
                    },
                    day_start: reservation.day_start,
                    week_start: reservation.week_start,
                    month_start: reservation.month_start,
                    now: reservation.now,
                },
            )
            .await
            .map_err(llm_usage_store_error)?;
        Ok(match outcome {
            tl_storage::ReserveLlmBudgetResult::Reserved => ReserveLlmBudgetOutcome::Reserved,
            tl_storage::ReserveLlmBudgetResult::Exceeded {
                window,
                cap_nanos,
                committed_nanos,
                requested_nanos,
            } => ReserveLlmBudgetOutcome::Exceeded {
                window: match window {
                    tl_storage::LlmBudgetWindow::Day => LlmBudgetWindow::Day,
                    tl_storage::LlmBudgetWindow::Week => LlmBudgetWindow::Week,
                    tl_storage::LlmBudgetWindow::Month => LlmBudgetWindow::Month,
                },
                cap_nanos,
                committed_nanos,
                requested_nanos,
            },
        })
    }

    async fn settle_budget(
        &self,
        workspace_id: &str,
        request_id: &str,
        event: RecordLlmUsageEvent,
    ) -> Result<(), LlmUsageStoreError> {
        self.0
            .settle_budget(
                workspace_id,
                request_id,
                tl_storage::NewLlmUsageEventParams {
                    principal_id: event.principal_id,
                    api_key_id: event.api_key_id,
                    model: event.model,
                    prompt_tokens: event.prompt_tokens,
                    completion_tokens: event.completion_tokens,
                    cost_minor: event.cost_minor,
                    cost_nanos: event.cost_nanos,
                    currency: event.currency,
                    request_id: event.request_id,
                    metadata: event.metadata,
                },
            )
            .await
            .map_err(llm_usage_store_error)
    }

    async fn release_budget(
        &self,
        workspace_id: &str,
        request_id: &str,
    ) -> Result<(), LlmUsageStoreError> {
        self.0
            .release_budget(workspace_id, request_id)
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
                unpriced: row.unpriced,
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
    LlmUsageStoreError::Internal(error.to_string())
}
