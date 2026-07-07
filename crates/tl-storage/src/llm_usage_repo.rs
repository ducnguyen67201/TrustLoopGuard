//! LLM gateway usage metering repository.
//!
//! Append-only `llm_usage_events` rows: one per metered gateway chat
//! completion. `net_llm_spend_minor` mirrors the financial ledger's
//! window-sum shape (`FinancialRepo::net_spend_minor`): rows are loaded
//! and folded client-side — usage events have no signed entry kinds, so
//! the fold is a plain sum.
//! `// ponytail: window sums load every row; push SUM(cost_minor) into SQL (needs diesel numeric/bigdecimal) if spend windows get hot`

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use uuid::Uuid;

use crate::models::{LlmUsageEventRecord, NewLlmUsageEvent};
use crate::postgres::{DbConnection, DbPool};
use crate::schema::llm_usage_events;
use crate::StorageError;

/// Cap on raw event listings; grouped queries aggregate in SQL and
/// don't need one.
const LIST_EVENTS_LIMIT: i64 = 1000;

#[derive(Debug, Clone, PartialEq)]
pub struct StoredLlmUsageEvent {
    pub workspace_id: String,
    pub id: String,
    pub principal_id: String,
    pub api_key_id: String,
    pub model: String,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub cost_minor: i64,
    pub currency: String,
    pub request_id: String,
    pub metadata: serde_json::Value,
    pub effective_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NewLlmUsageEventParams {
    pub principal_id: String,
    pub api_key_id: String,
    pub model: String,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub cost_minor: i64,
    pub currency: String,
    pub request_id: String,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct LlmUsageEventFilter {
    pub principal_id: Option<String>,
    pub model: Option<String>,
    pub start: Option<DateTime<Utc>>,
    pub end: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LlmUsageGroupBy {
    Day,
    Principal,
    Model,
}

/// One rollup row. `key` is the day (`YYYY-MM-DD`, UTC), principal,
/// or model depending on the grouping.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LlmUsageBucketRow {
    pub key: String,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub cost_minor: i64,
    pub calls: i64,
}

pub struct LlmUsageRepo {
    pool: DbPool,
}

impl LlmUsageRepo {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Insert one metered call. Idempotent on `(workspace_id,
    /// request_id)`: a retried metering write for the same gateway
    /// request is a no-op, never a duplicate row.
    pub async fn insert_event(
        &self,
        workspace_id: &str,
        params: NewLlmUsageEventParams,
    ) -> Result<(), StorageError> {
        let row = NewLlmUsageEvent {
            workspace_id: workspace_id.to_string(),
            id: Uuid::now_v7(),
            principal_id: params.principal_id,
            api_key_id: params.api_key_id,
            model: params.model,
            prompt_tokens: params.prompt_tokens,
            completion_tokens: params.completion_tokens,
            cost_minor: params.cost_minor,
            currency: params.currency.to_uppercase(),
            request_id: params.request_id,
            metadata: params.metadata,
        };
        let mut conn = self.connection().await?;
        diesel::insert_into(llm_usage_events::table)
            .values(&row)
            .on_conflict((llm_usage_events::workspace_id, llm_usage_events::request_id))
            .do_nothing()
            .execute(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("llm usage insert: {e}")))?;
        Ok(())
    }

    /// Total priced spend for one principal in `[start, end)`. Loads
    /// the window's `cost_minor` values and folds them — no signed
    /// kinds, usage only accrues.
    pub async fn net_llm_spend_minor(
        &self,
        workspace_id: &str,
        principal_id: &str,
        currency: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<i64, StorageError> {
        let mut conn = self.connection().await?;
        let costs = llm_usage_events::table
            .filter(llm_usage_events::workspace_id.eq(workspace_id))
            .filter(llm_usage_events::principal_id.eq(principal_id))
            .filter(llm_usage_events::currency.eq(currency.to_uppercase()))
            .filter(llm_usage_events::effective_at.ge(start))
            .filter(llm_usage_events::effective_at.lt(end))
            .select(llm_usage_events::cost_minor)
            .load::<i64>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("llm usage spend: {e}")))?;
        Ok(costs
            .into_iter()
            .fold(0_i64, |total, cost| total.saturating_add(cost)))
    }

