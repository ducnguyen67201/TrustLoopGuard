//! Restart-safe post-run evaluation worker.
//!
//! Postgres owns queue state and leases. This task is only an orchestrator, so
//! restarting the server cannot lose a finalized run or an in-flight job.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration as StdDuration;

use chrono::Duration;
use tl_core::{EvaluationFindingStatus, EvaluationVerdict, RunCaptureStatus, Severity};
use tl_eval::{FindingOutput, ManifestEntry, PolicyReplayPort, RubricGraderPort, SnapshotEvidence};
use tl_llm::{JsonSchema, JudgeKind, LlmCallAudit, LlmRouteKind, LlmRouter};
use tl_policy::family_ast::{EvaluationGrader, EvaluationPolicy};
use tl_policy::{AnyPolicy, FamilyPolicy};
use tl_storage::{
    EvaluationJobWork, EvaluationRepo, PersistEvaluationFinding, PersistEvaluationResult,
};

#[derive(Debug, Clone)]
pub struct EvaluationWorkerConfig {
    pub poll_interval: StdDuration,
    pub lease_duration: Duration,
    pub capture_batch_size: i64,
    pub max_attempts: i32,
}

impl Default for EvaluationWorkerConfig {
    fn default() -> Self {
        Self {
            poll_interval: StdDuration::from_millis(500),
            lease_duration: Duration::seconds(30),
            capture_batch_size: 25,
            max_attempts: 3,
        }
    }
}

pub fn spawn_evaluation_worker(
    repo: Arc<EvaluationRepo>,
    llm: Arc<LlmRouter>,
    config: EvaluationWorkerConfig,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let worker_id = format!("eval-{}", uuid::Uuid::now_v7());
        let mut ticker = tokio::time::interval(config.poll_interval);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            ticker.tick().await;
            if let Err(error) = repo.advance_due_captures(config.capture_batch_size).await {
                tracing::warn!(worker_id, error = %error, "evaluation capture sweep failed");
            }
            loop {
                let work = match repo
                    .claim_evaluation_job(&worker_id, config.lease_duration, config.max_attempts)
                    .await
                {
                    Ok(Some(work)) => work,
                    Ok(None) => break,
                    Err(error) => {
                        tracing::warn!(worker_id, error = %error, "evaluation job claim failed");
                        break;
                    }
                };
                process_job(&repo, &llm, &config, &worker_id, work).await;
            }
        }
    })
}

async fn process_job(
    repo: &EvaluationRepo,
    llm: &LlmRouter,
    config: &EvaluationWorkerConfig,
    worker_id: &str,
    work: EvaluationJobWork,
) {
    tracing::info!(
        worker_id,
        job_id = %work.job_id,
        run_id = %work.run_id,
        agent_id = %work.agent_id,
        attempt = work.attempt,
        "evaluation job started"
    );
    let outcome = evaluate(&work, llm).await;
    match outcome {
        Ok(result) => match repo.complete_evaluation_job(&work, result).await {
            Ok(result_id) => tracing::info!(
                worker_id,
                job_id = %work.job_id,
                result_id = %result_id,
                run_id = %work.run_id,
                agent_id = %work.agent_id,
                "evaluation job completed"
            ),
            Err(error) => {
                tracing::warn!(
                    worker_id,
                    job_id = %work.job_id,
                    run_id = %work.run_id,
                    agent_id = %work.agent_id,
                    error = %error,
                    "evaluation result persistence failed"
                );
                let _ = repo
                    .retry_evaluation_job(&work, &error.to_string(), config.max_attempts)
                    .await;
            }
        },
        Err(error) => {
            tracing::warn!(
                worker_id,
                job_id = %work.job_id,
                run_id = %work.run_id,
                agent_id = %work.agent_id,
                error,
                "evaluation job failed"
            );
            let _ = repo
                .retry_evaluation_job(&work, &error, config.max_attempts)
                .await;
        }
    }
}

