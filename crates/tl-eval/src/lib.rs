//! Pure post-run evaluator. This crate has no HTTP, database, or provider
//! dependency; orchestration and adapter implementations belong in tl-server.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use tl_core::{
    EvaluationCampaignAggregate, EvaluationCampaignCaseResult, EvaluationCaseSpec,
    EvaluationCaseStatus, EvaluationFindingStatus, EvaluationReleaseGate,
    EvaluationReleaseGateVerdict, EvaluationVerdict, MissingEvidenceBehavior, RunCaptureStatus,
    Severity,
};
use tl_policy::family_ast::{EvaluationGrader, EvaluationPolicy, RunMetricComparator};
use tl_policy::{AnyPolicy, FamilyPolicy};

pub const EVALUATOR_VERSION: &str = "tl-eval:v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotEvidence {
    pub snapshot_hash: String,
    pub capture_status: RunCaptureStatus,
    #[serde(default)]
    pub metrics: BTreeMap<String, i64>,
    #[serde(default)]
    pub triggered_policy_counts: BTreeMap<String, i64>,
    #[serde(default)]
    pub evidence_ids: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct ManifestEntry {
    pub policy_id: String,
    pub policy_version: i32,
    pub policy_hash: String,
    pub policy_yaml: String,
    pub weight: u32,
    pub critical: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindingOutput {
    pub policy_id: String,
    pub policy_version: i32,
    pub policy_hash: String,
    pub severity: Severity,
    pub critical: bool,
    pub weight: u32,
    pub status: EvaluationFindingStatus,
    pub score_bps: Option<u32>,
    pub reason: String,
    pub evidence_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationOutput {
    pub evaluator_version: String,
    pub verdict: EvaluationVerdict,
    pub score_bps: Option<u32>,
    pub findings: Vec<FindingOutput>,
}

pub trait PolicyReplayPort: Send + Sync {
    fn replay(
        &self,
        snapshot: &SnapshotEvidence,
        entry: &ManifestEntry,
        policy: &EvaluationPolicy,
    ) -> Result<FindingOutput, String>;
}

/// A server adapter must batch every rubric policy for one run/agent into a
/// single provider call before returning keyed findings.
pub trait RubricGraderPort: Send + Sync {
    fn grade_batch(
        &self,
        snapshot: &SnapshotEvidence,
        policies: &[(&ManifestEntry, &EvaluationPolicy)],
    ) -> Result<BTreeMap<String, FindingOutput>, String>;
}

#[derive(Debug, thiserror::Error)]
pub enum EvaluationError {
    #[error("manifest policy parse failed for {policy_id}: {message}")]
    PolicyParse { policy_id: String, message: String },
    #[error("manifest entry {0} is not an evaluation policy")]
    WrongFamily(String),
}

#[derive(Debug, thiserror::Error)]
pub enum CampaignAggregationError {
    #[error("campaign returned duplicate result for case {0}")]
    DuplicateCase(String),
    #[error("campaign returned result for unknown case {0}")]
    UnknownCase(String),
}

/// Content address a JSON-compatible manifest after recursively sorting every
/// object key. Integer basis points keep evaluation output free of unstable
/// floating-point serialization.
pub fn canonical_hash<T: Serialize>(value: &T) -> Result<String, serde_json::Error> {
    let value = serde_json::to_value(value)?;
    let bytes = serde_json::to_vec(&canonicalize(value))?;
    Ok(format!("blake3:v1:{}", blake3::hash(&bytes).to_hex()))
}

fn canonicalize(value: serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Array(values) => {
            serde_json::Value::Array(values.into_iter().map(canonicalize).collect())
        }
        serde_json::Value::Object(values) => {
            let sorted = values
                .into_iter()
                .map(|(key, value)| (key, canonicalize(value)))
                .collect::<BTreeMap<_, _>>();
            serde_json::to_value(sorted).expect("JSON object remains serializable")
        }
        scalar => scalar,
    }
}

pub fn evaluate_deterministic(
    snapshot: &SnapshotEvidence,
    manifest: &[ManifestEntry],
) -> Result<EvaluationOutput, EvaluationError> {
    let mut entries = manifest.to_vec();
    entries.sort_by(|left, right| left.policy_id.cmp(&right.policy_id));
    if entries.is_empty() {
        return Ok(EvaluationOutput {
            evaluator_version: EVALUATOR_VERSION.into(),
            verdict: EvaluationVerdict::NotConfigured,
            score_bps: None,
            findings: Vec::new(),
        });
    }

    let mut findings = Vec::with_capacity(entries.len());
    for entry in &entries {
        let parsed = tl_policy::load_any_str(&entry.policy_yaml).map_err(|error| {
            EvaluationError::PolicyParse {
                policy_id: entry.policy_id.clone(),
                message: error.to_string(),
            }
        })?;
        let AnyPolicy::Family(FamilyPolicy::Evaluation(policy)) = parsed else {
            return Err(EvaluationError::WrongFamily(entry.policy_id.clone()));
        };
        findings.push(evaluate_policy(snapshot, entry, &policy));
    }
    Ok(aggregate(findings))
}

/// Evaluate a frozen manifest with server-owned replay and rubric adapters.
/// Rubric targets are deliberately collected and sent through one batch call
/// for the whole run/agent; callers cannot accidentally create an LLM call
/// per policy.
pub fn evaluate_with_adapters(
    snapshot: &SnapshotEvidence,
    manifest: &[ManifestEntry],
    replay: &dyn PolicyReplayPort,
    rubric: &dyn RubricGraderPort,
) -> Result<EvaluationOutput, EvaluationError> {
    let mut entries = manifest.to_vec();
    entries.sort_by(|left, right| left.policy_id.cmp(&right.policy_id));
    if entries.is_empty() {
        return evaluate_deterministic(snapshot, &entries);
    }
    let mut parsed = Vec::with_capacity(entries.len());
    for entry in entries {
        let policy = tl_policy::load_any_str(&entry.policy_yaml).map_err(|error| {
            EvaluationError::PolicyParse {
                policy_id: entry.policy_id.clone(),
                message: error.to_string(),
            }
        })?;
        let AnyPolicy::Family(FamilyPolicy::Evaluation(policy)) = policy else {
            return Err(EvaluationError::WrongFamily(entry.policy_id));
        };
        parsed.push((entry, policy));
    }

    let rubric_targets = if snapshot.capture_status == RunCaptureStatus::Complete {
        parsed
            .iter()
            .filter(|(_, policy)| matches!(&policy.grader, EvaluationGrader::LlmRubric { .. }))
            .map(|(entry, policy)| (entry, policy))
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    let rubric_findings = if rubric_targets.is_empty() {
        Ok(BTreeMap::new())
    } else {
        rubric.grade_batch(snapshot, &rubric_targets)
    };

    let mut findings = Vec::with_capacity(parsed.len());
    for (entry, policy) in &parsed {
        let finding = if snapshot.capture_status != RunCaptureStatus::Complete {
            evaluate_policy(snapshot, entry, policy)
        } else {
            match &policy.grader {
                EvaluationGrader::PolicyReplay { .. } => replay
                    .replay(snapshot, entry, policy)
                    .unwrap_or_else(|error| adapter_error(entry, policy.severity, error)),
                EvaluationGrader::LlmRubric { .. } => match &rubric_findings {
                    Ok(by_policy) => {
                        by_policy.get(&entry.policy_id).cloned().unwrap_or_else(|| {
                            adapter_error(
                                entry,
                                policy.severity,
                                "batched rubric response omitted this policy".into(),
                            )
                        })
                    }
                    Err(error) => adapter_error(entry, policy.severity, error.clone()),
                },
                _ => evaluate_policy(snapshot, entry, policy),
            }
        };
        findings.push(finding);
    }
    Ok(aggregate(findings))
}

fn adapter_error(entry: &ManifestEntry, severity: Severity, reason: String) -> FindingOutput {
    finding(
        entry,
        severity,
        EvaluationFindingStatus::Error,
        None,
        reason,
        Vec::new(),
    )
}

fn evaluate_policy(
    snapshot: &SnapshotEvidence,
    entry: &ManifestEntry,
    policy: &EvaluationPolicy,
) -> FindingOutput {
    if snapshot.capture_status != RunCaptureStatus::Complete {
        let status = match policy.on_missing_evidence {
            MissingEvidenceBehavior::Fail => EvaluationFindingStatus::Failed,
            MissingEvidenceBehavior::Inconclusive => EvaluationFindingStatus::Inconclusive,
        };
        return finding(
            entry,
            policy.severity,
            status,
            (status == EvaluationFindingStatus::Failed).then_some(0),
            "required run evidence is incomplete".into(),
            Vec::new(),
        );
    }
    match &policy.grader {
        EvaluationGrader::RuntimePolicyObservation { policy_ids } => {
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
            finding(
                entry,
                policy.severity,
                if passed {
                    EvaluationFindingStatus::Passed
                } else {
                    EvaluationFindingStatus::Failed
                },
                Some(if passed { 10_000 } else { 0 }),
                format!(
                    "observed {violations} runtime-policy violation(s); allowed {}",
                    policy.expect.max_violations
                ),
                policy_ids
                    .iter()
                    .flat_map(|id| snapshot.evidence_ids.get(id).cloned().unwrap_or_default())
                    .collect(),
            )
        }
        EvaluationGrader::RunMetric {
            metric,
            comparator,
            value,
        } => {
            let Some(actual) = snapshot.metrics.get(metric).copied() else {
                let status = match policy.on_missing_evidence {
                    MissingEvidenceBehavior::Fail => EvaluationFindingStatus::Failed,
                    MissingEvidenceBehavior::Inconclusive => EvaluationFindingStatus::Inconclusive,
                };
                return finding(
                    entry,
                    policy.severity,
                    status,
                    (status == EvaluationFindingStatus::Failed).then_some(0),
                    format!("snapshot metric `{metric}` is missing"),
                    Vec::new(),
                );
            };
            let passed = match comparator {
                RunMetricComparator::Eq => actual == *value,
                RunMetricComparator::Lte => actual <= *value,
                RunMetricComparator::Gte => actual >= *value,
            };
            finding(
                entry,
                policy.severity,
                if passed {
                    EvaluationFindingStatus::Passed
                } else {
                    EvaluationFindingStatus::Failed
                },
                Some(if passed { 10_000 } else { 0 }),
                format!("metric `{metric}` was {actual}; expected {comparator:?} {value}"),
                snapshot
                    .evidence_ids
                    .get(metric)
                    .cloned()
                    .unwrap_or_default(),
            )
        }
        EvaluationGrader::PolicyReplay { .. } => finding(
            entry,
            policy.severity,
            EvaluationFindingStatus::Inconclusive,
            None,
            "policy replay adapter is not configured".into(),
            Vec::new(),
        ),
        EvaluationGrader::LlmRubric { .. } => finding(
            entry,
            policy.severity,
            EvaluationFindingStatus::Inconclusive,
            None,
            "rubric adapter is not configured".into(),
            Vec::new(),
        ),
    }
}

fn finding(
    entry: &ManifestEntry,
    severity: Severity,
    status: EvaluationFindingStatus,
    score_bps: Option<u32>,
    reason: String,
    evidence_ids: Vec<String>,
) -> FindingOutput {
    FindingOutput {
        policy_id: entry.policy_id.clone(),
        policy_version: entry.policy_version,
        policy_hash: entry.policy_hash.clone(),
        severity,
        critical: entry.critical,
        weight: entry.weight,
        status,
        score_bps,
        reason,
        evidence_ids,
    }
}

fn aggregate(findings: Vec<FindingOutput>) -> EvaluationOutput {
    let critical_failed = findings
        .iter()
        .any(|finding| finding.critical && finding.status == EvaluationFindingStatus::Failed);
    let has_failed = findings
        .iter()
        .any(|finding| finding.status == EvaluationFindingStatus::Failed);
    let has_inconclusive = findings
        .iter()
        .any(|finding| finding.status == EvaluationFindingStatus::Inconclusive);
    let has_error = findings
        .iter()
        .any(|finding| finding.status == EvaluationFindingStatus::Error);
    let verdict = if critical_failed || has_failed {
        EvaluationVerdict::Failed
    } else if has_error {
        EvaluationVerdict::Error
    } else if has_inconclusive {
        EvaluationVerdict::Inconclusive
    } else {
        EvaluationVerdict::Passed
    };
    let (weighted, total_weight) = findings.iter().fold((0_u64, 0_u64), |acc, finding| {
        let weight = u64::from(finding.weight);
        (
            acc.0 + u64::from(finding.score_bps.unwrap_or(0)) * weight,
            acc.1 + weight,
        )
    });
    EvaluationOutput {
        evaluator_version: EVALUATOR_VERSION.into(),
        verdict,
        score_bps: (total_weight > 0).then_some((weighted / total_weight) as u32),
        findings,
    }
}

/// Aggregate completed customer-run cases with the same integer, weighted,
/// critical-override semantics as production Run evaluation.
pub fn aggregate_campaign(
    cases: &[EvaluationCaseSpec],
    results: &[EvaluationCampaignCaseResult],
) -> Result<EvaluationCampaignAggregate, CampaignAggregationError> {
    let specs = cases
        .iter()
        .map(|case| (case.case_id.as_str(), case))
        .collect::<BTreeMap<_, _>>();
    let mut by_case = BTreeMap::new();
    for result in results {
        if !specs.contains_key(result.case_id.as_str()) {
            return Err(CampaignAggregationError::UnknownCase(
                result.case_id.clone(),
            ));
        }
        if by_case.insert(result.case_id.as_str(), result).is_some() {
            return Err(CampaignAggregationError::DuplicateCase(
                result.case_id.clone(),
            ));
        }
    }

    let ordered = specs
        .values()
        .map(|case| {
            by_case
                .get(case.case_id.as_str())
                .map(|result| (*result).clone())
                .unwrap_or_else(|| EvaluationCampaignCaseResult {
                    case_id: case.case_id.clone(),
                    run_id: String::new(),
                    status: EvaluationCaseStatus::Pending,
                    verdict: EvaluationVerdict::Inconclusive,
                    score_bps: None,
                    reason: Some("completed customer Run result has not been submitted".into()),
                })
        })
        .collect::<Vec<_>>();

    let critical_failed = ordered.iter().any(|result| {
        specs[result.case_id.as_str()].critical && result.verdict == EvaluationVerdict::Failed
    });
    let has_failed = ordered
        .iter()
        .any(|result| result.verdict == EvaluationVerdict::Failed);
    let has_error = ordered.iter().any(|result| {
        result.status == EvaluationCaseStatus::Error || result.verdict == EvaluationVerdict::Error
    });
    let has_incomplete = ordered.iter().any(|result| {
        matches!(
            result.status,
            EvaluationCaseStatus::Pending | EvaluationCaseStatus::Skipped
        ) || matches!(
            result.verdict,
            EvaluationVerdict::Inconclusive | EvaluationVerdict::NotConfigured
        )
    });
    let verdict = if critical_failed || has_failed {
        EvaluationVerdict::Failed
    } else if has_error {
        EvaluationVerdict::Error
    } else if has_incomplete {
        EvaluationVerdict::Inconclusive
    } else if ordered.is_empty() {
        EvaluationVerdict::NotConfigured
    } else {
        EvaluationVerdict::Passed
    };
    let (weighted, total_weight) = ordered.iter().fold((0_u64, 0_u64), |acc, result| {
        let weight = u64::from(specs[result.case_id.as_str()].weight);
        (
            acc.0 + u64::from(result.score_bps.unwrap_or(0)) * weight,
            acc.1 + weight,
        )
    });
    Ok(EvaluationCampaignAggregate {
        verdict,
        score_bps: (total_weight > 0).then_some((weighted / total_weight) as u32),
        completed_cases: ordered
            .iter()
            .filter(|result| result.status == EvaluationCaseStatus::Completed)
            .count() as u32,
        skipped_cases: ordered
            .iter()
            .filter(|result| result.status == EvaluationCaseStatus::Skipped)
            .count() as u32,
        error_cases: ordered
            .iter()
            .filter(|result| result.status == EvaluationCaseStatus::Error)
            .count() as u32,
        cases: ordered,
    })
}

/// Project immutable campaign evidence into a read-only deployment gate. A
/// manifest mismatch or missing evidence can never become a pass.
pub fn project_release_gate(
    agent_id: impl Into<String>,
    environment_id: impl Into<String>,
    required_manifest_hash: impl Into<String>,
    campaign_manifest_hash: &str,
    aggregate: &EvaluationCampaignAggregate,
    evidence_result_ids: Vec<String>,
    created_at: impl Into<String>,
) -> EvaluationReleaseGate {
    let manifest_hash = required_manifest_hash.into();
    let evidence_complete =
        manifest_hash == campaign_manifest_hash && !evidence_result_ids.is_empty();
    let verdict = if !evidence_complete {
        EvaluationReleaseGateVerdict::InsufficientEvidence
    } else {
        match aggregate.verdict {
            EvaluationVerdict::Passed => EvaluationReleaseGateVerdict::Passed,
            EvaluationVerdict::Failed => EvaluationReleaseGateVerdict::Failed,
            EvaluationVerdict::Inconclusive
            | EvaluationVerdict::Error
            | EvaluationVerdict::NotConfigured => {
                EvaluationReleaseGateVerdict::InsufficientEvidence
            }
        }
    };
    EvaluationReleaseGate {
        agent_id: agent_id.into(),
        environment_id: environment_id.into(),
        manifest_hash,
        verdict,
        evidence_result_ids,
        created_at: created_at.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn incomplete_evidence_never_passes() {
        let snapshot = SnapshotEvidence {
            snapshot_hash: "blake3:v1:test".into(),
            capture_status: RunCaptureStatus::Incomplete,
            metrics: BTreeMap::new(),
            triggered_policy_counts: BTreeMap::new(),
            evidence_ids: BTreeMap::new(),
        };
        let manifest = vec![ManifestEntry {
            policy_id: "no-denials".into(),
            policy_version: 1,
            policy_hash: "sha256:v1:test".into(),
            policy_yaml: r#"
family: evaluation
id: no-denials
severity: critical
scope: runtime_decisions
grader:
  kind: run_metric
  metric: denied_decisions
  comparator: lte
  value: 0
on_missing_evidence: fail
"#
            .into(),
            weight: 1,
            critical: true,
        }];
        let output = evaluate_deterministic(&snapshot, &manifest).expect("evaluation");
        assert_eq!(output.verdict, EvaluationVerdict::Failed);
        assert_ne!(output.verdict, EvaluationVerdict::Passed);
    }

    #[test]
    fn content_hash_is_independent_of_object_key_order() {
        let left = serde_json::json!({"b": 2, "a": {"d": 4, "c": 3}});
        let right = serde_json::json!({"a": {"c": 3, "d": 4}, "b": 2});
        assert_eq!(
            canonical_hash(&left).unwrap(),
            canonical_hash(&right).unwrap()
        );
    }

    fn campaign_case(id: &str, weight: u32, critical: bool) -> EvaluationCaseSpec {
        EvaluationCaseSpec {
            case_id: id.into(),
            case_hash: format!("blake3:v1:{id}"),
            input_hash: format!("blake3:v1:input-{id}"),
            reference_hash: None,
            scoring_mode: tl_core::EvaluationCaseScoringMode::Trajectory,
            weight,
            critical,
            budget: tl_core::EvaluationCaseBudget {
                max_turns: 4,
                max_tool_calls: 4,
                max_tokens: 2_000,
                max_duration_ms: 30_000,
            },
            oracle_metadata: serde_json::json!({}),
        }
    }

    #[test]
    fn campaign_missing_case_is_inconclusive_and_critical_failure_overrides_score() {
        let cases = vec![
            campaign_case("critical", 1, true),
            campaign_case("missing", 9, false),
        ];
        let aggregate = aggregate_campaign(
            &cases,
            &[EvaluationCampaignCaseResult {
                case_id: "critical".into(),
                run_id: "run-1".into(),
                status: EvaluationCaseStatus::Completed,
                verdict: EvaluationVerdict::Failed,
                score_bps: Some(9_900),
                reason: None,
            }],
        )
        .unwrap();
        assert_eq!(aggregate.verdict, EvaluationVerdict::Failed);
        assert_eq!(aggregate.completed_cases, 1);
        assert_eq!(aggregate.score_bps, Some(990));
        assert_eq!(aggregate.cases[1].status, EvaluationCaseStatus::Pending);
    }

    #[test]
    fn release_gate_requires_matching_manifest_and_evidence() {
        let aggregate = aggregate_campaign(
            &[campaign_case("pass", 1, false)],
            &[EvaluationCampaignCaseResult {
                case_id: "pass".into(),
                run_id: "run-1".into(),
                status: EvaluationCaseStatus::Completed,
                verdict: EvaluationVerdict::Passed,
                score_bps: Some(10_000),
                reason: None,
            }],
        )
        .unwrap();
        let gate = project_release_gate(
            "agent-a",
            "production",
            "manifest-a",
            "manifest-b",
            &aggregate,
            vec!["result-1".into()],
            "2026-01-01T00:00:00Z",
        );
        assert_eq!(
            gate.verdict,
            EvaluationReleaseGateVerdict::InsufficientEvidence
        );
    }

    #[test]
    fn rubric_policies_are_sent_in_one_batch() {
        struct Replay;
        impl PolicyReplayPort for Replay {
            fn replay(
                &self,
                _snapshot: &SnapshotEvidence,
                _entry: &ManifestEntry,
                _policy: &EvaluationPolicy,
            ) -> Result<FindingOutput, String> {
                unreachable!("test contains no replay policies")
            }
        }
        struct Rubric(std::sync::atomic::AtomicUsize);
        impl RubricGraderPort for Rubric {
            fn grade_batch(
                &self,
                _snapshot: &SnapshotEvidence,
                policies: &[(&ManifestEntry, &EvaluationPolicy)],
            ) -> Result<BTreeMap<String, FindingOutput>, String> {
                self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Ok(policies
                    .iter()
                    .map(|(entry, policy)| {
                        (
                            entry.policy_id.clone(),
                            finding(
                                entry,
                                policy.severity,
                                EvaluationFindingStatus::Passed,
                                Some(10_000),
                                "rubric passed".into(),
                                Vec::new(),
                            ),
                        )
                    })
                    .collect())
            }
        }
        let snapshot = SnapshotEvidence {
            snapshot_hash: "blake3:v1:snapshot".into(),
            capture_status: RunCaptureStatus::Complete,
            metrics: BTreeMap::new(),
            triggered_policy_counts: BTreeMap::new(),
            evidence_ids: BTreeMap::new(),
        };
        let policy_yaml = |id: &str| {
            format!(
                "family: evaluation\nid: {id}\nseverity: high\nscope: final_output\ngrader:\n  kind: llm_rubric\n  rubric: Be correct.\n  min_score: 0.8\non_missing_evidence: inconclusive\n"
            )
        };
        let manifest = ["rubric-a", "rubric-b"]
            .into_iter()
            .map(|id| ManifestEntry {
                policy_id: id.into(),
                policy_version: 1,
                policy_hash: format!("blake3:v1:{id}"),
                policy_yaml: policy_yaml(id),
                weight: 1,
                critical: false,
            })
            .collect::<Vec<_>>();
        let rubric = Rubric(std::sync::atomic::AtomicUsize::new(0));
        let output = evaluate_with_adapters(&snapshot, &manifest, &Replay, &rubric).unwrap();
        assert_eq!(rubric.0.load(std::sync::atomic::Ordering::SeqCst), 1);
        assert_eq!(output.verdict, EvaluationVerdict::Passed);
    }
}
