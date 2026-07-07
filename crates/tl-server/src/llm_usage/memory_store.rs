use std::collections::{BTreeMap, HashSet};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use tl_core::{LlmUsageBucket, LlmUsageBucketsResponse, LlmUsageEvent, LlmUsageListResponse};
use tokio::sync::RwLock;

use super::{
    LlmUsageFilter, LlmUsageGroupBy, LlmUsageStore, LlmUsageStoreError, RecordLlmUsageEvent,
};

/// Raw-listing cap, mirroring the postgres repo's `LIST_EVENTS_LIMIT`
/// so both `LlmUsageStore` backends truncate identically.
const LIST_EVENTS_LIMIT: usize = 1000;

#[derive(Debug, Default)]
pub struct MemoryLlmUsageStore {
    /// Stored events plus the parsed `effective_at` used for window
    /// filtering (the API struct carries it as an RFC 3339 string).
    events: RwLock<Vec<(DateTime<Utc>, LlmUsageEvent)>>,
    /// Seen `(workspace_id, request_id)` pairs — a tuple, not a joined
    /// string, so ids containing the separator can't collide.
    request_ids: RwLock<HashSet<(String, String)>>,
}

impl MemoryLlmUsageStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl LlmUsageStore for MemoryLlmUsageStore {
    async fn insert_event(
        &self,
        workspace_id: &str,
        event: RecordLlmUsageEvent,
    ) -> Result<(), LlmUsageStoreError> {
        let scoped_request_id = (workspace_id.to_string(), event.request_id.clone());
        let mut request_ids = self.request_ids.write().await;
        if request_ids.contains(&scoped_request_id) {
            // Retried metering write — first row wins, mirroring the
            // postgres ON CONFLICT DO NOTHING.
            return Ok(());
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
        request_ids.insert(scoped_request_id);
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
        let currency = currency.to_uppercase();
        Ok(self
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
                total.saturating_add(event.cost_minor)
            }))
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
        // BTreeMap keeps buckets ordered by key ascending, matching the
        // SQL `GROUP BY 1 ORDER BY 1`.
        let mut buckets: BTreeMap<String, LlmUsageBucket> = BTreeMap::new();
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
                key,
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
            bucket.cost_minor = bucket.cost_minor.saturating_add(event.cost_minor);
            bucket.calls += 1;
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
        RecordLlmUsageEvent {
            principal_id: principal.into(),
            api_key_id: "key_1".into(),
            model: model.into(),
            prompt_tokens: 100,
            completion_tokens: 10,
            cost_minor: 5,
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