async fn evaluate(
    work: &EvaluationJobWork,
    llm: &LlmRouter,
) -> Result<PersistEvaluationResult, String> {
    let metrics = agent_value_map(&work.snapshot, "agent_metrics", &work.agent_id)?
        .unwrap_or(value_map(&work.snapshot, "metrics")?);
    let triggered_policy_counts = agent_value_map(
        &work.snapshot,
        "agent_triggered_policy_counts",
        &work.agent_id,
    )?
    .unwrap_or(value_map(&work.snapshot, "triggered_policy_counts")?);
    let evidence_ids = agent_evidence_map(&work.snapshot, &work.agent_id)?.unwrap_or(
        work.snapshot
            .get("evidence_ids")
            .cloned()
            .map(serde_json::from_value::<BTreeMap<String, Vec<String>>>)
            .transpose()
            .map_err(|error| format!("snapshot evidence IDs are invalid: {error}"))?
            .unwrap_or_default(),
    );
    let capture_status = match work.capture_status.as_str() {
        "complete" => RunCaptureStatus::Complete,
        "incomplete" => RunCaptureStatus::Incomplete,
        "waiting" => RunCaptureStatus::Waiting,
        _ => RunCaptureStatus::Open,
    };
    let snapshot = SnapshotEvidence {
        snapshot_hash: work.snapshot_hash.clone(),
        capture_status,
        metrics,
        triggered_policy_counts,
        evidence_ids,
    };
    let manifest = work
        .manifest
        .iter()
        .map(|policy| ManifestEntry {
            policy_id: policy.policy_id.clone(),
            policy_version: policy.policy_version,
            policy_hash: policy.policy_hash.clone(),
            policy_yaml: policy.policy_yaml.clone(),
            weight: policy.weight,
            critical: policy.critical,
        })
        .collect::<Vec<_>>();
    let rubric = prepare_rubric_grader(work, &snapshot, &manifest, llm).await;
    let output =
        tl_eval::evaluate_with_adapters(&snapshot, &manifest, &SnapshotPolicyReplay, &rubric)
            .map_err(|error| error.to_string())?;
    Ok(PersistEvaluationResult {
        verdict: verdict_text(output.verdict).to_string(),
        score_bps: output.score_bps.map(|score| score as i32),
        llm_audit: rubric.audit.clone(),
        findings: output
            .findings
            .into_iter()
            .map(|finding| PersistEvaluationFinding {
                policy_id: finding.policy_id,
                policy_version: finding.policy_version,
                severity: severity_text(finding.severity).to_string(),
                critical: finding.critical,
                status: finding_status_text(finding.status).to_string(),
                score_bps: finding.score_bps.map(|score| score as i32),
                reason: finding.reason,
                evidence: serde_json::Value::Array(
                    finding
                        .evidence_ids
                        .into_iter()
                        .map(|id| serde_json::json!({ "kind": "trace_or_span", "id": id }))
                        .collect(),
                ),
            })
            .collect(),
    })
}

#[derive(Debug)]
struct SnapshotPolicyReplay;

impl PolicyReplayPort for SnapshotPolicyReplay {
    fn replay(
        &self,
        snapshot: &SnapshotEvidence,
        entry: &ManifestEntry,
        policy: &EvaluationPolicy,
    ) -> Result<FindingOutput, String> {
        let EvaluationGrader::PolicyReplay { policy_ids } = &policy.grader else {
            return Err("policy replay adapter received a non-replay grader".into());
        };
        let violations = policy_ids
            .iter()
            .map(|id| {
                snapshot
                    .triggered_policy_counts
                    .get(id)
                    .copied()
                    .unwrap_or(0)
            })
            .sum::<i64>();
        let passed = violations <= i64::from(policy.expect.max_violations);
        Ok(FindingOutput {
            policy_id: entry.policy_id.clone(),
            policy_version: entry.policy_version,
            policy_hash: entry.policy_hash.clone(),
            severity: policy.severity,
            critical: entry.critical,
            weight: entry.weight,
            status: if passed {
                EvaluationFindingStatus::Passed
            } else {
                EvaluationFindingStatus::Failed
            },
            score_bps: Some(if passed { 10_000 } else { 0 }),
            reason: format!(
                "replayed persisted outcomes for {} policy version(s): {violations} violation(s)",
                policy_ids.len()
            ),
            evidence_ids: policy_ids
                .iter()
                .flat_map(|id| snapshot.evidence_ids.get(id).cloned().unwrap_or_default())
                .collect(),
        })
    }
}

