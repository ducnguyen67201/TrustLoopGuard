use std::collections::HashMap;

use async_trait::async_trait;
use tl_core::{
    BoundaryConfidence, CreateRunEventRequest, CreateRunRequest, EvaluationJobStatus,
    FinalizeRunRequest, FinalizeRunResponse, RunCaptureStatus, RunEventSummary,
    RunFinalizationSummary, RunStatus, RunSummary, TraceSummary, UpdateRunRequest,
};
use tokio::sync::RwLock;

use super::validation::{
    clean_optional, normalize_metadata, validate_create_run, validate_create_run_event,
    validate_update_run,
};
use super::{RunListFilter, RunStore, RunStoreError};

#[derive(Debug, Default)]
pub struct MemoryRunStore {
    runs: RwLock<HashMap<String, RunSummary>>,
    events: RwLock<HashMap<String, Vec<RunEventSummary>>>,
    latencies: RwLock<HashMap<String, Vec<i32>>>,
    finalizations: RwLock<HashMap<String, RunFinalizationSummary>>,
}

impl MemoryRunStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl RunStore for MemoryRunStore {
    async fn create(
        &self,
        workspace_id: &str,
        environment_id: &str,
        input: CreateRunRequest,
    ) -> Result<RunSummary, RunStoreError> {
        validate_create_run(&input)?;
        let now = chrono::Utc::now().to_rfc3339();
        let id = uuid::Uuid::now_v7().to_string();
        let run = RunSummary {
            id: id.clone(),
            workspace_id: workspace_id.to_string(),
            environment_id: environment_id.to_string(),
            environment: environment_id.to_string(),
            agent_id: input.agent_id.trim().to_string(),
            kind: input.kind,
            status: input.status.unwrap_or(RunStatus::Running),
            evaluation_eligibility: Some(tl_core::RunEvaluationEligibility::Eligible),
            external_id: clean_optional(input.external_id),
            metadata: normalize_metadata(input.metadata),
            started_at: now.clone(),
            ended_at: None,
            created_at: now.clone(),
            updated_at: now,
            trace_count: 0,
            blocked_count: 0,
            rewritten_count: 0,
            escalated_count: 0,
            p95_latency_ms: None,
        };
        self.runs.write().await.insert(id, run.clone());
        Ok(run)
    }

    async fn list(
        &self,
        workspace_id: &str,
        environment_id: &str,
        filter: RunListFilter,
    ) -> Result<Vec<RunSummary>, RunStoreError> {
        let mut rows: Vec<_> = self
            .runs
            .read()
            .await
            .values()
            .filter(|run| run.workspace_id == workspace_id)
            .filter(|run| run.environment_id == environment_id)
            .filter(|run| {
                filter
                    .agent_id
                    .as_deref()
                    .map_or(true, |id| run.agent_id == id)
            })
            .filter(|run| filter.status.map_or(true, |status| run.status == status))
            .filter(|run| filter.kind.map_or(true, |kind| run.kind == kind))
            .filter(|run| {
                filter.external_id.as_deref().map_or(true, |external_id| {
                    run.external_id.as_deref() == Some(external_id)
                })
            })
            .cloned()
            .collect();
        rows.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        rows.truncate(filter.limit.clamp(1, 100));
        Ok(rows)
    }

    async fn get(
        &self,
        workspace_id: &str,
        environment_id: &str,
        run_id: &str,
    ) -> Result<RunSummary, RunStoreError> {
        self.runs
            .read()
            .await
            .get(run_id)
            .filter(|run| run.workspace_id == workspace_id && run.environment_id == environment_id)
            .cloned()
            .ok_or(RunStoreError::NotFound)
    }

    async fn update(
        &self,
        workspace_id: &str,
        environment_id: &str,
        run_id: &str,
        input: UpdateRunRequest,
    ) -> Result<RunSummary, RunStoreError> {
        validate_update_run(&input)?;
        let mut runs = self.runs.write().await;
        let run = runs
            .get_mut(run_id)
            .filter(|run| run.workspace_id == workspace_id && run.environment_id == environment_id)
            .ok_or(RunStoreError::NotFound)?;
        if let Some(status) = input.status {
            run.status = status;
            if status.is_terminal() && run.ended_at.is_none() {
                run.ended_at = Some(chrono::Utc::now().to_rfc3339());
            }
        }
        if let Some(metadata) = input.metadata {
            run.metadata = normalize_metadata(metadata);
        }
        if let Some(ended_at) = input.ended_at {
            run.ended_at = Some(ended_at);
        }
        run.updated_at = chrono::Utc::now().to_rfc3339();
        Ok(run.clone())
    }

