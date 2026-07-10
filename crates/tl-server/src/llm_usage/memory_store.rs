use std::collections::{BTreeMap, HashMap, HashSet};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use tl_core::{LlmUsageBucket, LlmUsageBucketsResponse, LlmUsageEvent, LlmUsageListResponse};
use tokio::sync::{Mutex, RwLock};

use super::{
    LlmBudgetWindow, LlmUsageFilter, LlmUsageGroupBy, LlmUsageStore, LlmUsageStoreError,
    RecordLlmUsageEvent, ReserveLlmBudget, ReserveLlmBudgetOutcome,
};

/// Raw-listing cap, mirroring the postgres repo's `LIST_EVENTS_LIMIT`
/// so both `LlmUsageStore` backends truncate identically.
const LIST_EVENTS_LIMIT: usize = 1000;
const NANOS_PER_MINOR: i64 = 10_000_000;

#[derive(Debug, Clone)]
struct MemoryBudgetReservation {
    workspace_id: String,
    request_id: String,
    principal_id: String,
    currency: String,
    reserved_nanos: i64,
    status: &'static str,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Default)]
pub struct MemoryLlmUsageStore {
    /// Serializes usage writes and budget reservations so component
    /// tests exercise the same per-principal atomicity as Postgres.
    mutation: Mutex<()>,
    /// Stored events plus the parsed `effective_at` used for window
    /// filtering (the API struct carries it as an RFC 3339 string).
    events: RwLock<Vec<(DateTime<Utc>, LlmUsageEvent)>>,
    /// Seen `(workspace_id, request_id)` pairs — a tuple, not a joined
    /// string, so ids containing the separator can't collide.
    request_ids: RwLock<HashSet<(String, String)>>,
    event_cost_nanos: RwLock<HashMap<(String, String), i64>>,
    reservations: RwLock<Vec<MemoryBudgetReservation>>,
}

impl MemoryLlmUsageStore {
    pub fn new() -> Self {
        Self::default()
    }

    async fn insert_event_locked(&self, workspace_id: &str, event: RecordLlmUsageEvent) {
        let scoped_request_id = (workspace_id.to_string(), event.request_id.clone());
        let cost_nanos = event.cost_nanos;
        let mut request_ids = self.request_ids.write().await;
        if request_ids.contains(&scoped_request_id) {
            return;
        }
        let effective_at = Utc::now();
        self.events.write().await.push((
            effective_at,
            LlmUsageEvent {
                id: uuid::Uuid::now_v7().to_string(),
                workspace_id: workspace_id.to_string(),
                principal_id: event.principal_id,
                api_key_id: event.api_key_id,
                model: event.model,
                prompt_tokens: event.prompt_tokens,
                completion_tokens: event.completion_tokens,
                cost_minor: event.cost_minor,
                currency: event.currency.to_uppercase(),
                request_id: event.request_id,
                metadata: event.metadata,
                effective_at: effective_at.to_rfc3339(),
            },
        ));
        self.event_cost_nanos
            .write()
            .await
            .insert(scoped_request_id.clone(), cost_nanos);
        request_ids.insert(scoped_request_id);
    }
}

#[async_trait]
impl LlmUsageStore for MemoryLlmUsageStore {
    async fn insert_event(
        &self,
        workspace_id: &str,
        event: RecordLlmUsageEvent,
    ) -> Result<(), LlmUsageStoreError> {
        let _mutation = self.mutation.lock().await;
        self.insert_event_locked(workspace_id, event).await;
        Ok(())
    }