#[derive(Debug)]
struct PreparedRubricGrader {
    outcome: Result<BTreeMap<String, FindingOutput>, String>,
    audit: Option<serde_json::Value>,
}

impl RubricGraderPort for PreparedRubricGrader {
    fn grade_batch(
        &self,
        _snapshot: &SnapshotEvidence,
        policies: &[(&ManifestEntry, &EvaluationPolicy)],
    ) -> Result<BTreeMap<String, FindingOutput>, String> {
        let result = self.outcome.clone()?;
        let expected = policies
            .iter()
            .map(|(entry, _)| entry.policy_id.as_str())
            .collect::<std::collections::BTreeSet<_>>();
        let actual = result
            .keys()
            .map(String::as_str)
            .collect::<std::collections::BTreeSet<_>>();
        if actual != expected {
            return Err("batched rubric result policy IDs do not match the frozen manifest".into());
        }
        Ok(result)
    }
}

async fn prepare_rubric_grader(
    work: &EvaluationJobWork,
    snapshot: &SnapshotEvidence,
    manifest: &[ManifestEntry],
    llm: &LlmRouter,
) -> PreparedRubricGrader {
    if snapshot.capture_status != RunCaptureStatus::Complete {
        return PreparedRubricGrader {
            outcome: Ok(BTreeMap::new()),
            audit: None,
        };
    }
    let mut policies = Vec::new();
    for entry in manifest {
        let parsed = match tl_policy::load_any_str(&entry.policy_yaml) {
            Ok(AnyPolicy::Family(FamilyPolicy::Evaluation(policy))) => policy,
            Ok(_) => continue,
            Err(error) => {
                return PreparedRubricGrader {
                    outcome: Err(format!(
                        "rubric manifest policy {} failed to parse: {error}",
                        entry.policy_id
                    )),
                    audit: None,
                }
            }
        };
        if matches!(parsed.grader, EvaluationGrader::LlmRubric { .. }) {
            policies.push((entry, parsed));
        }
    }
    if policies.is_empty() {
        return PreparedRubricGrader {
            outcome: Ok(BTreeMap::new()),
            audit: None,
        };
    }
    if !llm.has_workload_route(LlmRouteKind::RunEvaluation) {
        return PreparedRubricGrader {
            outcome: Err("run evaluation rubric route is not configured".into()),
            audit: None,
        };
    }

    let policy_json = policies
        .iter()
        .map(|(entry, policy)| {
            let EvaluationGrader::LlmRubric { rubric, min_score } = &policy.grader else {
                unreachable!("rubric policies were filtered")
            };
            serde_json::json!({
                "policy_id": entry.policy_id,
                "scope": policy.scope,
                "severity": policy.severity,
                "rubric": rubric,
                "minimum_score_bps": (*min_score * 10_000.0).round() as u32,
            })
        })
        .collect::<Vec<_>>();
    let evidence = serde_json::json!({
        "snapshot_hash": snapshot.snapshot_hash,
        "agent_id": work.agent_id,
        "metrics": snapshot.metrics,
        "events": work.snapshot.get("events").cloned().unwrap_or_default(),
        "traces": work.snapshot.get("traces").cloned().unwrap_or_default(),
        "spans": work.snapshot.get("spans").cloned().unwrap_or_default(),
    });
    let prompt = match serde_json::to_string(&serde_json::json!({
        "instruction": "Score every policy exactly once using only the supplied immutable evidence. Return basis-point integer scores and concise reasons.",
        "policies": policy_json,
        "evidence": evidence,
    })) {
        Ok(prompt) if prompt.len() <= 64 * 1024 => prompt,
        Ok(_) => {
            return PreparedRubricGrader {
                outcome: Err("rubric evidence exceeds the 64 KiB evaluation limit".into()),
                audit: None,
            }
        }
        Err(error) => {
            return PreparedRubricGrader {
                outcome: Err(format!("rubric prompt serialization failed: {error}")),
                audit: None,
            }
        }
    };
    let policy_ids = policies
        .iter()
        .map(|(entry, _)| entry.policy_id.clone())
        .collect::<Vec<_>>();
    let schema = JsonSchema {
        name: "RunEvaluationRubricBatch".into(),
        schema: serde_json::json!({
            "type": "object",
            "properties": {
                "findings": {
                    "type": "array",
                    "minItems": policy_ids.len(),
                    "maxItems": policy_ids.len(),
                    "items": {
                        "type": "object",
                        "properties": {
                            "policy_id": { "type": "string", "enum": policy_ids },
                            "score_bps": { "type": "integer", "minimum": 0, "maximum": 10000 },
                            "reason": { "type": "string", "maxLength": 1000 },
                            "evidence_ids": {
                                "type": "array",
                                "maxItems": 64,
                                "items": { "type": "string", "maxLength": 256 }
                            }
                        },
                        "required": ["policy_id", "score_bps", "reason", "evidence_ids"],
                        "additionalProperties": false
                    }
                }
            },
            "required": ["findings"],
            "additionalProperties": false
        }),
    };
    match llm
        .judge_with_audit(
            JudgeKind::RunEvaluation,
            &work.workspace_id,
            &prompt,
            &schema,
        )
        .await
    {
        Ok(output) => PreparedRubricGrader {
            outcome: parse_rubric_findings(&output.output.json, snapshot, &policies),
            audit: Some(llm_audit_json(&output.audit)),
        },
        Err(error) => PreparedRubricGrader {
            outcome: Err(format!("batched rubric call failed: {}", error.error)),
            audit: Some(llm_audit_json(&error.audit)),
        },
    }
}

