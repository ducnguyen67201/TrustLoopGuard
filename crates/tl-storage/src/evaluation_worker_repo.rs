//! Capture barrier, immutable snapshot, and restart-safe evaluation job lease
//! operations. Protocol and evaluator logic remain outside storage.

use std::collections::{BTreeMap, HashMap};

use chrono::{Duration, Utc};
use diesel::dsl::{max, now};
use diesel::prelude::*;
use diesel_async::{AsyncConnection, RunQueryDsl};
use serde::{Deserialize, Serialize};
use tl_core::{EvaluationJobStatus, EvaluationJobSummary};
use uuid::Uuid;

use crate::evaluation_repo::EvaluationRepo;
use crate::models::{
    EvaluationJobRecord, RunEvaluationPolicyManifestRecord, RunEventRecord, RunParticipantRecord,
    RunRecord, RunSnapshotRecord, RunSpanRecord,
};
use crate::schema::{
    agent_evaluation_profiles, evaluation_findings, evaluation_jobs, evaluation_results,
    otel_flush_receipts, run_evaluation_policy_manifest, run_events, run_participants,
    run_snapshots, run_spans, runs, traces,
};
use crate::StorageError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureAdvanceResult {
    Waiting,
    SnapshotCreated { jobs_created: usize },
    AlreadyClosed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrozenEvaluationPolicy {
    pub policy_id: String,
    pub policy_version: i32,
    pub policy_hash: String,
    pub policy_yaml: String,
    pub weight: u32,
    pub critical: bool,
    pub evidence_requirements: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct EvaluationJobWork {
    pub workspace_id: String,
    pub environment_id: String,
    pub job_id: Uuid,
    pub run_id: Uuid,
    pub agent_id: String,
    pub snapshot_hash: String,
    pub manifest_hash: String,
    pub capture_status: String,
    pub snapshot: serde_json::Value,
    pub manifest: Vec<FrozenEvaluationPolicy>,
    pub lease_owner: String,
    pub attempt: i32,
}

#[derive(Debug, Clone)]
pub struct PersistEvaluationFinding {
    pub policy_id: String,
    pub policy_version: i32,
    pub severity: String,
    pub critical: bool,
    pub status: String,
    pub score_bps: Option<i32>,
    pub reason: String,
    pub evidence: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct PersistEvaluationResult {
    pub verdict: String,
    pub score_bps: Option<i32>,
    pub llm_audit: Option<serde_json::Value>,
    pub findings: Vec<PersistEvaluationFinding>,
}

impl EvaluationRepo {
    pub async fn list_evaluation_jobs(
        &self,
        workspace_id: &str,
        environment_id: &str,
        run_id: Uuid,
    ) -> Result<Vec<EvaluationJobSummary>, StorageError> {
        let mut conn = self.connection().await?;
        evaluation_jobs::table
            .filter(evaluation_jobs::workspace_id.eq(workspace_id))
            .filter(evaluation_jobs::environment_id.eq(environment_id))
            .filter(evaluation_jobs::run_id.eq(run_id))
            .select(EvaluationJobRecord::as_select())
            .order(evaluation_jobs::created_at.asc())
            .load::<EvaluationJobRecord>(&mut conn)
            .await?
            .into_iter()
            .map(|record| {
                Ok(EvaluationJobSummary {
                    id: record.id.to_string(),
                    run_id: record.run_id.to_string(),
                    agent_id: record.agent_id,
                    status: parse_job_status(&record.status)?,
                    attempts: record.attempts,
                    error: record.error,
                    updated_at: record.updated_at.to_rfc3339(),
                })
            })
            .collect()
    }

    pub async fn advance_due_captures(&self, limit: i64) -> Result<usize, StorageError> {
        let mut conn = self.connection().await?;
        let candidates = runs::table
            .filter(runs::finalized_at.is_not_null())
            .filter(runs::capture_status.eq("waiting"))
            .select((runs::workspace_id, runs::environment_id, runs::id))
            .order(runs::finalized_at.asc())
            .limit(limit.clamp(1, 100))
            .load::<(String, String, Uuid)>(&mut conn)
            .await?;
        drop(conn);
        let mut advanced = 0;
        for (workspace_id, environment_id, run_id) in candidates {
            if matches!(
                self.try_create_snapshot(&workspace_id, &environment_id, run_id)
                    .await?,
                CaptureAdvanceResult::SnapshotCreated { .. }
            ) {
                advanced += 1;
            }
        }
        Ok(advanced)
    }

    pub async fn try_create_snapshot(
        &self,
        workspace_id: &str,
        environment_id: &str,
        run_id: Uuid,
    ) -> Result<CaptureAdvanceResult, StorageError> {
        let mut conn = self.connection().await?;
        conn.transaction::<CaptureAdvanceResult, StorageError, _>(async |conn| {
            let run = diesel::update(
                runs::table
                    .filter(runs::workspace_id.eq(workspace_id))
                    .filter(runs::environment_id.eq(environment_id))
                    .filter(runs::id.eq(run_id)),
            )
            .set(runs::updated_at.eq(runs::updated_at))
            .returning(RunRecord::as_returning())
            .get_result::<RunRecord>(conn)
            .await
            .optional()?
            .ok_or(StorageError::NotFound)?;
            if run.capture_status != "waiting" {
                return Ok(CaptureAdvanceResult::AlreadyClosed);
            }
            let finalized_at = run.finalized_at.ok_or_else(|| {
                StorageError::Internal("waiting capture has no finalization".into())
            })?;
            let deadline = run
                .capture_deadline
                .ok_or_else(|| StorageError::Internal("waiting capture has no deadline".into()))?;
            let participants = run_participants::table
                .filter(run_participants::workspace_id.eq(workspace_id))
                .filter(run_participants::run_id.eq(run_id))
                .select(RunParticipantRecord::as_select())
                .order(run_participants::agent_id.asc())
                .load::<RunParticipantRecord>(conn)
                .await?;
            let evaluation_participants = match run.reevaluation_agent_ids.as_ref() {
                Some(requested) => participants
                    .iter()
                    .filter(|participant| requested.contains(&participant.agent_id))
                    .cloned()
                    .collect::<Vec<_>>(),
                None => participants.clone(),
            };
            let agent_ids = participants
                .iter()
                .map(|participant| participant.agent_id.clone())
                .collect::<Vec<_>>();
            let profile_rows = if agent_ids.is_empty() {
                Vec::new()
            } else {
                agent_evaluation_profiles::table
                    .filter(agent_evaluation_profiles::workspace_id.eq(workspace_id))
                    .filter(agent_evaluation_profiles::environment_id.eq(environment_id))
                    .filter(agent_evaluation_profiles::agent_id.eq_any(&agent_ids))
                    .filter(agent_evaluation_profiles::enabled.eq(true))
                    .select((
                        agent_evaluation_profiles::agent_id,
                        agent_evaluation_profiles::quiet_period_ms,
                        agent_evaluation_profiles::on_incomplete,
                        agent_evaluation_profiles::content_mode,
                    ))
                    .load::<(String, i64, String, String)>(conn)
                    .await?
            };
            let quiet_period_ms = if agent_ids.is_empty() {
                2_000
            } else {
                profile_rows
                    .iter()
                    .map(|(_, quiet_period_ms, _, _)| *quiet_period_ms)
                    .max()
                    .unwrap_or(2_000)
            };
            let agent_on_incomplete = profile_rows
                .iter()
                .map(|(agent_id, _, behavior, _)| (agent_id.clone(), behavior.clone()))
                .collect::<BTreeMap<_, _>>();
            let agent_content_modes = profile_rows
                .iter()
                .map(|(agent_id, _, _, mode)| (agent_id.clone(), mode.clone()))
                .collect::<BTreeMap<_, _>>();
            let receipt = match run.expected_flush_id.as_deref() {
                Some(flush_id) => otel_flush_receipts::table
                    .filter(otel_flush_receipts::workspace_id.eq(workspace_id))
                    .filter(otel_flush_receipts::environment_id.eq(environment_id))
                    .filter(otel_flush_receipts::run_id.eq(run_id))
                    .filter(otel_flush_receipts::flush_id.eq(flush_id))
                    .select((
                        otel_flush_receipts::accepted_at,
                        otel_flush_receipts::rejected_span_count,
                    ))
                    .first::<(chrono::DateTime<Utc>, i32)>(conn)
                    .await
                    .optional()?,
                None => None,
            };
            let current = Utc::now();
            let receipt_ready = receipt
                .as_ref()
                .is_some_and(|(accepted_at, _)| *accepted_at <= deadline);
            let receipt_rejected_spans = receipt.as_ref().map_or(0, |(_, rejected)| *rejected);
            let latest_evidence = run.last_evidence_at.unwrap_or(finalized_at);
            let quiet_ready = run.expected_flush_id.is_none()
                && current - latest_evidence >= Duration::milliseconds(quiet_period_ms);
            let deadline_reached = current >= deadline;
            if !receipt_ready && !quiet_ready && !deadline_reached {
                return Ok(CaptureAdvanceResult::Waiting);
            }

            let barrier_incomplete = deadline_reached && !receipt_ready && !quiet_ready
                || run.dropped_trace_count > 0
                || receipt_rejected_spans > 0;
            let event_rows = run_events::table
                .filter(run_events::workspace_id.eq(workspace_id))
                .filter(run_events::run_id.eq(run_id))
                .select(RunEventRecord::as_select())
                .order((run_events::sequence.asc(), run_events::id.asc()))
                .load::<RunEventRecord>(conn)
                .await?;
            let trace_rows = traces::table
                .filter(traces::workspace_id.eq(workspace_id))
                .filter(traces::run_id.eq(Some(run_id)))
                .filter(traces::created_at.le(current))
                .select((
                    traces::trace_id,
                    traces::agent_id,
                    traces::decision,
                    traces::elapsed_ms,
                    traces::payload,
                    traces::created_at,
                    traces::late_evidence,
                ))
                .order((traces::created_at.asc(), traces::trace_id.asc()))
                .load::<(
                    Uuid,
                    Option<String>,
                    String,
                    i32,
                    serde_json::Value,
                    chrono::DateTime<Utc>,
                    bool,
                )>(conn)
                .await?;
            let span_rows = run_spans::table
                .filter(run_spans::workspace_id.eq(workspace_id))
                .filter(run_spans::run_id.eq(run_id))
                .filter(run_spans::ingested_at.le(current))
                .select(RunSpanRecord::as_select())
                .order((run_spans::started_at.asc(), run_spans::otel_span_id.asc()))
                .load::<RunSpanRecord>(conn)
                .await?;
            let manifest = run_evaluation_policy_manifest::table
                .filter(run_evaluation_policy_manifest::workspace_id.eq(workspace_id))
                .filter(run_evaluation_policy_manifest::run_id.eq(run_id))
                .select(RunEvaluationPolicyManifestRecord::as_select())
                .order((
                    run_evaluation_policy_manifest::agent_id.asc(),
                    run_evaluation_policy_manifest::policy_id.asc(),
                ))
                .load::<RunEvaluationPolicyManifestRecord>(conn)
                .await?;

            let has_unattributed_traces = trace_rows.iter().any(|row| row.1.is_none());
            let capture_status = if barrier_incomplete || has_unattributed_traces {
                "incomplete"
            } else {
                "complete"
            };

            let mut triggered_counts = BTreeMap::<String, i64>::new();
            let mut evidence_ids = BTreeMap::<String, Vec<String>>::new();
            let mut agent_triggered_counts = agent_ids
                .iter()
                .map(|agent_id| (agent_id.clone(), BTreeMap::<String, i64>::new()))
                .collect::<BTreeMap<_, _>>();
            let mut agent_evidence_ids = agent_ids
                .iter()
                .map(|agent_id| (agent_id.clone(), BTreeMap::<String, Vec<String>>::new()))
                .collect::<BTreeMap<_, _>>();
            let mut denied = 0_i64;
            let trace_json = trace_rows
                .iter()
                .map(
                    |(trace_id, agent_id, decision, elapsed_ms, payload, created_at, late)| {
                        if decision == "deny" {
                            denied += 1;
                        }
                        if let Some(policies) = payload
                            .get("triggered_policies")
                            .and_then(serde_json::Value::as_array)
                        {
                            for policy in policies {
                                if let Some(id) =
                                    policy.get("id").and_then(serde_json::Value::as_str)
                                {
                                    *triggered_counts.entry(id.to_string()).or_default() += 1;
                                    evidence_ids
                                        .entry(id.to_string())
                                        .or_default()
                                        .push(trace_id.to_string());
                                    if let Some(agent_id) = agent_id.as_ref() {
                                        *agent_triggered_counts
                                            .entry(agent_id.clone())
                                            .or_default()
                                            .entry(id.to_string())
                                            .or_default() += 1;
                                        agent_evidence_ids
                                            .entry(agent_id.clone())
                                            .or_default()
                                            .entry(id.to_string())
                                            .or_default()
                                            .push(trace_id.to_string());
                                    }
                                }
                            }
                        }
                        let payload = minimize_snapshot_payload(
                            payload.clone(),
                            agent_id
                                .as_ref()
                                .and_then(|agent_id| agent_content_modes.get(agent_id)),
                        );
                        serde_json::json!({
                            "trace_id": trace_id,
                            "agent_id": agent_id,
                            "decision": decision,
                            "elapsed_ms": elapsed_ms,
                            "payload": payload,
                            "created_at": created_at,
                            "late_evidence": late,
                        })
                    },
                )
                .collect::<Vec<_>>();
            let duration_ms = run
                .ended_at
                .map(|ended| (ended - run.started_at).num_milliseconds().max(0))
                .unwrap_or(0);
            let tool_call_count = event_rows
                .iter()
                .filter(|event| event.kind == "tool_call")
                .count() as i64;
            let metrics = BTreeMap::from([
                ("denied_decisions".to_string(), denied),
                ("event_count".to_string(), event_rows.len() as i64),
                ("trace_count".to_string(), trace_rows.len() as i64),
                ("span_count".to_string(), span_rows.len() as i64),
                ("tool_call_count".to_string(), tool_call_count),
                ("duration_ms".to_string(), duration_ms),
            ]);
            let mut agent_metrics = agent_ids
                .iter()
                .map(|agent_id| {
                    (
                        agent_id.clone(),
                        BTreeMap::from([
                            ("denied_decisions".to_string(), 0_i64),
                            ("event_count".to_string(), 0_i64),
                            ("trace_count".to_string(), 0_i64),
                            ("span_count".to_string(), 0_i64),
                            ("tool_call_count".to_string(), 0_i64),
                            ("duration_ms".to_string(), duration_ms),
                        ]),
                    )
                })
                .collect::<BTreeMap<_, _>>();
            for event in &event_rows {
                *agent_metrics
                    .entry(event.agent_id.clone())
                    .or_default()
                    .entry("event_count".into())
                    .or_default() += 1;
                if event.kind == "tool_call" {
                    *agent_metrics
                        .entry(event.agent_id.clone())
                        .or_default()
                        .entry("tool_call_count".into())
                        .or_default() += 1;
                }
            }
            for (_, agent_id, decision, _, _, _, _) in &trace_rows {
                let Some(agent_id) = agent_id else {
                    continue;
                };
                let metrics = agent_metrics.entry(agent_id.clone()).or_default();
                *metrics.entry("trace_count".into()).or_default() += 1;
                if decision == "deny" {
                    *metrics.entry("denied_decisions".into()).or_default() += 1;
                }
            }
            for span in &span_rows {
                *agent_metrics
                    .entry(span.agent_id.clone())
                    .or_default()
                    .entry("span_count".into())
                    .or_default() += 1;
            }
            let snapshot_version = run_snapshots::table
                .filter(run_snapshots::workspace_id.eq(workspace_id))
                .filter(run_snapshots::run_id.eq(run_id))
                .select(max(run_snapshots::snapshot_version))
                .first::<Option<i32>>(conn)
                .await?
                .unwrap_or(0)
                + 1;
            let snapshot_body = serde_json::json!({
                "snapshot_version": snapshot_version,
                "run": run,
                "capture_status": capture_status,
                "cutoff": current,
                "participants": participants,
                "events": event_rows.iter().map(|event| {
                    minimize_snapshot_event(
                        serde_json::to_value(event).unwrap_or(serde_json::Value::Null),
                    )
                }).collect::<Vec<_>>(),
                "traces": trace_json,
                "spans": span_rows,
                "metrics": metrics,
                "agent_metrics": agent_metrics,
                "triggered_policy_counts": triggered_counts,
                "agent_triggered_policy_counts": agent_triggered_counts,
                "evidence_ids": evidence_ids,
                "agent_evidence_ids": agent_evidence_ids,
                "agent_on_incomplete": agent_on_incomplete,
                "agent_content_modes": agent_content_modes,
                "unattributed_trace_count": trace_rows.iter().filter(|row| row.1.is_none()).count(),
            });
            let manifest_body = serde_json::to_value(&manifest)
                .map_err(|error| StorageError::Internal(format!("manifest serialize: {error}")))?;
            let snapshot_hash = blake3_hash(&snapshot_body)?;
            let manifest_hash = blake3_hash(&manifest_body)?;
            let snapshot_id = Uuid::now_v7();
            let late_count = trace_rows.iter().filter(|row| row.6).count()
                + span_rows.iter().filter(|row| row.late_evidence).count();
            diesel::insert_into(run_snapshots::table)
                .values((
                    run_snapshots::workspace_id.eq(workspace_id),
                    run_snapshots::environment_id.eq(environment_id),
                    run_snapshots::id.eq(snapshot_id),
                    run_snapshots::run_id.eq(run_id),
                    run_snapshots::snapshot_version.eq(snapshot_version),
                    run_snapshots::snapshot_hash.eq(&snapshot_hash),
                    run_snapshots::manifest_hash.eq(&manifest_hash),
                    run_snapshots::capture_status.eq(capture_status),
                    run_snapshots::event_cutoff.eq(current),
                    run_snapshots::event_count.eq(event_rows.len() as i64),
                    run_snapshots::trace_count.eq(trace_rows.len() as i64),
                    run_snapshots::span_count.eq(span_rows.len() as i64),
                    run_snapshots::dropped_trace_count.eq(run.dropped_trace_count),
                    run_snapshots::late_evidence_count.eq(late_count as i64),
                    run_snapshots::snapshot.eq(&snapshot_body),
                ))
                .execute(conn)
                .await?;
            diesel::update(
                runs::table
                    .filter(runs::workspace_id.eq(workspace_id))
                    .filter(runs::id.eq(run_id)),
            )
            .set((
                runs::capture_status.eq(capture_status),
                runs::reevaluation_agent_ids.eq::<Option<Vec<String>>>(None),
                runs::updated_at.eq(now),
            ))
            .execute(conn)
            .await?;

            let manifest_by_agent = manifest.iter().fold(
                HashMap::<String, Vec<&RunEvaluationPolicyManifestRecord>>::new(),
                |mut grouped, policy| {
                    grouped
                        .entry(policy.agent_id.clone())
                        .or_default()
                        .push(policy);
                    grouped
                },
            );
            let mut jobs_created = 0;
            for participant in &evaluation_participants {
                let agent_manifest = manifest_by_agent
                    .get(&participant.agent_id)
                    .cloned()
                    .unwrap_or_default();
                let agent_manifest_hash =
                    blake3_hash(&serde_json::to_value(&agent_manifest).map_err(|error| {
                        StorageError::Internal(format!("agent manifest serialize: {error}"))
                    })?)?;
                let job_id = Uuid::now_v7();
                let status = if agent_manifest.is_empty() {
                    "completed"
                } else {
                    "queued"
                };
                let inserted = diesel::insert_into(evaluation_jobs::table)
                    .values((
                        evaluation_jobs::workspace_id.eq(workspace_id),
                        evaluation_jobs::environment_id.eq(environment_id),
                        evaluation_jobs::id.eq(job_id),
                        evaluation_jobs::run_id.eq(run_id),
                        evaluation_jobs::agent_id.eq(&participant.agent_id),
                        evaluation_jobs::snapshot_id.eq(snapshot_id),
                        evaluation_jobs::snapshot_hash.eq(&snapshot_hash),
                        evaluation_jobs::manifest_hash.eq(&agent_manifest_hash),
                        evaluation_jobs::evaluator_version.eq("tl-eval:v1"),
                        evaluation_jobs::status.eq(status),
                    ))
                    .on_conflict_do_nothing()
                    .execute(conn)
                    .await?;
                if inserted == 0 {
                    continue;
                }
                jobs_created += 1;
                if agent_manifest.is_empty() {
                    diesel::insert_into(evaluation_results::table)
                        .values((
                            evaluation_results::workspace_id.eq(workspace_id),
                            evaluation_results::environment_id.eq(environment_id),
                            evaluation_results::id.eq(Uuid::now_v7()),
                            evaluation_results::job_id.eq(job_id),
                            evaluation_results::run_id.eq(run_id),
                            evaluation_results::agent_id.eq(&participant.agent_id),
                            evaluation_results::snapshot_hash.eq(&snapshot_hash),
                            evaluation_results::manifest_hash.eq(&agent_manifest_hash),
                            evaluation_results::evaluator_version.eq("tl-eval:v1"),
                            evaluation_results::verdict.eq("not_configured"),
                            evaluation_results::score_bps.eq::<Option<i32>>(None),
                            evaluation_results::capture_status.eq(capture_status),
                            evaluation_results::llm_audit.eq::<Option<serde_json::Value>>(None),
                        ))
                        .execute(conn)
                        .await?;
                }
            }
            Ok(CaptureAdvanceResult::SnapshotCreated { jobs_created })
        })
        .await
    }

    pub async fn claim_evaluation_job(
        &self,
        worker_id: &str,
        lease_duration: Duration,
        max_attempts: i32,
    ) -> Result<Option<EvaluationJobWork>, StorageError> {
        let mut conn = self.connection().await?;
        conn.transaction::<Option<EvaluationJobWork>, StorageError, _>(async |conn| {
            let current = Utc::now();
            diesel::update(
                evaluation_jobs::table
                    .filter(evaluation_jobs::status.eq("running"))
                    .filter(evaluation_jobs::attempts.ge(max_attempts))
                    .filter(evaluation_jobs::lease_expires_at.lt(Some(current))),
            )
            .set((
                evaluation_jobs::status.eq("error"),
                evaluation_jobs::lease_owner.eq::<Option<String>>(None),
                evaluation_jobs::lease_expires_at.eq::<Option<chrono::DateTime<Utc>>>(None),
                evaluation_jobs::error.eq("evaluation lease expired after the maximum attempts"),
                evaluation_jobs::updated_at.eq(now),
            ))
            .execute(conn)
            .await?;
            let record = evaluation_jobs::table
                .filter(evaluation_jobs::available_at.le(current))
                .filter(evaluation_jobs::attempts.lt(max_attempts))
                .filter(
                    evaluation_jobs::status
                        .eq("queued")
                        .or(evaluation_jobs::status
                            .eq("running")
                            .and(evaluation_jobs::lease_expires_at.lt(Some(current)))),
                )
                .order(evaluation_jobs::created_at.asc())
                .for_update()
                .skip_locked()
                .select(EvaluationJobRecord::as_select())
                .first::<EvaluationJobRecord>(conn)
                .await
                .optional()?;
            let Some(record) = record else {
                return Ok(None);
            };
            let leased = diesel::update(
                evaluation_jobs::table
                    .filter(evaluation_jobs::workspace_id.eq(&record.workspace_id))
                    .filter(evaluation_jobs::id.eq(record.id)),
            )
            .set((
                evaluation_jobs::status.eq("running"),
                evaluation_jobs::attempts.eq(evaluation_jobs::attempts + 1),
                evaluation_jobs::lease_owner.eq(worker_id),
                evaluation_jobs::lease_expires_at.eq(current + lease_duration),
                evaluation_jobs::updated_at.eq(now),
            ))
            .returning(EvaluationJobRecord::as_returning())
            .get_result::<EvaluationJobRecord>(conn)
            .await?;
            let snapshot = run_snapshots::table
                .filter(run_snapshots::workspace_id.eq(&leased.workspace_id))
                .filter(run_snapshots::id.eq(leased.snapshot_id))
                .select(RunSnapshotRecord::as_select())
                .first::<RunSnapshotRecord>(conn)
                .await?;
            let manifest = run_evaluation_policy_manifest::table
                .filter(run_evaluation_policy_manifest::workspace_id.eq(&leased.workspace_id))
                .filter(run_evaluation_policy_manifest::run_id.eq(leased.run_id))
                .filter(run_evaluation_policy_manifest::agent_id.eq(&leased.agent_id))
                .select(RunEvaluationPolicyManifestRecord::as_select())
                .order(run_evaluation_policy_manifest::policy_id.asc())
                .load::<RunEvaluationPolicyManifestRecord>(conn)
                .await?
                .into_iter()
                .map(|policy| FrozenEvaluationPolicy {
                    policy_id: policy.policy_id,
                    policy_version: policy.policy_version,
                    policy_hash: policy.policy_hash,
                    policy_yaml: policy.policy_yaml,
                    weight: policy.weight as u32,
                    critical: policy.critical,
                    evidence_requirements: policy.evidence_requirements,
                })
                .collect();
            Ok(Some(EvaluationJobWork {
                workspace_id: leased.workspace_id,
                environment_id: leased.environment_id,
                job_id: leased.id,
                run_id: leased.run_id,
                agent_id: leased.agent_id,
                snapshot_hash: leased.snapshot_hash,
                manifest_hash: leased.manifest_hash,
                capture_status: snapshot.capture_status,
                snapshot: snapshot.snapshot,
                manifest,
                lease_owner: worker_id.to_string(),
                attempt: leased.attempts,
            }))
        })
        .await
    }

    pub async fn complete_evaluation_job(
        &self,
        work: &EvaluationJobWork,
        result: PersistEvaluationResult,
    ) -> Result<Uuid, StorageError> {
        let mut conn = self.connection().await?;
        conn.transaction::<Uuid, StorageError, _>(async |conn| {
            let lease = evaluation_jobs::table
                .filter(evaluation_jobs::workspace_id.eq(&work.workspace_id))
                .filter(evaluation_jobs::id.eq(work.job_id))
                .select(EvaluationJobRecord::as_select())
                .for_update()
                .first::<EvaluationJobRecord>(conn)
                .await
                .optional()?
                .ok_or(StorageError::NotFound)?;
            if lease.status != "running"
                || lease.lease_owner.as_deref() != Some(work.lease_owner.as_str())
                || lease.attempts != work.attempt
                || lease
                    .lease_expires_at
                    .map_or(true, |lease_expires_at| lease_expires_at <= Utc::now())
            {
                return Err(StorageError::Conflict);
            }
            let result_id = Uuid::now_v7();
            diesel::insert_into(evaluation_results::table)
                .values((
                    evaluation_results::workspace_id.eq(&work.workspace_id),
                    evaluation_results::environment_id.eq(&work.environment_id),
                    evaluation_results::id.eq(result_id),
                    evaluation_results::job_id.eq(work.job_id),
                    evaluation_results::run_id.eq(work.run_id),
                    evaluation_results::agent_id.eq(&work.agent_id),
                    evaluation_results::snapshot_hash.eq(&work.snapshot_hash),
                    evaluation_results::manifest_hash.eq(&work.manifest_hash),
                    evaluation_results::evaluator_version.eq("tl-eval:v1"),
                    evaluation_results::verdict.eq(&result.verdict),
                    evaluation_results::score_bps.eq(result.score_bps),
                    evaluation_results::capture_status.eq(&work.capture_status),
                    evaluation_results::llm_audit.eq(result.llm_audit),
                ))
                .on_conflict((evaluation_results::workspace_id, evaluation_results::job_id))
                .do_nothing()
                .execute(conn)
                .await?;
            let persisted_id = evaluation_results::table
                .filter(evaluation_results::workspace_id.eq(&work.workspace_id))
                .filter(evaluation_results::job_id.eq(work.job_id))
                .select(evaluation_results::id)
                .first::<Uuid>(conn)
                .await?;
            for finding in result.findings {
                diesel::insert_into(evaluation_findings::table)
                    .values((
                        evaluation_findings::workspace_id.eq(&work.workspace_id),
                        evaluation_findings::environment_id.eq(&work.environment_id),
                        evaluation_findings::id.eq(Uuid::now_v7()),
                        evaluation_findings::result_id.eq(persisted_id),
                        evaluation_findings::run_id.eq(work.run_id),
                        evaluation_findings::agent_id.eq(&work.agent_id),
                        evaluation_findings::policy_id.eq(finding.policy_id),
                        evaluation_findings::policy_version.eq(finding.policy_version),
                        evaluation_findings::severity.eq(finding.severity),
                        evaluation_findings::critical.eq(finding.critical),
                        evaluation_findings::status.eq(finding.status),
                        evaluation_findings::score_bps.eq(finding.score_bps),
                        evaluation_findings::reason.eq(finding.reason),
                        evaluation_findings::evidence.eq(finding.evidence),
                    ))
                    .on_conflict_do_nothing()
                    .execute(conn)
                    .await?;
            }
            let job_status = match result.verdict.as_str() {
                "passed" => "completed",
                "failed" => "failed",
                "inconclusive" => "inconclusive",
                _ => "error",
            };
            diesel::update(
                evaluation_jobs::table
                    .filter(evaluation_jobs::workspace_id.eq(&work.workspace_id))
                    .filter(evaluation_jobs::id.eq(work.job_id)),
            )
            .set((
                evaluation_jobs::status.eq(job_status),
                evaluation_jobs::lease_owner.eq::<Option<String>>(None),
                evaluation_jobs::lease_expires_at.eq::<Option<chrono::DateTime<Utc>>>(None),
                evaluation_jobs::error.eq::<Option<String>>(None),
                evaluation_jobs::updated_at.eq(now),
            ))
            .execute(conn)
            .await?;
            Ok(persisted_id)
        })
        .await
    }

    pub async fn retry_evaluation_job(
        &self,
        work: &EvaluationJobWork,
        error: &str,
        max_attempts: i32,
    ) -> Result<(), StorageError> {
        let mut conn = self.connection().await?;
        let terminal = work.attempt >= max_attempts;
        let updated = diesel::update(
            evaluation_jobs::table
                .filter(evaluation_jobs::workspace_id.eq(&work.workspace_id))
                .filter(evaluation_jobs::id.eq(work.job_id))
                .filter(evaluation_jobs::status.eq("running"))
                .filter(evaluation_jobs::lease_owner.eq(&work.lease_owner))
                .filter(evaluation_jobs::attempts.eq(work.attempt))
                .filter(evaluation_jobs::lease_expires_at.gt(Some(Utc::now()))),
        )
        .set((
            evaluation_jobs::status.eq(if terminal { "error" } else { "queued" }),
            evaluation_jobs::available_at
                .eq(Utc::now() + Duration::seconds(2_i64.pow(work.attempt.min(6) as u32))),
            evaluation_jobs::lease_owner.eq::<Option<String>>(None),
            evaluation_jobs::lease_expires_at.eq::<Option<chrono::DateTime<Utc>>>(None),
            evaluation_jobs::error.eq(error.chars().take(1_000).collect::<String>()),
            evaluation_jobs::updated_at.eq(now),
        ))
        .execute(&mut conn)
        .await?;
        if updated != 1 {
            return Err(StorageError::Conflict);
        }
        Ok(())
    }

    pub async fn request_reevaluation(
        &self,
        workspace_id: &str,
        environment_id: &str,
        run_id: Uuid,
        agent_ids: Option<&[String]>,
    ) -> Result<(), StorageError> {
        let mut conn = self.connection().await?;
        conn.transaction::<(), StorageError, _>(async |conn| {
            let run = runs::table
                .filter(runs::workspace_id.eq(workspace_id))
                .filter(runs::environment_id.eq(environment_id))
                .filter(runs::id.eq(run_id))
                .select(RunRecord::as_select())
                .for_update()
                .first::<RunRecord>(conn)
                .await
                .optional()?
                .ok_or(StorageError::NotFound)?;
            if run.finalized_at.is_none() {
                return Err(StorageError::Conflict);
            }
            if run.capture_status == "waiting" {
                return Err(StorageError::Conflict);
            }
            let participants = run_participants::table
                .filter(run_participants::workspace_id.eq(workspace_id))
                .filter(run_participants::run_id.eq(run_id))
                .select(run_participants::agent_id)
                .load::<String>(conn)
                .await?;
            if let Some(requested) = agent_ids {
                if requested.is_empty()
                    || requested
                        .iter()
                        .any(|agent_id| !participants.contains(agent_id))
                {
                    return Err(StorageError::NotFound);
                }
            }
            let max_wait_ms = agent_evaluation_profiles::table
                .filter(agent_evaluation_profiles::workspace_id.eq(workspace_id))
                .filter(agent_evaluation_profiles::environment_id.eq(environment_id))
                .filter(agent_evaluation_profiles::agent_id.eq_any(&participants))
                .filter(agent_evaluation_profiles::enabled.eq(true))
                .select(max(agent_evaluation_profiles::max_capture_wait_ms))
                .first::<Option<i64>>(conn)
                .await?
                .unwrap_or(30_000)
                .clamp(1_000, 3_600_000);
            diesel::update(
                runs::table
                    .filter(runs::workspace_id.eq(workspace_id))
                    .filter(runs::id.eq(run_id)),
            )
            .set((
                runs::capture_status.eq("waiting"),
                runs::capture_deadline.eq(Utc::now() + Duration::milliseconds(max_wait_ms)),
                runs::expected_flush_id.eq::<Option<String>>(None),
                runs::reevaluation_agent_ids.eq(agent_ids.map(ToOwned::to_owned)),
                runs::updated_at.eq(now),
            ))
            .execute(conn)
            .await?;
            Ok(())
        })
        .await
    }
}

fn minimize_snapshot_payload(
    mut payload: serde_json::Value,
    content_mode: Option<&String>,
) -> serde_json::Value {
    let verified_redacted = content_mode.is_some_and(|mode| mode == "redacted")
        && payload.get("redaction").is_some_and(|redaction| {
            redaction.get("status").and_then(serde_json::Value::as_str) == Some("applied")
                && redaction
                    .get("input_redacted")
                    .and_then(serde_json::Value::as_bool)
                    == Some(true)
                && redaction
                    .get("proposed_output_redacted")
                    .and_then(serde_json::Value::as_bool)
                    == Some(true)
                && redaction
                    .get("context_redacted")
                    .and_then(serde_json::Value::as_bool)
                    == Some(true)
        });
    if verified_redacted {
        return payload;
    }
    if let Some(object) = payload.as_object_mut() {
        object.remove("safe_output");
        object.remove("checked_input_excerpt");
        object.remove("checked_output_excerpt");
        if let Some(event) = object.get_mut("event") {
            *event = minimize_snapshot_event(std::mem::take(event));
        }
    }
    payload
}

fn minimize_snapshot_event(mut event: serde_json::Value) -> serde_json::Value {
    let Some(object) = event.as_object_mut() else {
        return event;
    };
    for key in ["label", "input_summary", "output_summary", "context"] {
        if object.contains_key(key) {
            object.insert(key.into(), serde_json::Value::Null);
        }
    }
    for key in ["metadata", "provenance"] {
        if object.contains_key(key) {
            object.insert(key.into(), serde_json::json!({}));
        }
    }
    if object.contains_key("sources") {
        object.insert("sources".into(), serde_json::json!([]));
    }
    if let Some(action) = object
        .get_mut("action")
        .and_then(serde_json::Value::as_object_mut)
    {
        action.insert("parameters".into(), serde_json::Value::Null);
    }
    event
}

fn blake3_hash(value: &serde_json::Value) -> Result<String, StorageError> {
    let bytes = serde_json::to_vec(value)
        .map_err(|error| StorageError::Internal(format!("canonical json encode: {error}")))?;
    Ok(format!("blake3:v1:{}", blake3::hash(&bytes).to_hex()))
}

fn parse_job_status(value: &str) -> Result<EvaluationJobStatus, StorageError> {
    match value {
        "waiting_capture" => Ok(EvaluationJobStatus::WaitingCapture),
        "queued" => Ok(EvaluationJobStatus::Queued),
        "running" => Ok(EvaluationJobStatus::Running),
        "completed" => Ok(EvaluationJobStatus::Completed),
        "failed" => Ok(EvaluationJobStatus::Failed),
        "inconclusive" => Ok(EvaluationJobStatus::Inconclusive),
        "error" => Ok(EvaluationJobStatus::Error),
        other => Err(StorageError::Internal(format!(
            "unknown evaluation job status `{other}`"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacted_snapshot_requires_verified_redaction_report() {
        let mode = "redacted".to_string();
        let payload = serde_json::json!({
            "safe_output": "secret",
            "event": {
                "action": { "parameters": { "secret": true } },
                "context": { "secret": true }
            }
        });

        let minimized = minimize_snapshot_payload(payload, Some(&mode));
        assert!(minimized.get("safe_output").is_none());
        assert!(minimized["event"]["action"]["parameters"].is_null());
        assert!(minimized["event"]["context"].is_null());
    }

    #[test]
    fn verified_redacted_snapshot_retains_redacted_content() {
        let mode = "redacted".to_string();
        let payload = serde_json::json!({
            "safe_output": "[REDACTED]",
            "redaction": {
                "status": "applied",
                "input_redacted": true,
                "proposed_output_redacted": true,
                "context_redacted": true
            },
            "event": { "context": { "account": "[REDACTED]" } }
        });

        assert_eq!(
            minimize_snapshot_payload(payload.clone(), Some(&mode)),
            payload
        );
    }
}
