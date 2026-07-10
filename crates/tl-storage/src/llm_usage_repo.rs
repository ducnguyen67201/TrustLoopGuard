//! LLM gateway usage metering repository.
//!
//! Append-only `llm_usage_events` rows plus durable maximum-cost
//! reservations for budgeted Gateway calls. Per-principal lock rows
//! serialize admission across replicas; settled events retain USD-nano
//! precision while public totals remain in minor units.
//! `// ponytail: window sums load every row; push SUM(cost_nanos) into SQL if spend windows get hot`

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel_async::{AsyncConnection, RunQueryDsl};
use uuid::Uuid;

use crate::models::{
    LlmUsageEventRecord, NewLlmBudgetPrincipalLock, NewLlmBudgetReservation, NewLlmUsageEvent,
};
use crate::postgres::{DbConnection, DbPool};
use crate::schema::{llm_budget_principal_locks, llm_budget_reservations, llm_usage_events};
use crate::StorageError;

/// Cap on raw event listings; grouped queries aggregate in SQL and
/// don't need one.
const LIST_EVENTS_LIMIT: i64 = 1000;
const NANOS_PER_MINOR: i64 = 10_000_000;

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
    pub cost_nanos: i64,
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
    pub cost_nanos: i64,
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LlmBudgetCapsNanos {
    pub daily: Option<i64>,
    pub weekly: Option<i64>,
    pub monthly: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewLlmBudgetReservationParams {
    pub request_id: String,
    pub principal_id: String,
    pub api_key_id: String,
    pub currency: String,
    pub reserved_nanos: i64,
    pub caps: LlmBudgetCapsNanos,
    pub day_start: DateTime<Utc>,
    pub week_start: DateTime<Utc>,
    pub month_start: DateTime<Utc>,
    pub now: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LlmBudgetWindow {
    Day,
    Week,
    Month,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReserveLlmBudgetResult {
    Reserved,
    Exceeded {
        window: LlmBudgetWindow,
        cap_nanos: i64,
        committed_nanos: i64,
        requested_nanos: i64,
    },
}

/// One rollup row. `key` is the day (`YYYY-MM-DD`, UTC), principal,
/// or model depending on the grouping.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LlmUsageBucketRow {
    pub key: String,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub cost_minor: i64,
    pub cost_nanos: i64,
    pub calls: i64,
    pub unpriced: Option<bool>,
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
            cost_nanos: params.cost_nanos,
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

    /// Atomically reserve the maximum possible cost for one provider
    /// request. The principal lock serializes all replicas before they
    /// read committed spend and active reservations.
    pub async fn reserve_budget(
        &self,
        workspace_id: &str,
        params: NewLlmBudgetReservationParams,
    ) -> Result<ReserveLlmBudgetResult, StorageError> {
        let workspace_id = workspace_id.to_string();
        let mut conn = self.connection().await?;
        conn.transaction::<_, StorageError, _>(async move |conn| {
            diesel::insert_into(llm_budget_principal_locks::table)
                .values(&NewLlmBudgetPrincipalLock {
                    workspace_id: workspace_id.clone(),
                    principal_id: params.principal_id.clone(),
                })
                .on_conflict((
                    llm_budget_principal_locks::workspace_id,
                    llm_budget_principal_locks::principal_id,
                ))
                .do_nothing()
                .execute(conn)
                .await
                .map_err(|error| {
                    StorageError::Internal(format!("llm budget lock insert: {error}"))
                })?;

            llm_budget_principal_locks::table
                .filter(llm_budget_principal_locks::workspace_id.eq(&workspace_id))
                .filter(llm_budget_principal_locks::principal_id.eq(&params.principal_id))
                .select((
                    llm_budget_principal_locks::workspace_id,
                    llm_budget_principal_locks::principal_id,
                ))
                .for_update()
                .first::<(String, String)>(conn)
                .await
                .map_err(|error| {
                    StorageError::Internal(format!("llm budget principal lock: {error}"))
                })?;

            for (window, start, cap) in [
                (LlmBudgetWindow::Day, params.day_start, params.caps.daily),
                (LlmBudgetWindow::Week, params.week_start, params.caps.weekly),
                (
                    LlmBudgetWindow::Month,
                    params.month_start,
                    params.caps.monthly,
                ),
            ] {
                let Some(cap_nanos) = cap else { continue };
                let spent = usage_nanos_in_window(
                    conn,
                    &workspace_id,
                    &params.principal_id,
                    &params.currency,
                    start,
                    params.now,
                )
                .await?;
                let active = active_reservation_nanos_in_window(
                    conn,
                    &workspace_id,
                    &params.principal_id,
                    &params.currency,
                    start,
                    params.now,
                )
                .await?;
                let committed_nanos = spent.saturating_add(active);
                if committed_nanos.saturating_add(params.reserved_nanos) > cap_nanos {
                    return Ok(ReserveLlmBudgetResult::Exceeded {
                        window,
                        cap_nanos,
                        committed_nanos,
                        requested_nanos: params.reserved_nanos,
                    });
                }
            }

            diesel::insert_into(llm_budget_reservations::table)
                .values(&NewLlmBudgetReservation {
                    workspace_id: workspace_id.clone(),
                    request_id: params.request_id,
                    principal_id: params.principal_id,
                    api_key_id: params.api_key_id,
                    currency: params.currency.to_uppercase(),
                    reserved_nanos: params.reserved_nanos,
                    actual_nanos: None,
                    status: "active".to_string(),
                })
                .execute(conn)
                .await
                .map_err(|error| {
                    StorageError::Internal(format!("llm budget reservation insert: {error}"))
                })?;
            Ok(ReserveLlmBudgetResult::Reserved)
        })
        .await
    }

    /// Record actual usage before releasing the reservation. This
    /// ordering can temporarily double-count on failure, but can never
    /// undercount and admit spend above the cap.
    pub async fn settle_budget(
        &self,
        workspace_id: &str,
        request_id: &str,
        event: NewLlmUsageEventParams,
    ) -> Result<(), StorageError> {
        let actual_nanos = event.cost_nanos;
        self.insert_event(workspace_id, event).await?;
        let mut conn = self.connection().await?;
        diesel::update(
            llm_budget_reservations::table
                .filter(llm_budget_reservations::workspace_id.eq(workspace_id))
                .filter(llm_budget_reservations::request_id.eq(request_id))
                .filter(llm_budget_reservations::status.eq("active")),
        )
        .set((
            llm_budget_reservations::status.eq("settled"),
            llm_budget_reservations::actual_nanos.eq(Some(actual_nanos)),
            llm_budget_reservations::updated_at.eq(Utc::now()),
        ))
        .execute(&mut conn)
        .await
        .map_err(|error| StorageError::Internal(format!("llm budget settle: {error}")))?;
        Ok(())
    }

    pub async fn release_budget(
        &self,
        workspace_id: &str,
        request_id: &str,
    ) -> Result<(), StorageError> {
        let mut conn = self.connection().await?;
        diesel::update(
            llm_budget_reservations::table
                .filter(llm_budget_reservations::workspace_id.eq(workspace_id))
                .filter(llm_budget_reservations::request_id.eq(request_id))
                .filter(llm_budget_reservations::status.eq("active")),
        )
        .set((
            llm_budget_reservations::status.eq("released"),
            llm_budget_reservations::updated_at.eq(Utc::now()),
        ))
        .execute(&mut conn)
        .await
        .map_err(|error| StorageError::Internal(format!("llm budget release: {error}")))?;
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
            .select(llm_usage_events::cost_nanos)
            .load::<i64>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("llm usage spend: {e}")))?;
        let total_nanos = costs
            .into_iter()
            .fold(0_i64, |total, cost| total.saturating_add(cost));
        Ok(total_nanos / NANOS_PER_MINOR)
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
                llm_usage_events::cost_nanos,
                llm_usage_events::effective_at,
            ))
            .load::<(String, String, i64, i64, i64, i64, DateTime<Utc>)>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("llm usage grouped: {e}")))?;

        // BTreeMap keeps buckets ordered by key ascending, matching the
        // memory store's fold.
        let mut buckets: BTreeMap<String, LlmUsageBucketRow> = BTreeMap::new();
        for (
            principal_id,
            model,
            prompt_tokens,
            completion_tokens,
            _cost_minor,
            cost_nanos,
            effective_at,
        ) in rows
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
                cost_nanos: 0,
                calls: 0,
                unpriced: None,
            });
            bucket.prompt_tokens = bucket.prompt_tokens.saturating_add(prompt_tokens);
            bucket.completion_tokens = bucket.completion_tokens.saturating_add(completion_tokens);
            bucket.cost_nanos = bucket.cost_nanos.saturating_add(cost_nanos);
            bucket.cost_minor = bucket.cost_nanos / NANOS_PER_MINOR;
            bucket.calls += 1;
            if group_by == LlmUsageGroupBy::Model
                && cost_nanos == 0
                && prompt_tokens.saturating_add(completion_tokens) > 0
            {
                bucket.unpriced = Some(true);
            }
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