    /// Raw event list, newest first, capped at `LIST_EVENTS_LIMIT`
    /// (1000 rows).
    pub async fn list_events(
        &self,
        workspace_id: &str,
        filter: &LlmUsageEventFilter,
    ) -> Result<Vec<StoredLlmUsageEvent>, StorageError> {
        let mut conn = self.connection().await?;
        let mut query = llm_usage_events::table
            .filter(llm_usage_events::workspace_id.eq(workspace_id))
            .into_boxed();
        if let Some(principal_id) = &filter.principal_id {
            query = query.filter(llm_usage_events::principal_id.eq(principal_id.clone()));
        }
        if let Some(model) = &filter.model {
            query = query.filter(llm_usage_events::model.eq(model.clone()));
        }
        if let Some(start) = filter.start {
            query = query.filter(llm_usage_events::effective_at.ge(start));
        }
        if let Some(end) = filter.end {
            query = query.filter(llm_usage_events::effective_at.lt(end));
        }
        let rows = query
            .order((
                llm_usage_events::effective_at.desc(),
                llm_usage_events::id.desc(),
            ))
            .limit(LIST_EVENTS_LIMIT)
            .select(LlmUsageEventRecord::as_select())
            .load::<LlmUsageEventRecord>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("llm usage list: {e}")))?;
        Ok(rows.into_iter().map(stored_event).collect())
    }

    /// Rollup by day (UTC date key), principal, or model — rows are
    /// loaded with the typed DSL and folded in Rust, exactly like the
    /// memory store, so both backends produce identical buckets.
    /// `// ponytail: loads all rows in the window; switch to typed group_by aggregates if usage volume makes this hot`
    pub async fn grouped_usage(
        &self,
        workspace_id: &str,
        group_by: LlmUsageGroupBy,
        filter: &LlmUsageEventFilter,
    ) -> Result<Vec<LlmUsageBucketRow>, StorageError> {
        let mut conn = self.connection().await?;
        let mut query = llm_usage_events::table
            .filter(llm_usage_events::workspace_id.eq(workspace_id))
            .into_boxed();
        if let Some(principal_id) = &filter.principal_id {
            query = query.filter(llm_usage_events::principal_id.eq(principal_id.clone()));
        }
        if let Some(model) = &filter.model {
            query = query.filter(llm_usage_events::model.eq(model.clone()));
        }
        if let Some(start) = filter.start {
            query = query.filter(llm_usage_events::effective_at.ge(start));
        }
        if let Some(end) = filter.end {
            query = query.filter(llm_usage_events::effective_at.lt(end));
        }
        let rows = query
            .select((
                llm_usage_events::principal_id,
                llm_usage_events::model,
                llm_usage_events::prompt_tokens,
                llm_usage_events::completion_tokens,
                llm_usage_events::cost_minor,
                llm_usage_events::effective_at,
            ))
            .load::<(String, String, i64, i64, i64, DateTime<Utc>)>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("llm usage grouped: {e}")))?;

        // BTreeMap keeps buckets ordered by key ascending, matching the
        // memory store's fold.
        let mut buckets: BTreeMap<String, LlmUsageBucketRow> = BTreeMap::new();
        for (principal_id, model, prompt_tokens, completion_tokens, cost_minor, effective_at) in
            rows
        {
            let key = match group_by {
                // `YYYY-MM-DD`, UTC — the RFC 3339 date key.
                LlmUsageGroupBy::Day => effective_at.date_naive().to_string(),
                LlmUsageGroupBy::Principal => principal_id,
                LlmUsageGroupBy::Model => model,
            };
            let bucket = buckets.entry(key.clone()).or_insert(LlmUsageBucketRow {
                key,
                prompt_tokens: 0,
                completion_tokens: 0,
                cost_minor: 0,
                calls: 0,
            });
            bucket.prompt_tokens = bucket.prompt_tokens.saturating_add(prompt_tokens);
            bucket.completion_tokens = bucket.completion_tokens.saturating_add(completion_tokens);
            bucket.cost_minor = bucket.cost_minor.saturating_add(cost_minor);
            bucket.calls += 1;
        }
        Ok(buckets.into_values().collect())
    }

    async fn connection(&self) -> Result<DbConnection<'_>, StorageError> {
        self.pool
            .get()
            .await
            .map_err(|e| StorageError::Internal(format!("db pool: {e}")))
    }
}

impl std::fmt::Debug for LlmUsageRepo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LlmUsageRepo").finish_non_exhaustive()
    }
}

fn stored_event(row: LlmUsageEventRecord) -> StoredLlmUsageEvent {
    StoredLlmUsageEvent {
        workspace_id: row.workspace_id,
        id: row.id.to_string(),
        principal_id: row.principal_id,
        api_key_id: row.api_key_id,
        model: row.model,
        prompt_tokens: row.prompt_tokens,
        completion_tokens: row.completion_tokens,
        cost_minor: row.cost_minor,
        currency: row.currency,
        request_id: row.request_id,
        metadata: row.metadata,
        effective_at: row.effective_at,
    }
}