    async fn reserve_budget(
        &self,
        workspace_id: &str,
        reservation: ReserveLlmBudget,
    ) -> Result<ReserveLlmBudgetOutcome, LlmUsageStoreError> {
        let _mutation = self.mutation.lock().await;
        let events = self.events.read().await;
        let event_costs = self.event_cost_nanos.read().await;
        let reservations = self.reservations.read().await;
        for (window, start, cap) in [
            (
                LlmBudgetWindow::Day,
                reservation.day_start,
                reservation.caps.daily,
            ),
            (
                LlmBudgetWindow::Week,
                reservation.week_start,
                reservation.caps.weekly,
            ),
            (
                LlmBudgetWindow::Month,
                reservation.month_start,
                reservation.caps.monthly,
            ),
        ] {
            let Some(cap_nanos) = cap else { continue };
            let spent = events
                .iter()
                .filter(|(effective_at, event)| {
                    event.workspace_id == workspace_id
                        && event.principal_id == reservation.principal_id
                        && event.currency.eq_ignore_ascii_case(&reservation.currency)
                        && *effective_at >= start
                        && *effective_at < reservation.now
                })
                .fold(0_i64, |total, (_, event)| {
                    total.saturating_add(
                        *event_costs
                            .get(&(workspace_id.to_string(), event.request_id.clone()))
                            .unwrap_or(&0),
                    )
                });
            let active = reservations
                .iter()
                .filter(|existing| {
                    existing.workspace_id == workspace_id
                        && existing.principal_id == reservation.principal_id
                        && existing
                            .currency
                            .eq_ignore_ascii_case(&reservation.currency)
                        && existing.status == "active"
                        && existing.created_at >= start
                        && existing.created_at < reservation.now
                })
                .fold(0_i64, |total, existing| {
                    total.saturating_add(existing.reserved_nanos)
                });
            let committed_nanos = spent.saturating_add(active);
            if committed_nanos.saturating_add(reservation.reserved_nanos) > cap_nanos {
                return Ok(ReserveLlmBudgetOutcome::Exceeded {
                    window,
                    cap_nanos,
                    committed_nanos,
                    requested_nanos: reservation.reserved_nanos,
                });
            }
        }
        drop(reservations);
        drop(event_costs);
        drop(events);
        self.reservations
            .write()
            .await
            .push(MemoryBudgetReservation {
                workspace_id: workspace_id.to_string(),
                request_id: reservation.request_id,
                principal_id: reservation.principal_id,
                currency: reservation.currency.to_uppercase(),
                reserved_nanos: reservation.reserved_nanos,
                status: "active",
                created_at: reservation.now,
            });
        Ok(ReserveLlmBudgetOutcome::Reserved)
    }

    async fn settle_budget(
        &self,
        workspace_id: &str,
        request_id: &str,
        event: RecordLlmUsageEvent,
    ) -> Result<(), LlmUsageStoreError> {
        let _mutation = self.mutation.lock().await;
        self.insert_event_locked(workspace_id, event).await;
        if let Some(reservation) = self
            .reservations
            .write()
            .await
            .iter_mut()
            .find(|reservation| {
                reservation.workspace_id == workspace_id
                    && reservation.request_id == request_id
                    && reservation.status == "active"
            })
        {
            reservation.status = "settled";
        }
        Ok(())
    }

    async fn release_budget(
        &self,
        workspace_id: &str,
        request_id: &str,
    ) -> Result<(), LlmUsageStoreError> {
        let _mutation = self.mutation.lock().await;
        if let Some(reservation) = self
            .reservations
            .write()
            .await
            .iter_mut()
            .find(|reservation| {
                reservation.workspace_id == workspace_id
                    && reservation.request_id == request_id
                    && reservation.status == "active"
            })
        {
            reservation.status = "released";
        }
        Ok(())
    }

