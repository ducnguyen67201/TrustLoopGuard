use std::collections::HashMap;

use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use tl_core::{
    CreateHumanReviewEventRequest, HumanReviewAnalyticsResponse, HumanReviewAnalyticsSummary,
    HumanReviewEvent, HumanReviewGroupRow, HumanReviewOutcome, HumanReviewOutcomeCounts,
    HumanReviewPolicyRow, HumanReviewReasonRow, HumanReviewWorkflowStepRow,
};
use uuid::Uuid;

use crate::models::{HumanReviewEventRecord, NewHumanReviewEvent};
use crate::postgres::{DbConnection, DbPool};
use crate::schema::{human_review_events, run_events, runs, traces};
use crate::StorageError;

#[derive(Clone)]
pub struct HumanReviewRepo {
    pool: DbPool,
}

impl HumanReviewRepo {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub async fn create_event(
        &self,
        workspace_id: &str,
        trace_id: &str,
        input: CreateHumanReviewEventRequest,
        reviewer_id: Option<String>,
    ) -> Result<HumanReviewEvent, StorageError> {
        validate_create_event(&input)?;
        let trace_uuid = parse_uuid("trace_id", trace_id)?;
        let mut conn = self.connection().await?;
        let trace = traces::table
            .filter(traces::workspace_id.eq(workspace_id))
            .filter(traces::trace_id.eq(trace_uuid))
            .select((traces::run_id, traces::run_event_id))
            .order(traces::created_at.desc())
            .first::<(Option<Uuid>, Option<Uuid>)>(&mut conn)
            .await
            .optional()
            .map_err(|e| StorageError::Internal(format!("review trace lookup: {e}")))?
            .ok_or(StorageError::NotFound)?;

        let id = Uuid::now_v7();
        let event = NewHumanReviewEvent {
            workspace_id: workspace_id.to_string(),
            id,
            trace_id: trace_uuid,
            run_id: trace.0,
            run_event_id: trace.1,
            outcome: outcome_text(input.outcome).to_string(),
            reviewer_id: reviewer_id.and_then(|value| non_empty_string(value.trim())),
            reason_codes: serde_json::json!(clean_reason_codes(input.reason_codes)),
            note: input.note.and_then(|value| non_empty_string(value.trim())),
            metadata: normalize_metadata(input.metadata),
        };
        diesel::insert_into(human_review_events::table)
            .values(&event)
            .execute(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("review event insert: {e}")))?;

