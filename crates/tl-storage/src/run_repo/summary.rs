use tl_core::{RunEventSummary, RunSummary};

use crate::{
    models::{RunEventRecord, RunRecord},
    StorageError,
};

use super::text::{parse_evaluation_eligibility, parse_event_kind, parse_kind, parse_status};

#[derive(Default)]
pub(super) struct RunStats {
    pub(super) trace_count: i64,
    pub(super) blocked_count: i64,
    pub(super) rewritten_count: i64,
    pub(super) escalated_count: i64,
    pub(super) p95_latency_ms: Option<i32>,
}

pub(super) fn run_summary(record: RunRecord, stats: RunStats) -> Result<RunSummary, StorageError> {
    Ok(RunSummary {
        id: record.id.to_string(),
        workspace_id: record.workspace_id,
        environment_id: record.environment_id.clone(),
        environment: record.environment_id,
        agent_id: record.agent_id,
        kind: parse_kind(&record.kind)?,
        status: parse_status(&record.status)?,
        evaluation_eligibility: Some(parse_evaluation_eligibility(
            &record.evaluation_eligibility,
        )?),
        external_id: record.external_id,
        metadata: record.metadata,
        started_at: record.started_at.to_rfc3339(),
        ended_at: record.ended_at.map(|value| value.to_rfc3339()),
        created_at: record.created_at.to_rfc3339(),
        updated_at: record.updated_at.to_rfc3339(),
        trace_count: stats.trace_count,
        blocked_count: stats.blocked_count,
        rewritten_count: stats.rewritten_count,
        escalated_count: stats.escalated_count,
        p95_latency_ms: stats.p95_latency_ms,
    })
}

pub(super) fn event_summary(record: RunEventRecord) -> Result<RunEventSummary, StorageError> {
    Ok(RunEventSummary {
        id: record.id.to_string(),
        workspace_id: record.workspace_id,
        run_id: record.run_id.to_string(),
        agent_id: record.agent_id,
        sequence: record.sequence,
        kind: parse_event_kind(&record.kind)?,
        label: record.label,
        input_summary: record.input_summary,
        output_summary: record.output_summary,
        metadata: record.metadata,
        occurred_at: record.occurred_at.to_rfc3339(),
        created_at: record.created_at.to_rfc3339(),
    })
}

pub(super) fn p95(mut latencies: Vec<i32>) -> Option<i32> {
    if latencies.is_empty() {
        return None;
    }
    latencies.sort_unstable();
    let index = ((latencies.len() as f64) * 0.95).ceil() as usize;
    latencies.get(index.saturating_sub(1)).copied()
}