    async fn net_llm_spend_minor(
        &self,
        workspace_id: &str,
        principal_id: &str,
        currency: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<i64, LlmUsageStoreError> {
        let _mutation = self.mutation.lock().await;
        let currency = currency.to_uppercase();
        let event_costs = self.event_cost_nanos.read().await;
        let total_nanos = self
            .events
            .read()
            .await
            .iter()
            .filter(|(effective_at, event)| {
                event.workspace_id == workspace_id
                    && event.principal_id == principal_id
                    && event.currency == currency
                    && *effective_at >= start
                    && *effective_at < end
            })
            .fold(0_i64, |total, (_, event)| {
                total.saturating_add(
                    *event_costs
                        .get(&(workspace_id.to_string(), event.request_id.clone()))
                        .unwrap_or(&0),
                )
            });
        Ok(total_nanos / NANOS_PER_MINOR)
    }

    async fn list_events(
        &self,
        workspace_id: &str,
        filter: &LlmUsageFilter,
    ) -> Result<LlmUsageListResponse, LlmUsageStoreError> {
        let mut events = self
            .events
            .read()
            .await
            .iter()
            .filter(|(effective_at, event)| {
                event_matches(*effective_at, event, workspace_id, filter)
            })
            .cloned()
            .collect::<Vec<_>>();
        events.sort_by(|(at_a, a), (at_b, b)| at_b.cmp(at_a).then_with(|| b.id.cmp(&a.id)));
        events.truncate(LIST_EVENTS_LIMIT);
        Ok(LlmUsageListResponse {
            events: events.into_iter().map(|(_, event)| event).collect(),
        })
    }

    async fn grouped_usage(
        &self,
        workspace_id: &str,
        group_by: LlmUsageGroupBy,
        filter: &LlmUsageFilter,
    ) -> Result<LlmUsageBucketsResponse, LlmUsageStoreError> {
        let _mutation = self.mutation.lock().await;
        let event_costs = self.event_cost_nanos.read().await;
        // BTreeMap keeps buckets ordered by key ascending, matching the
        // SQL `GROUP BY 1 ORDER BY 1`.
        let mut buckets: BTreeMap<String, LlmUsageBucket> = BTreeMap::new();
        let mut bucket_nanos: HashMap<String, i64> = HashMap::new();
        for (effective_at, event) in
            self.events
                .read()
                .await
                .iter()
                .filter(|(effective_at, event)| {
                    event_matches(*effective_at, event, workspace_id, filter)
                })
        {
            let key = match group_by {
                // Same key the SQL date_trunc('day', … AT TIME ZONE
                // 'UTC') + to_char('YYYY-MM-DD') produces.
                LlmUsageGroupBy::Day => effective_at.date_naive().to_string(),
                LlmUsageGroupBy::Principal => event.principal_id.clone(),
                LlmUsageGroupBy::Model => event.model.clone(),
            };
            let bucket = buckets.entry(key.clone()).or_insert(LlmUsageBucket {
                key: key.clone(),
                prompt_tokens: 0,
                completion_tokens: 0,
                cost_minor: 0,
                calls: 0,
                unpriced: None,
            });
            bucket.prompt_tokens = bucket.prompt_tokens.saturating_add(event.prompt_tokens);
            bucket.completion_tokens = bucket
                .completion_tokens
                .saturating_add(event.completion_tokens);
            let cost_nanos = *event_costs
                .get(&(workspace_id.to_string(), event.request_id.clone()))
                .unwrap_or(&0);
            let total_nanos = bucket_nanos.entry(key).or_default();
            *total_nanos = total_nanos.saturating_add(cost_nanos);
            bucket.cost_minor = *total_nanos / NANOS_PER_MINOR;
            bucket.calls += 1;
            if group_by == LlmUsageGroupBy::Model
                && cost_nanos == 0
                && event.prompt_tokens.saturating_add(event.completion_tokens) > 0
            {
                bucket.unpriced = Some(true);
            }
        }
        Ok(LlmUsageBucketsResponse {
            buckets: buckets.into_values().collect(),
        })
    }
}

fn event_matches(
    effective_at: DateTime<Utc>,
    event: &LlmUsageEvent,
    workspace_id: &str,
    filter: &LlmUsageFilter,
) -> bool {
    event.workspace_id == workspace_id
        && filter
            .principal_id
            .as_deref()
            .map_or(true, |principal| event.principal_id == principal)
        && filter
            .model
            .as_deref()
            .map_or(true, |model| event.model == model)
        && filter.start.map_or(true, |start| effective_at >= start)
        && filter.end.map_or(true, |end| effective_at < end)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(principal: &str, model: &str, request_id: &str) -> RecordLlmUsageEvent {
        event_with_cost(principal, model, request_id, 5)
    }

    fn event_with_cost(
        principal: &str,
        model: &str,
        request_id: &str,
        cost_minor: i64,
    ) -> RecordLlmUsageEvent {
        RecordLlmUsageEvent {
            principal_id: principal.into(),
            api_key_id: "key_1".into(),
            model: model.into(),
            prompt_tokens: 100,
            completion_tokens: 10,
            cost_minor,
            cost_nanos: cost_minor.saturating_mul(NANOS_PER_MINOR),
            currency: "USD".into(),
            request_id: request_id.into(),
            metadata: serde_json::json!({}),
        }
    }

    #[tokio::test]
    async fn grouped_usage_folds_by_principal_and_model() {
        let store = MemoryLlmUsageStore::new();
        store
            .insert_event("ws", event("user:a", "m1", "r1"))
            .await
            .unwrap();
        store
            .insert_event("ws", event("user:a", "m2", "r2"))
            .await
            .unwrap();
        store
            .insert_event("ws", event("user:b", "m1", "r3"))
            .await
            .unwrap();
        store
            .insert_event("ws_other", event("user:a", "m1", "r4"))
            .await
            .unwrap();

        let by_principal = store
            .grouped_usage("ws", LlmUsageGroupBy::Principal, &LlmUsageFilter::default())
            .await
            .unwrap()
            .buckets;
        assert_eq!(by_principal.len(), 2);
        assert_eq!(by_principal[0].key, "user:a");
        assert_eq!(by_principal[0].calls, 2);
        assert_eq!(by_principal[0].prompt_tokens, 200);
        assert_eq!(by_principal[0].cost_minor, 10);
        assert_eq!(by_principal[1].key, "user:b");
        assert_eq!(by_principal[1].calls, 1);

        let by_model = store
            .grouped_usage("ws", LlmUsageGroupBy::Model, &LlmUsageFilter::default())
            .await
            .unwrap()
            .buckets;
        assert_eq!(by_model.len(), 2);
        assert_eq!(by_model[0].key, "m1");
        assert_eq!(by_model[0].calls, 2);
    }

    #[tokio::test]
    async fn grouped_model_usage_preserves_zero_cost_undercount_signal() {
        let store = MemoryLlmUsageStore::new();
        store
            .insert_event("ws", event_with_cost("user:a", "unknown-model", "r1", 0))
            .await
            .unwrap();
        store
            .insert_event("ws", event_with_cost("user:a", "unknown-model", "r2", 10))
            .await
            .unwrap();

        let by_model = store
            .grouped_usage("ws", LlmUsageGroupBy::Model, &LlmUsageFilter::default())
            .await
            .unwrap()
            .buckets;
        assert_eq!(by_model.len(), 1);
        assert_eq!(by_model[0].key, "unknown-model");
        assert_eq!(by_model[0].cost_minor, 10);
        assert_eq!(by_model[0].unpriced, Some(true));
    }

    #[tokio::test]
    async fn grouped_usage_accumulates_sub_cent_precision_before_rounding() {
        let store = MemoryLlmUsageStore::new();
        for request_id in ["r1", "r2"] {
            let mut usage = event_with_cost("user:a", "cheap-model", request_id, 0);
            usage.cost_nanos = 6_000_000;
            store.insert_event("ws", usage).await.unwrap();
        }

        let buckets = store
            .grouped_usage("ws", LlmUsageGroupBy::Model, &LlmUsageFilter::default())
            .await
            .unwrap()
            .buckets;
        assert_eq!(buckets[0].cost_minor, 1);
        assert!(buckets[0].unpriced.is_none());
    }

    #[tokio::test]
    async fn grouped_usage_by_day_uses_utc_date_key() {
        let store = MemoryLlmUsageStore::new();
        store
            .insert_event("ws", event("user:a", "m1", "r1"))
            .await
            .unwrap();
        let buckets = store
            .grouped_usage("ws", LlmUsageGroupBy::Day, &LlmUsageFilter::default())
            .await
            .unwrap()
            .buckets;
        assert_eq!(buckets.len(), 1);
        assert_eq!(buckets[0].key, Utc::now().date_naive().to_string());
        assert_eq!(buckets[0].calls, 1);
    }

    #[tokio::test]
    async fn duplicate_request_id_is_a_noop() {
        let store = MemoryLlmUsageStore::new();
        store
            .insert_event("ws", event("user:a", "m1", "r1"))
            .await
            .unwrap();
        store
            .insert_event("ws", event("user:a", "m1", "r1"))
            .await
            .unwrap();
        let events = store
            .list_events("ws", &LlmUsageFilter::default())
            .await
            .unwrap()
            .events;
        assert_eq!(events.len(), 1);
    }

    #[tokio::test]
    async fn window_sum_respects_principal_and_bounds() {
        let store = MemoryLlmUsageStore::new();
        store
            .insert_event("ws", event("user:a", "m1", "r1"))
            .await
            .unwrap();
        store
            .insert_event("ws", event("user:b", "m1", "r2"))
            .await
            .unwrap();
        let now = Utc::now();
        let spent = store
            .net_llm_spend_minor(
                "ws",
                "user:a",
                "usd",
                now - chrono::Duration::hours(1),
                now + chrono::Duration::hours(1),
            )
            .await
            .unwrap();
        assert_eq!(spent, 5);
        let outside = store
            .net_llm_spend_minor(
                "ws",
                "user:a",
                "USD",
                now - chrono::Duration::hours(2),
                now - chrono::Duration::hours(1),
            )
            .await
            .unwrap();
        assert_eq!(outside, 0);
    }
}
