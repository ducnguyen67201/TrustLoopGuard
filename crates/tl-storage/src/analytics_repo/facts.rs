use std::collections::HashMap;

use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use uuid::Uuid;

use crate::schema::{human_review_events, run_events, runs, traces};
use crate::StorageError;

use super::AnalyticsRepo;

#[derive(Clone)]
pub(super) struct AnalyticsFact {
    pub(super) environment_id: String,
    pub(super) decision: String,
    pub(super) elapsed_ms: i32,
    pub(super) agent_id: String,
    pub(super) run_kind: String,
    pub(super) run_status: String,
    pub(super) external_id: String,
    pub(super) workflow_step: String,
    pub(super) review_outcome: String,
    pub(super) policy_ids: Vec<String>,
}

impl AnalyticsRepo {
    pub(super) async fn facts(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<AnalyticsFact>, StorageError> {
        let mut conn = self.connection().await?;
        let trace_rows = traces::table
            .filter(traces::workspace_id.eq(workspace_id))
            .select((
                traces::trace_id,
                traces::run_id,
                traces::run_event_id,
                traces::environment_id,
                traces::decision,
                traces::elapsed_ms,
                traces::payload,
            ))
            .order(traces::created_at.desc())
            .limit(5_000)
            .load::<(
                Uuid,
                Option<Uuid>,
                Option<Uuid>,
                String,
                String,
                i32,
                serde_json::Value,
            )>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("analytics traces: {e}")))?;
        let run_rows = runs::table
            .filter(runs::workspace_id.eq(workspace_id))
            .select((
                runs::id,
                runs::agent_id,
                runs::kind,
                runs::status,
                runs::external_id,
            ))
            .load::<(Uuid, String, String, String, Option<String>)>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("analytics runs: {e}")))?;
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
            .map_err(|e| StorageError::Internal(format!("analytics run events: {e}")))?;
        let review_rows = human_review_events::table
            .filter(human_review_events::workspace_id.eq(workspace_id))
            .select((
                human_review_events::trace_id,
                human_review_events::outcome,
                human_review_events::created_at,
            ))
            .order(human_review_events::created_at.desc())
            .load::<(Uuid, String, DateTime<Utc>)>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("analytics reviews: {e}")))?;

        let runs_by_id = run_rows
            .into_iter()
            .map(|(id, agent_id, kind, status, external_id)| {
                (
                    id,
                    (
                        agent_id,
                        kind,
                        status,
                        external_id.unwrap_or_else(|| "none".into()),
                    ),
                )
            })
            .collect::<HashMap<_, _>>();
        let events_by_id = event_rows
            .into_iter()
            .map(|(id, kind, label, metadata)| (id, (kind, label, metadata)))
            .collect::<HashMap<_, _>>();
        let mut latest_reviews = HashMap::<Uuid, String>::new();
        for (trace_id, outcome, _) in review_rows {
            latest_reviews.entry(trace_id).or_insert(outcome);
        }

        Ok(trace_rows
            .into_iter()
            .map(
                |(
                    trace_id,
                    run_id,
                    run_event_id,
                    environment_id,
                    decision,
                    elapsed_ms,
                    payload,
                )| {
                    let run = run_id.and_then(|id| runs_by_id.get(&id));
                    let event = run_event_id.and_then(|id| events_by_id.get(&id));
                    AnalyticsFact {
                        environment_id,
                        decision,
                        elapsed_ms,
                        agent_id: run
                            .map(|row| row.0.clone())
                            .or_else(|| payload_string(&payload, "agent_id"))
                            .unwrap_or_else(|| "unknown".into()),
                        run_kind: run
                            .map(|row| row.1.clone())
                            .unwrap_or_else(|| "ungrouped".into()),
                        run_status: run
                            .map(|row| row.2.clone())
                            .unwrap_or_else(|| "unknown".into()),
                        external_id: run
                            .map(|row| row.3.clone())
                            .unwrap_or_else(|| "none".into()),
                        workflow_step: event
                            .and_then(|row| workflow_step(&row.0, row.1.as_deref(), &row.2))
                            .unwrap_or_else(|| "unlabeled".into()),
                        review_outcome: latest_reviews
                            .get(&trace_id)
                            .cloned()
                            .unwrap_or_else(|| "not_reviewed".into()),
                        policy_ids: policy_ids(&payload),
                    }
                },
            )
            .collect())
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
        .or_else(|| {
            let trimmed = event_kind.trim();
            (!trimmed.is_empty()).then(|| trimmed.to_string())
        })
}