async fn usage_nanos_in_window(
    conn: &mut diesel_async::AsyncPgConnection,
    workspace_id: &str,
    principal_id: &str,
    currency: &str,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> Result<i64, StorageError> {
    let costs = llm_usage_events::table
        .filter(llm_usage_events::workspace_id.eq(workspace_id))
        .filter(llm_usage_events::principal_id.eq(principal_id))
        .filter(llm_usage_events::currency.eq(currency.to_uppercase()))
        .filter(llm_usage_events::effective_at.ge(start))
        .filter(llm_usage_events::effective_at.lt(end))
        .select(llm_usage_events::cost_nanos)
        .load::<i64>(conn)
        .await
        .map_err(|error| StorageError::Internal(format!("llm usage reservation sum: {error}")))?;
    Ok(costs
        .into_iter()
        .fold(0_i64, |total, cost| total.saturating_add(cost)))
}

async fn active_reservation_nanos_in_window(
    conn: &mut diesel_async::AsyncPgConnection,
    workspace_id: &str,
    principal_id: &str,
    currency: &str,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> Result<i64, StorageError> {
    let reservations = llm_budget_reservations::table
        .filter(llm_budget_reservations::workspace_id.eq(workspace_id))
        .filter(llm_budget_reservations::principal_id.eq(principal_id))
        .filter(llm_budget_reservations::currency.eq(currency.to_uppercase()))
        .filter(llm_budget_reservations::status.eq("active"))
        .filter(llm_budget_reservations::created_at.ge(start))
        .filter(llm_budget_reservations::created_at.lt(end))
        .select(llm_budget_reservations::reserved_nanos)
        .load::<i64>(conn)
        .await
        .map_err(|error| StorageError::Internal(format!("llm active reservation sum: {error}")))?;
    Ok(reservations
        .into_iter()
        .fold(0_i64, |total, cost| total.saturating_add(cost)))
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
        cost_nanos: row.cost_nanos,
        currency: row.currency,
        request_id: row.request_id,
        metadata: row.metadata,
        effective_at: row.effective_at,
    }
}
