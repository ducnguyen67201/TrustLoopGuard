use std::collections::HashMap;

use async_trait::async_trait;
use tl_core::{
    CreateHumanReviewEventRequest, HumanReviewAnalyticsResponse, HumanReviewAnalyticsSummary,
    HumanReviewEvent, HumanReviewOutcomeCounts,
};
use tokio::sync::RwLock;

use super::{
    validation::{clean_string, normalize_metadata, validate_create_event},
    HumanReviewAnalyticsFilter, HumanReviewStore, HumanReviewStoreError,
};

#[derive(Debug, Default)]
pub struct MemoryHumanReviewStore {
    events: RwLock<HashMap<String, Vec<HumanReviewEvent>>>,
}

impl MemoryHumanReviewStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl HumanReviewStore for MemoryHumanReviewStore {
    async fn create_event(
        &self,
        workspace_id: &str,
        trace_id: &str,
        input: CreateHumanReviewEventRequest,
        reviewer_id: Option<String>,
    ) -> Result<HumanReviewEvent, HumanReviewStoreError> {
        validate_create_event(&input)?;
        let now = chrono::Utc::now().to_rfc3339();
        let event = HumanReviewEvent {
            id: uuid::Uuid::now_v7().to_string(),
            workspace_id: workspace_id.to_string(),
            trace_id: trace_id.to_string(),
            run_id: None,
            run_event_id: None,
            outcome: input.outcome,
            reason_codes: input
                .reason_codes
                .into_iter()
                .filter_map(|value| clean_string(value.trim()))
                .collect(),
            note: input.note.and_then(|value| clean_string(value.trim())),
            reviewer_id: reviewer_id.and_then(|value| clean_string(value.trim())),
            metadata: normalize_metadata(input.metadata),
            created_at: now,
        };
        self.events
            .write()
            .await
            .entry(key(workspace_id, trace_id))
            .or_default()
            .push(event.clone());
        Ok(event)
    }

    async fn list_events(
        &self,
        workspace_id: &str,
        trace_id: &str,
        limit: usize,
    ) -> Result<Vec<HumanReviewEvent>, HumanReviewStoreError> {
        let events = self.events.read().await;
        let mut rows = events
            .get(&key(workspace_id, trace_id))
            .cloned()
            .unwrap_or_default();
        rows.truncate(limit.clamp(1, 100));
        Ok(rows)
    }

    async fn analytics(
        &self,
        _workspace_id: &str,
        _filter: HumanReviewAnalyticsFilter,
    ) -> Result<HumanReviewAnalyticsResponse, HumanReviewStoreError> {
        Ok(empty_analytics())
    }
}

fn key(workspace_id: &str, trace_id: &str) -> String {
    format!("{workspace_id}:{trace_id}")
}

fn empty_analytics() -> HumanReviewAnalyticsResponse {
    HumanReviewAnalyticsResponse {
        summary: HumanReviewAnalyticsSummary::default(),
        outcomes: HumanReviewOutcomeCounts::default(),
        by_workflow_step: vec![],
        by_policy: vec![],
        by_agent: vec![],
        by_run_kind: vec![],
        top_reasons: vec![],
    }
}