        drop(conn);
        self.event(workspace_id, id).await
    }

    pub async fn list_events(
        &self,
        workspace_id: &str,
        trace_id: &str,
        limit: i64,
    ) -> Result<Vec<HumanReviewEvent>, StorageError> {
        let trace_uuid = parse_uuid("trace_id", trace_id)?;
        let mut conn = self.connection().await?;
        let records = human_review_events::table
            .filter(human_review_events::workspace_id.eq(workspace_id))
            .filter(human_review_events::trace_id.eq(trace_uuid))
            .select(HumanReviewEventRecord::as_select())
            .order(human_review_events::created_at.asc())
            .limit(limit.clamp(1, 100))
            .load::<HumanReviewEventRecord>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("review event list: {e}")))?;
        records.into_iter().map(event_summary).collect()
    }

    pub async fn latest_by_trace_ids(
        &self,
        workspace_id: &str,
        trace_ids: &[String],
    ) -> Result<HashMap<String, HumanReviewEvent>, StorageError> {
        if trace_ids.is_empty() {
            return Ok(HashMap::new());
        }
        let trace_uuids = trace_ids
            .iter()
            .map(|id| parse_uuid("trace_id", id))
            .collect::<Result<Vec<_>, _>>()?;
        let mut conn = self.connection().await?;
        let records = human_review_events::table
            .filter(human_review_events::workspace_id.eq(workspace_id))
            .filter(human_review_events::trace_id.eq_any(trace_uuids))
            .select(HumanReviewEventRecord::as_select())
            .order(human_review_events::created_at.desc())
            .load::<HumanReviewEventRecord>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("review latest: {e}")))?;

        let mut latest = HashMap::new();
        for record in records {
            let key = record.trace_id.to_string();
            latest.entry(key).or_insert(event_summary(record)?);
        }
        Ok(latest)
    }

    pub async fn analytics(
        &self,
        workspace_id: &str,
        filter: HumanReviewAnalyticsFilter,
    ) -> Result<HumanReviewAnalyticsResponse, StorageError> {
        let mut conn = self.connection().await?;
        let trace_rows = traces::table
            .filter(traces::workspace_id.eq(workspace_id))
            .select((
                traces::trace_id,
                traces::run_id,
                traces::run_event_id,
                traces::decision,
                traces::payload,
            ))
            .load::<(Uuid, Option<Uuid>, Option<Uuid>, String, serde_json::Value)>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("review analytics traces: {e}")))?;
        let run_rows = runs::table
            .filter(runs::workspace_id.eq(workspace_id))
            .select((runs::id, runs::agent_id, runs::kind))
            .load::<(Uuid, String, String)>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("review analytics runs: {e}")))?;
        let event_rows = run_events::table
            .filter(run_events::workspace_id.eq(workspace_id))
            .select((
                run_events::id,
                run_events::kind,
                run_events::label,
                run_events::metadata,
            ))
            .load::<(Uuid, String, Option<String>, serde_json::Value)>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("review analytics run events: {e}")))?;
        drop(conn);

        let trace_ids = trace_rows
            .iter()
            .map(|row| row.0.to_string())
            .collect::<Vec<_>>();
        let latest = self.latest_by_trace_ids(workspace_id, &trace_ids).await?;
        let runs_by_id = run_rows
            .into_iter()
            .map(|(id, agent_id, kind)| (id, (agent_id, kind)))
            .collect::<HashMap<_, _>>();
        let events_by_id = event_rows
            .into_iter()
            .map(|(id, kind, label, metadata)| (id, (kind, label, metadata)))
            .collect::<HashMap<_, _>>();

        let mut summary = HumanReviewAnalyticsSummary::default();
        let mut outcomes = HumanReviewOutcomeCounts::default();
        let mut workflow = HashMap::<String, WorkflowAccumulator>::new();
        let mut policy = HashMap::<String, PolicyAccumulator>::new();
        let mut by_agent = HashMap::<String, GroupAccumulator>::new();
        let mut by_run_kind = HashMap::<String, GroupAccumulator>::new();
        let mut reasons = HashMap::<String, i64>::new();

        for (trace_id, run_id, run_event_id, decision, payload) in trace_rows {
            let agent_id = run_id
                .and_then(|id| runs_by_id.get(&id))
                .map(|row| row.0.clone())
                .or_else(|| payload_string(&payload, "agent_id"))
                .unwrap_or_else(|| "unknown".to_string());
            let run_kind = run_id
                .and_then(|id| runs_by_id.get(&id))
                .map(|row| row.1.clone())
                .unwrap_or_else(|| "ungrouped".to_string());
            let workflow_step = run_event_id
                .and_then(|id| events_by_id.get(&id))
                .and_then(|row| workflow_step(&row.0, row.1.as_deref(), &row.2))
                .unwrap_or_else(|| "unlabeled".to_string());
            let policies = policy_ids(&payload);
            let latest_event = latest.get(&trace_id.to_string());

            if !filter.matches(&agent_id, &run_kind, &workflow_step, &policies) {
                continue;
            }

            summary.trace_count += 1;
            let automated = matches!(decision.as_str(), "block" | "rewrite" | "escalate");
            if automated {
                summary.automated_intervention_count += 1;
            }
            for policy_id in &policies {
                let entry = policy.entry(policy_id.clone()).or_default();
                if decision == "escalate" {
                    entry.escalation_count += 1;
                }
            }

            if let Some(event) = latest_event {
                summary.human_review_count += 1;
                count_outcome(&mut outcomes, event.outcome);
                let is_human_intervention = is_human_intervention(event.outcome);
                if is_human_intervention {
                    summary.human_intervention_count += 1;
                }
                let workflow_entry = workflow.entry(workflow_step.clone()).or_default();
                workflow_entry.human_review_count += 1;
                match event.outcome {
                    HumanReviewOutcome::Corrected => workflow_entry.corrected_count += 1,
                    HumanReviewOutcome::Rejected => workflow_entry.rejected_count += 1,
                    HumanReviewOutcome::FalsePositive => {
                        workflow_entry.false_positive_count += 1;
                    }
                    _ => {}
                }
                for policy_id in &policies {
                    let entry = policy.entry(policy_id.clone()).or_default();
                    match event.outcome {
                        HumanReviewOutcome::Corrected => entry.corrected_count += 1,
                        HumanReviewOutcome::FalsePositive => entry.false_positive_count += 1,
                        _ => {}
                    }
                }
                let agent_entry = by_agent.entry(agent_id.clone()).or_default();
                agent_entry.human_review_count += 1;
                if is_human_intervention {
                    agent_entry.human_intervention_count += 1;
                }
                let kind_entry = by_run_kind.entry(run_kind.clone()).or_default();
                kind_entry.human_review_count += 1;
                if is_human_intervention {
                    kind_entry.human_intervention_count += 1;
                }
                for reason in &event.reason_codes {
                    *reasons.entry(reason.clone()).or_default() += 1;
                }
            }
        }

        summary.human_intervention_rate =
            percentage(summary.human_intervention_count, summary.trace_count);
        summary.false_positive_rate =
            percentage(outcomes.false_positive_count, summary.trace_count);

        Ok(HumanReviewAnalyticsResponse {
            summary,
            outcomes,
            by_workflow_step: sort_named_rows(
                workflow
                    .into_iter()
                    .map(|(workflow_step, counts)| HumanReviewWorkflowStepRow {
                        workflow_step,
                        human_review_count: counts.human_review_count,
                        corrected_count: counts.corrected_count,
                        rejected_count: counts.rejected_count,
                        false_positive_count: counts.false_positive_count,
                    })
                    .collect(),
                |row| &row.workflow_step,
            ),
            by_policy: sort_policy_rows(
                policy
                    .into_iter()
                    .filter(|(_, counts)| {
                        counts.escalation_count
                            + counts.corrected_count
                            + counts.false_positive_count
                            > 0
                    })
                    .map(|(policy_id, counts)| HumanReviewPolicyRow {
                        policy_id,
                        escalation_count: counts.escalation_count,
                        corrected_count: counts.corrected_count,
                        false_positive_count: counts.false_positive_count,
                    })
                    .collect(),
            ),
            by_agent: sort_named_rows(
                by_agent
                    .into_iter()
                    .map(|(group, counts)| HumanReviewGroupRow {
                        group,
                        human_review_count: counts.human_review_count,
                        human_intervention_count: counts.human_intervention_count,
                    })
                    .collect(),
                |row| &row.group,
            ),
            by_run_kind: sort_named_rows(
                by_run_kind
                    .into_iter()
                    .map(|(group, counts)| HumanReviewGroupRow {
                        group,
                        human_review_count: counts.human_review_count,
                        human_intervention_count: counts.human_intervention_count,
                    })
                    .collect(),
                |row| &row.group,
            ),
            top_reasons: sort_reason_rows(
                reasons
                    .into_iter()
                    .map(|(reason_code, count)| HumanReviewReasonRow { reason_code, count })
                    .collect(),
            ),
        })
    }

    async fn event(&self, workspace_id: &str, id: Uuid) -> Result<HumanReviewEvent, StorageError> {
        let mut conn = self.connection().await?;
        let record = human_review_events::table
            .filter(human_review_events::workspace_id.eq(workspace_id))
            .filter(human_review_events::id.eq(id))
            .select(HumanReviewEventRecord::as_select())
            .first::<HumanReviewEventRecord>(&mut conn)
            .await
            .optional()
            .map_err(|e| StorageError::Internal(format!("review event get: {e}")))?
            .ok_or(StorageError::NotFound)?;
        event_summary(record)
    }

    async fn connection(&self) -> Result<DbConnection<'_>, StorageError> {
        self.pool
            .get()
            .await
            .map_err(|e| StorageError::Internal(format!("db pool: {e}")))
    }
}

