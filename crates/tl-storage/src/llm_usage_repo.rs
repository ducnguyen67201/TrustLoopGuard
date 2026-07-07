//! LLM gateway usage metering repository.
//!
//! Append-only `llm_usage_events` rows: one per metered gateway chat
//! completion. `net_llm_spend_minor` mirrors the financial ledger's
//! window-sum query shape (`FinancialRepo::net_spend_minor`) but is a
//! plain `SUM(cost_minor)` — usage events have no signed entry kinds.

use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::sql_types::{Nullable, Text, Timestamptz};
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

/// One SQL rollup row. `key` is the day (`YYYY-MM-DD`, UTC), principal,
/// or model depending on the grouping.
#[derive(Debug, Clone, PartialEq, Eq, QueryableByName)]
pub struct LlmUsageBucketRow {
    #[diesel(sql_type = Text)]
    pub key: String,
    #[diesel(sql_type = diesel::sql_types::Int8)]
    pub prompt_tokens: i64,
    #[diesel(sql_type = diesel::sql_types::Int8)]
    pub completion_tokens: i64,
    #[diesel(sql_type = diesel::sql_types::Int8)]
    pub cost_minor: i64,
    #[diesel(sql_type = diesel::sql_types::Int8)]
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
        let clean_principal = clean_required("principal_id", &params.principal_id)?;
        let clean_request_id = clean_required("request_id", &params.request_id)?;
        let clean_currency = clean_required("currency", &params.currency)?.to_uppercase();
        if params.prompt_tokens < 0 || params.completion_tokens < 0 || params.cost_minor < 0 {
            return Err(StorageError::Internal(
                "llm usage tokens and cost must be non-negative".into(),
            ));
        }
        let row = NewLlmUsageEvent {
            workspace_id: workspace_id.to_string(),
            id: Uuid::now_v7(),
            principal_id: clean_principal,
            api_key_id: params.api_key_id,
            model: params.model,
            prompt_tokens: params.prompt_tokens,
            completion_tokens: params.completion_tokens,
            cost_minor: params.cost_minor,
            currency: clean_currency,
            request_id: clean_request_id,
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

    /// Total priced spend for one principal in `[start, end)`. Plain
    /// `SUM(cost_minor)` — no signed kinds, usage only accrues.
    pub async fn net_llm_spend_minor(
        &self,
        workspace_id: &str,
        principal_id: &str,
        currency: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<i64, StorageError> {
        let clean_principal = clean_required("principal_id", principal_id)?;
        let clean_currency = clean_required("currency", currency)?.to_uppercase();
        let mut conn = self.connection().await?;
        let costs = llm_usage_events::table
            .filter(llm_usage_events::workspace_id.eq(workspace_id))
            .filter(llm_usage_events::principal_id.eq(clean_principal))
            .filter(llm_usage_events::currency.eq(clean_currency))
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

    /// Raw event list, newest first, capped at [`LIST_EVENTS_LIMIT`].
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

    /// Rollup by day (UTC), principal, or model. Grouping happens in
    /// SQL; the memory store folds the exact same buckets.
    pub async fn grouped_usage(
        &self,
        workspace_id: &str,
        group_by: LlmUsageGroupBy,
        filter: &LlmUsageEventFilter,
    ) -> Result<Vec<LlmUsageBucketRow>, StorageError> {
        let key_expr = match group_by {
            LlmUsageGroupBy::Day => {
                "to_char(date_trunc('day', effective_at AT TIME ZONE 'UTC'), 'YYYY-MM-DD')"
            }
            LlmUsageGroupBy::Principal => "principal_id",
            LlmUsageGroupBy::Model => "model",
        };
        let sql = format!(
            "SELECT {key_expr} AS key, \
                    COALESCE(SUM(prompt_tokens), 0)::bigint AS prompt_tokens, \
                    COALESCE(SUM(completion_tokens), 0)::bigint AS completion_tokens, \
                    COALESCE(SUM(cost_minor), 0)::bigint AS cost_minor, \
                    COUNT(*)::bigint AS calls \
             FROM llm_usage_events \
             WHERE workspace_id = $1 \
               AND ($2 IS NULL OR principal_id = $2) \
               AND ($3 IS NULL OR model = $3) \
               AND ($4 IS NULL OR effective_at >= $4) \
               AND ($5 IS NULL OR effective_at < $5) \
             GROUP BY 1 \
             ORDER BY 1"
        );
        let mut conn = self.connection().await?;
        diesel::sql_query(sql)
            .bind::<Text, _>(workspace_id)
            .bind::<Nullable<Text>, _>(filter.principal_id.clone())
            .bind::<Nullable<Text>, _>(filter.model.clone())
            .bind::<Nullable<Timestamptz>, _>(filter.start)
            .bind::<Nullable<Timestamptz>, _>(filter.end)
            .load::<LlmUsageBucketRow>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("llm usage grouped: {e}")))
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

fn clean_required(name: &str, value: &str) -> Result<String, StorageError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(StorageError::Internal(format!("{name} must not be empty")));
    }
    Ok(trimmed.to_string())
}
