use std::collections::HashMap;

use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use tl_core::{
    HumanReviewAnalyticsResponse, HumanReviewAnalyticsSummary, HumanReviewOutcome,
    HumanReviewOutcomeCounts, HumanReviewPolicyRow, HumanReviewReasonRow,
    HumanReviewWorkflowStepRow,
};
use uuid::Uuid;

use super::analytics::{
    count_outcome, group_row, is_human_intervention, payload_string, percentage, policy_ids,
    sort_named_rows, sort_policy_rows, sort_reason_rows, workflow_step, GroupAccumulator,
    PolicyAccumulator, WorkflowAccumulator,
};
use super::HumanReviewRepo;
use crate::schema::{run_events, runs, traces};
use crate::StorageError;

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

impl HumanReviewRepo {
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
            let automated = matches!(
                decision.as_str(),
                "deny" | "transform" | "require_approval" | "defer"
            );
            if automated {
                summary.automated_intervention_count += 1;
            }
            for policy_id in &policies {
                let entry = policy.entry(policy_id.clone()).or_default();
                if decision == "require_approval" {
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
                    .map(|(group, counts)| group_row(group, counts))
                    .collect(),
                |row| &row.group,
            ),
            by_run_kind: sort_named_rows(
                by_run_kind
                    .into_iter()
                    .map(|(group, counts)| group_row(group, counts))
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
}