#[derive(Debug, Clone, Default)]
pub struct HumanReviewAnalyticsFilter {
    pub agent_id: Option<String>,
    pub policy_id: Option<String>,
    pub run_kind: Option<String>,
    pub workflow_step: Option<String>,
}

impl HumanReviewAnalyticsFilter {
    fn matches(
        &self,
        agent_id: &str,
        run_kind: &str,
        workflow_step: &str,
        policy_ids: &[String],
    ) -> bool {
        self.agent_id
            .as_deref()
            .map_or(true, |value| value == agent_id)
            && self
                .run_kind
                .as_deref()
                .map_or(true, |value| value == run_kind)
            && self
                .workflow_step
                .as_deref()
                .map_or(true, |value| value == workflow_step)
            && self
                .policy_id
                .as_deref()
                .map_or(true, |value| policy_ids.iter().any(|id| id == value))
    }
}

#[derive(Default)]
struct WorkflowAccumulator {
    human_review_count: i64,
    corrected_count: i64,
    rejected_count: i64,
    false_positive_count: i64,
}

#[derive(Default)]
struct PolicyAccumulator {
    escalation_count: i64,
    corrected_count: i64,
    false_positive_count: i64,
}

#[derive(Default)]
struct GroupAccumulator {
    human_review_count: i64,
    human_intervention_count: i64,
}