    async fn finalize(
        &self,
        workspace_id: &str,
        environment_id: &str,
        run_id: &str,
        input: FinalizeRunRequest,
        capture_wait_ms: u64,
    ) -> Result<FinalizeRunResponse, RunStoreError> {
        if !input.status.is_terminal() {
            return Err(RunStoreError::Validation(
                "final run status must be terminal".into(),
            ));
        }
        if let Some(existing) = self.finalizations.read().await.get(run_id).cloned() {
            let run = self.get(workspace_id, environment_id, run_id).await?;
            if run.status != input.status
                || existing.boundary_source != input.boundary_source
                || existing.expected_flush_id != input.expected_flush_id
                || input
                    .ended_at
                    .as_ref()
                    .is_some_and(|ended_at| run.ended_at.as_deref() != Some(ended_at.as_str()))
            {
                return Err(RunStoreError::Conflict);
            }
            return Ok(FinalizeRunResponse {
                run,
                finalization: existing,
                evaluation_status: EvaluationJobStatus::WaitingCapture,
            });
        }
        if let Some(expected) = input.last_event_sequence {
            let events = self.events.read().await;
            let actual = events
                .get(run_id)
                .and_then(|events| events.iter().map(|event| event.sequence).max())
                .unwrap_or(0);
            if actual != expected {
                return Err(RunStoreError::Conflict);
            }
        }
        let now = chrono::Utc::now();
        let ended_at = input.ended_at.clone().unwrap_or_else(|| now.to_rfc3339());
        {
            let mut runs = self.runs.write().await;
            let run = runs
                .get_mut(run_id)
                .filter(|run| {
                    run.workspace_id == workspace_id && run.environment_id == environment_id
                })
                .ok_or(RunStoreError::NotFound)?;
            run.status = input.status;
            run.ended_at = Some(ended_at);
            run.updated_at = now.to_rfc3339();
        }
        let confidence = match input.boundary_source {
            tl_core::RunBoundarySource::ExplicitSdk | tl_core::RunBoundarySource::Admin => {
                BoundaryConfidence::Authoritative
            }
            tl_core::RunBoundarySource::FrameworkAdapter
            | tl_core::RunBoundarySource::OtelSessionEnd
            | tl_core::RunBoundarySource::LegacySdk => BoundaryConfidence::Strong,
            _ => BoundaryConfidence::Inferred,
        };
        let finalization = RunFinalizationSummary {
            finalized_at: now.to_rfc3339(),
            boundary_source: input.boundary_source,
            boundary_confidence: confidence,
            capture_status: RunCaptureStatus::Waiting,
            capture_deadline: (now
                + chrono::Duration::milliseconds(
                    i64::try_from(capture_wait_ms.min(3_600_000)).unwrap_or(3_600_000),
                ))
            .to_rfc3339(),
            expected_flush_id: input.expected_flush_id,
        };
        self.finalizations
            .write()
            .await
            .insert(run_id.to_string(), finalization.clone());
        Ok(FinalizeRunResponse {
            run: self.get(workspace_id, environment_id, run_id).await?,
            finalization,
            evaluation_status: EvaluationJobStatus::WaitingCapture,
        })
    }

    async fn finalization(
        &self,
        workspace_id: &str,
        environment_id: &str,
        run_id: &str,
    ) -> Result<Option<RunFinalizationSummary>, RunStoreError> {
        self.get(workspace_id, environment_id, run_id).await?;
        Ok(self.finalizations.read().await.get(run_id).cloned())
    }

    async fn traces(
        &self,
        workspace_id: &str,
        environment_id: &str,
        run_id: &str,
        _limit: usize,
    ) -> Result<Vec<TraceSummary>, RunStoreError> {
        self.get(workspace_id, environment_id, run_id).await?;
        Ok(vec![])
    }