fn llm_audit_json(audit: &LlmCallAudit) -> serde_json::Value {
    serde_json::json!({
        "usage_kind": "guardrail",
        "judge": audit.judge,
        "provider": audit.provider,
        "model": audit.model,
        "status": audit.status,
        "prompt_tokens": audit.prompt_tokens,
        "completion_tokens": audit.completion_tokens,
        "fallback_used": audit.fallback_used,
        "latency_ms": audit.latency_ms,
        "error_code": audit.error_code,
    })
}

fn parse_rubric_findings(
    output: &serde_json::Value,
    snapshot: &SnapshotEvidence,
    policies: &[(&ManifestEntry, EvaluationPolicy)],
) -> Result<BTreeMap<String, FindingOutput>, String> {
    let rows = output
        .get("findings")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "batched rubric response has no findings array".to_string())?;
    let known_evidence = snapshot
        .evidence_ids
        .values()
        .flatten()
        .map(String::as_str)
        .collect::<std::collections::HashSet<_>>();
    let mut findings = BTreeMap::new();
    for row in rows {
        let policy_id = row
            .get("policy_id")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| "rubric finding policy_id is missing".to_string())?;
        let (entry, policy) = policies
            .iter()
            .find(|(entry, _)| entry.policy_id == policy_id)
            .ok_or_else(|| format!("rubric returned unknown policy `{policy_id}`"))?;
        let score_bps = row
            .get("score_bps")
            .and_then(serde_json::Value::as_u64)
            .and_then(|score| u32::try_from(score).ok())
            .filter(|score| *score <= 10_000)
            .ok_or_else(|| format!("rubric score for `{policy_id}` is outside 0..=10000"))?;
        let reason = row
            .get("reason")
            .and_then(serde_json::Value::as_str)
            .filter(|reason| !reason.trim().is_empty() && reason.len() <= 1_000)
            .ok_or_else(|| format!("rubric reason for `{policy_id}` is invalid"))?;
        let evidence_ids = row
            .get("evidence_ids")
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| format!("rubric evidence for `{policy_id}` is invalid"))?
            .iter()
            .map(|value| {
                value
                    .as_str()
                    .filter(|id| known_evidence.contains(id))
                    .map(str::to_string)
                    .ok_or_else(|| format!("rubric cited unknown evidence for `{policy_id}`"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let EvaluationGrader::LlmRubric { min_score, .. } = &policy.grader else {
            return Err(format!("manifest policy `{policy_id}` is not a rubric"));
        };
        let passed = score_bps >= (*min_score * 10_000.0).round() as u32;
        let finding = FindingOutput {
            policy_id: entry.policy_id.clone(),
            policy_version: entry.policy_version,
            policy_hash: entry.policy_hash.clone(),
            severity: policy.severity,
            critical: entry.critical,
            weight: entry.weight,
            status: if passed {
                EvaluationFindingStatus::Passed
            } else {
                EvaluationFindingStatus::Failed
            },
            score_bps: Some(score_bps),
            reason: reason.to_string(),
            evidence_ids,
        };
        if findings.insert(policy_id.to_string(), finding).is_some() {
            return Err(format!("rubric returned duplicate policy `{policy_id}`"));
        }
    }
    if findings.len() != policies.len() {
        return Err("batched rubric response omitted one or more policies".into());
    }
    Ok(findings)
}

fn value_map(value: &serde_json::Value, key: &str) -> Result<BTreeMap<String, i64>, String> {
    value
        .get(key)
        .cloned()
        .map(serde_json::from_value)
        .transpose()
        .map_err(|error| format!("snapshot `{key}` is invalid: {error}"))
        .map(Option::unwrap_or_default)
}

fn agent_value_map(
    value: &serde_json::Value,
    key: &str,
    agent_id: &str,
) -> Result<Option<BTreeMap<String, i64>>, String> {
    value
        .get(key)
        .and_then(|agents| agents.get(agent_id))
        .cloned()
        .map(serde_json::from_value)
        .transpose()
        .map_err(|error| format!("snapshot `{key}.{agent_id}` is invalid: {error}"))
}

fn agent_evidence_map(
    value: &serde_json::Value,
    agent_id: &str,
) -> Result<Option<BTreeMap<String, Vec<String>>>, String> {
    value
        .get("agent_evidence_ids")
        .and_then(|agents| agents.get(agent_id))
        .cloned()
        .map(serde_json::from_value)
        .transpose()
        .map_err(|error| format!("snapshot agent evidence for `{agent_id}` is invalid: {error}"))
}

fn verdict_text(verdict: EvaluationVerdict) -> &'static str {
    match verdict {
        EvaluationVerdict::Passed => "passed",
        EvaluationVerdict::Failed => "failed",
        EvaluationVerdict::Inconclusive => "inconclusive",
        EvaluationVerdict::Error => "error",
        EvaluationVerdict::NotConfigured => "not_configured",
    }
}

fn finding_status_text(status: EvaluationFindingStatus) -> &'static str {
    match status {
        EvaluationFindingStatus::Passed => "passed",
        EvaluationFindingStatus::Failed => "failed",
        EvaluationFindingStatus::Inconclusive => "inconclusive",
        EvaluationFindingStatus::Error => "error",
        EvaluationFindingStatus::NotApplicable => "not_applicable",
    }
}