fn validate_create_event(input: &CreateHumanReviewEventRequest) -> Result<(), StorageError> {
    validate_metadata(&input.metadata)?;
    for code in &input.reason_codes {
        if code.trim().is_empty() {
            return Err(StorageError::Internal(
                "reason_codes must not contain empty values".into(),
            ));
        }
    }
    Ok(())
}

fn validate_metadata(value: &serde_json::Value) -> Result<(), StorageError> {
    if value.is_null() || value.is_object() {
        Ok(())
    } else {
        Err(StorageError::Internal(
            "metadata must be a JSON object".into(),
        ))
    }
}

fn event_summary(record: HumanReviewEventRecord) -> Result<HumanReviewEvent, StorageError> {
    Ok(HumanReviewEvent {
        id: record.id.to_string(),
        workspace_id: record.workspace_id,
        trace_id: record.trace_id.to_string(),
        run_id: record.run_id.map(|id| id.to_string()),
        run_event_id: record.run_event_id.map(|id| id.to_string()),
        outcome: parse_outcome(&record.outcome)?,
        reason_codes: parse_reason_codes(record.reason_codes)?,
        note: record.note,
        reviewer_id: record.reviewer_id,
        metadata: record.metadata,
        created_at: record.created_at.to_rfc3339(),
    })
}

fn parse_reason_codes(value: serde_json::Value) -> Result<Vec<String>, StorageError> {
    match value {
        serde_json::Value::Array(values) => values
            .into_iter()
            .map(|value| match value {
                serde_json::Value::String(code) => Ok(code),
                _ => Err(StorageError::Internal(
                    "reason_codes contains a non-string value".into(),
                )),
            })
            .collect(),
        _ => Err(StorageError::Internal(
            "reason_codes must be a JSON array".into(),
        )),
    }
}

fn clean_reason_codes(values: Vec<String>) -> Vec<String> {
    values
        .into_iter()
        .filter_map(|value| non_empty_string(value.trim()))
        .collect()
}

fn normalize_metadata(value: serde_json::Value) -> serde_json::Value {
    if value.is_null() {
        serde_json::json!({})
    } else {
        value
    }
}