    async fn create_event(
        &self,
        workspace_id: &str,
        environment_id: &str,
        run_id: &str,
        input: CreateRunEventRequest,
    ) -> Result<RunEventSummary, RunStoreError> {
        validate_create_run_event(&input)?;
        let run = self.get(workspace_id, environment_id, run_id).await?;
        if self.finalizations.read().await.contains_key(run_id) {
            return Err(RunStoreError::Conflict);
        }
        let mut events = self.events.write().await;
        let run_events = events.entry(run_id.to_string()).or_default();
        let sequence = input.sequence.unwrap_or_else(|| {
            run_events
                .last()
                .map(|event| event.sequence + 1)
                .unwrap_or(1)
        });
        let now = chrono::Utc::now().to_rfc3339();
        let event = RunEventSummary {
            id: uuid::Uuid::now_v7().to_string(),
            workspace_id: workspace_id.to_string(),
            run_id: run_id.to_string(),
            agent_id: input
                .agent_id
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or(&run.agent_id)
                .to_string(),
            sequence,
            kind: input.kind,
            label: clean_optional(input.label),
            input_summary: clean_optional(input.input_summary),
            output_summary: clean_optional(input.output_summary),
            metadata: normalize_metadata(input.metadata),
            occurred_at: input.occurred_at.unwrap_or_else(|| now.clone()),
            created_at: now,
        };
        run_events.push(event.clone());
        run_events.sort_by_key(|event| event.sequence);
        Ok(event)
    }

    async fn events(
        &self,
        workspace_id: &str,
        environment_id: &str,
        run_id: &str,
        limit: usize,
    ) -> Result<Vec<RunEventSummary>, RunStoreError> {
        self.get(workspace_id, environment_id, run_id).await?;
        let events = self.events.read().await;
        let mut rows = events.get(run_id).cloned().unwrap_or_default();
        rows.retain(|event| event.workspace_id == workspace_id);
        rows.truncate(limit.clamp(1, 200));
        Ok(rows)
    }

    async fn event_belongs_to_run(
        &self,
        workspace_id: &str,
        environment_id: &str,
        run_id: &str,
        run_event_id: &str,
    ) -> Result<(), RunStoreError> {
        self.get(workspace_id, environment_id, run_id).await?;
        let events = self.events.read().await;
        events
            .get(run_id)
            .and_then(|rows| {
                rows.iter()
                    .find(|event| event.workspace_id == workspace_id && event.id == run_event_id)
            })
            .map(|_| ())
            .ok_or(RunStoreError::NotFound)
    }

    async fn record_check(
        &self,
        workspace_id: &str,
        environment_id: &str,
        run_id: &str,
        effect: &str,
        elapsed_ms: i32,
    ) -> Result<(), RunStoreError> {
        {
            let mut runs = self.runs.write().await;
            let run = runs
                .get_mut(run_id)
                .filter(|r| r.workspace_id == workspace_id && r.environment_id == environment_id)
                .ok_or(RunStoreError::NotFound)?;
            run.trace_count += 1;
            match effect {
                "deny" => run.blocked_count += 1,
                "transform" => run.rewritten_count += 1,
                "require_approval" | "defer" => run.escalated_count += 1,
                _ => {}
            }
        }

        let p95 = {
            let mut latencies = self.latencies.write().await;
            let values = latencies.entry(run_id.to_string()).or_default();
            values.push(elapsed_ms);
            p95_latency(values)
        };

        let mut runs = self.runs.write().await;
        if let Some(run) = runs
            .get_mut(run_id)
            .filter(|r| r.workspace_id == workspace_id)
            .filter(|r| r.environment_id == environment_id)
        {
            run.p95_latency_ms = p95;
        }

        Ok(())
    }
}

fn p95_latency(latencies: &[i32]) -> Option<i32> {
    if latencies.is_empty() {
        return None;
    }
    let mut sorted = latencies.to_vec();
    sorted.sort_unstable();
    let index = ((sorted.len() as f64) * 0.95).ceil() as usize;
    sorted.get(index.saturating_sub(1)).copied()
}