fn severity_text(severity: Severity) -> &'static str {
    match severity {
        Severity::Low => "low",
        Severity::Medium => "medium",
        Severity::High => "high",
        Severity::Critical => "critical",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn incomplete_snapshot_cannot_become_passed() {
        let work = EvaluationJobWork {
            workspace_id: "ws".into(),
            environment_id: "env".into(),
            job_id: uuid::Uuid::nil(),
            run_id: uuid::Uuid::nil(),
            agent_id: "agent".into(),
            snapshot_hash: "blake3:v1:snapshot".into(),
            manifest_hash: "blake3:v1:manifest".into(),
            capture_status: "incomplete".into(),
            snapshot: serde_json::json!({ "metrics": {}, "triggered_policy_counts": {} }),
            manifest: vec![tl_storage::evaluation_worker_repo::FrozenEvaluationPolicy {
                policy_id: "completion".into(),
                policy_version: 1,
                policy_hash: "blake3:v1:policy".into(),
                policy_yaml: "family: evaluation\nid: completion\nseverity: high\nscope: trajectory\ngrader:\n  kind: run_metric\n  metric: event_count\n  comparator: gte\n  value: 1\non_missing_evidence: inconclusive\n".into(),
                weight: 1,
                critical: false,
            }],
            attempt: 1,
        };
        let result = evaluate(&work, &LlmRouter::empty())
            .await
            .expect("evaluation result");
        assert_ne!(result.verdict, "passed");
        assert!(result.llm_audit.is_none());
    }
}