fn non_empty_string(value: &str) -> Option<String> {
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn parse_uuid(label: &str, value: &str) -> Result<Uuid, StorageError> {
    Uuid::parse_str(value).map_err(|e| StorageError::Internal(format!("{label} parse: {e}")))
}

fn outcome_text(outcome: HumanReviewOutcome) -> &'static str {
    match outcome {
        HumanReviewOutcome::Accepted => "accepted",
        HumanReviewOutcome::Corrected => "corrected",
        HumanReviewOutcome::Rejected => "rejected",
        HumanReviewOutcome::FalsePositive => "false_positive",
        HumanReviewOutcome::MissedIssue => "missed_issue",
        HumanReviewOutcome::Ignored => "ignored",
    }
}

fn parse_outcome(value: &str) -> Result<HumanReviewOutcome, StorageError> {
    match value {
        "accepted" => Ok(HumanReviewOutcome::Accepted),
        "corrected" => Ok(HumanReviewOutcome::Corrected),
        "rejected" => Ok(HumanReviewOutcome::Rejected),
        "false_positive" => Ok(HumanReviewOutcome::FalsePositive),
        "missed_issue" => Ok(HumanReviewOutcome::MissedIssue),
        "ignored" => Ok(HumanReviewOutcome::Ignored),
        other => Err(StorageError::Internal(format!(
            "unknown human review outcome: {other}"
        ))),
    }
}

fn is_human_intervention(outcome: HumanReviewOutcome) -> bool {
    matches!(
        outcome,
        HumanReviewOutcome::Corrected
            | HumanReviewOutcome::Rejected
            | HumanReviewOutcome::MissedIssue
    )
}

fn count_outcome(counts: &mut HumanReviewOutcomeCounts, outcome: HumanReviewOutcome) {
    match outcome {
        HumanReviewOutcome::Accepted => counts.accepted_count += 1,
        HumanReviewOutcome::Corrected => counts.corrected_count += 1,
        HumanReviewOutcome::Rejected => counts.rejected_count += 1,
        HumanReviewOutcome::FalsePositive => counts.false_positive_count += 1,
        HumanReviewOutcome::MissedIssue => counts.missed_issue_count += 1,
        HumanReviewOutcome::Ignored => counts.ignored_count += 1,
    }
}

fn percentage(numerator: i64, denominator: i64) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        ((numerator as f64 / denominator as f64) * 10_000.0).round() / 100.0
    }
}

fn payload_string(payload: &serde_json::Value, key: &str) -> Option<String> {
    payload
        .get(key)
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn policy_ids(payload: &serde_json::Value) -> Vec<String> {
    let ids = payload
        .get("triggered_policies")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|policy| payload_string(policy, "id"))
        .collect::<Vec<_>>();
    if ids.is_empty() {
        vec!["baseline".to_string()]
    } else {
        ids
    }
}

fn workflow_step(
    event_kind: &str,
    event_label: Option<&str>,
    metadata: &serde_json::Value,
) -> Option<String> {
    metadata
        .get("workflow_step")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| {
            event_label
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
        })
        .or_else(|| non_empty_string(event_kind))
}

fn sort_named_rows<T, F>(mut rows: Vec<T>, name: F) -> Vec<T>
where
    F: Fn(&T) -> &str,
{
    rows.sort_by(|a, b| name(a).cmp(name(b)));
    rows
}

fn sort_reason_rows(mut rows: Vec<HumanReviewReasonRow>) -> Vec<HumanReviewReasonRow> {
    rows.sort_by(|a, b| {
        b.count
            .cmp(&a.count)
            .then_with(|| a.reason_code.cmp(&b.reason_code))
    });
    rows
}

fn sort_policy_rows(mut rows: Vec<HumanReviewPolicyRow>) -> Vec<HumanReviewPolicyRow> {
    rows.sort_by(|a, b| {
        let b_total = b.escalation_count + b.corrected_count + b.false_positive_count;
        let a_total = a.escalation_count + a.corrected_count + a.false_positive_count;
        b_total
            .cmp(&a_total)
            .then_with(|| a.policy_id.cmp(&b.policy_id))
    });
    rows
}

impl std::fmt::Debug for HumanReviewRepo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HumanReviewRepo").finish_non_exhaustive()
    }
}
