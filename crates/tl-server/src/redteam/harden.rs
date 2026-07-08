//! `POST /v1/redteam/jobs/{id}/harden` — synthesize + verify guardrails from a
//! job's landed attacks.
//!
//! For each landed (non-control) attack we classify the harm mechanism, group by
//! class so one policy covers a class, draft/synthesize a generalized candidate,
//! and *verify* it through the real evaluator before recommending. Survivors are
//! returned `enabled = false` (and persisted when `persist`), mirroring
//! `guardrails:generate` — an operator opts in via `PATCH /v1/policies/{id}/enabled`.

use std::collections::{BTreeMap, BTreeSet};

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
#[allow(unused_imports)]
use tl_core::ApiError;
use tl_core::{
    AllowedSource, ApiErrorCode, ApprovalRule, Confidentiality, EventKind, EventVerifyResult,
    GuardEvent, HardenCandidate, HardenCandidateOperation, HardenEventCandidate,
    HardenLabelPolicyCandidate, HardenRejection, HardenRejectionReason, HardenRequest,
    HardenResponse, Integrity, JobStatus, LabelBasis, LabelBasisSet, LabelPolicyStatus, Labels,
    Origin, ParamRole, ParamSpec, PolicyDocument, RedteamAttackSession, RedteamJobSummary,
    RegressionCaseSource, RegressionCaseSummary, RegressionExpectedOutcome, SideEffectClass,
    SourceLabelEvidence, SourceLabelPolicy, ToolMetadata, ToolResolution, Trust, Verdict,
    VerifyResult, WorkflowRequirement,
};
use tl_engine::{
    resolve_source_labels, ApprovalChecker, Checker, InformationFlowChecker, MemoryChecker,
    ParameterAuthChecker, ProvenancePropagator, ProvenanceResolver, SemanticPolicyJudge,
};
use tl_policy::policy_ast::WhenClause;
use tl_policy::synthesis::{
    classify_with_context, harden_policy_id, synthesize, HarmKind, LandedSignal, SynthesisContext,
};
use tl_policy::{MatchClause, Matcher, Policy, ValidationIssue};

use super::response::job_error_response;
use super::verify::verify_candidate;
use super::{HardenDraftError, HardenDraftInput, HardenDrafter};
use super::{NewRegressionCase, RedteamRegressionStoreError, RedteamState};
use crate::agents::AgentStoreError;
use crate::label_policy::LabelPolicyStoreError;
use crate::policies::{
    api_error_response, policy_store_error_response, workspace_id_from_headers, PolicyStoreError,
};
use crate::tool_metadata::ToolMetadataStoreError;

const OUTPUT_SUBSTRATE: &str = "semantic_output";
const APPROVAL_SUBSTRATE: &str = "approval";
const PARAM_SOURCE_SUBSTRATE: &str = "param_source";
const LABEL_POLICY_SUBSTRATE: &str = "label_policy";

fn is_control(session: &RedteamAttackSession) -> bool {
    matches!(session.kind.as_deref(), Some("benign") | Some("control"))
        || session.outcome == "clean"
}

fn signal<'a>(session: &'a RedteamAttackSession, reply: &'a str) -> LandedSignal<'a> {
    LandedSignal {
        attack: &session.attack,
        goal: &session.goal,
        reply,
        failure_modes: &[],
        harm_classes: &[],
    }
}

/// One harm class's landed evidence. `rep_*` is the first case, used to
/// re-derive the same class inside `synthesize`.
struct ClassGroup {
    harm: HarmKind,
    rep_attack: String,
    rep_goal: String,
    replies: Vec<String>,
    seqs: Vec<i32>,
}

#[derive(Clone)]
struct EventEvidence {
    seq: i32,
    event: GuardEvent,
}

#[derive(Clone)]
struct LabelPolicyGroup {
    origin: Origin,
    trust: Option<Trust>,
    confidentiality: Option<Confidentiality>,
    integrity: Option<Integrity>,
    evidence: Vec<EventEvidence>,
}

#[utoipa::path(
    post,
    path = "/v1/redteam/jobs/{id}/harden",
    tag = "redteam",
    params(("id" = String, Path, description = "Job id")),
    request_body = HardenRequest,
    responses(
        (status = 200, description = "Synthesized + verified guardrail candidates", body = HardenResponse),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
        (status = 404, description = "Job not found", body = ApiError),
        (status = 422, description = "Job is not complete", body = ApiError),
    ),
)]
pub async fn harden_job(
    State(state): State<RedteamState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    body: Option<Json<HardenRequest>>,
) -> Response {
    let request = body.map(|Json(b)| b).unwrap_or_default();
    let workspace_id = workspace_id_from_headers(&headers);
    let environment_id = match crate::environments::resolve_environment_id(
        &headers,
        state.environment_store.as_ref(),
        &workspace_id,
    )
    .await
    {
        Ok(environment_id) => environment_id,
        Err(error) => return crate::environments::environment_error_response(error),
    };

    let job = match state.store.get(&workspace_id, &id).await {
        Ok(job) => job,
        Err(e) => return job_error_response(e),
    };
    // Hardening reasons over a finished run. A queued/running/errored/cancelled
    // job has partial or no results, so synthesizing from it would recommend
    // guards from an incomplete attack set.
    if job.status != JobStatus::Complete {
        return api_error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            ApiErrorCode::Unprocessable,
            format!(
                "job is not complete (status: {:?}); harden requires a completed job",
                job.status
            ),
        );
    }
    let sessions = match state.store.list_sessions(&workspace_id, &id).await {
        Ok(sessions) => sessions,
        Err(e) => return job_error_response(e),
    };

    let agent_id = job.agent_id.clone();
    let agent_scope: Vec<String> = agent_id.clone().into_iter().collect();
    let workflow_requirements =
        load_workflow_requirements(&state, &workspace_id, agent_id.as_deref()).await;
    let synthesis_context = SynthesisContext {
        workflow_requirements: &workflow_requirements,
    };
    let controls: Vec<String> = sessions
        .iter()
        .filter(|r| is_control(r))
        .filter_map(session_reply)
        .collect();
    let control_action_events: Vec<GuardEvent> = sessions
        .iter()
        .filter(|r| is_control(r))
        .flat_map(action_event_evidence)
        .map(|evidence| evidence.event)
        .collect();

    // Group landed (non-control) cases by harm class so one policy covers a
    // class and re-hardening upserts in place via a stable id.
    let mut classes: Vec<ClassGroup> = Vec::new();
    let mut approval_groups: BTreeMap<String, Vec<EventEvidence>> = BTreeMap::new();
    let mut param_source_groups: BTreeMap<(String, String), Vec<EventEvidence>> = BTreeMap::new();
    let mut label_policy_groups: Vec<LabelPolicyGroup> = Vec::new();
    let mut rejections: Vec<HardenRejection> = Vec::new();
    let mut unreachable: BTreeSet<String> = BTreeSet::new();
    for session in sessions.iter().filter(|r| r.landed && !is_control(r)) {
        let Some(reply) = session_reply(session) else {
            rejections.push(rejection(
                HardenRejectionReason::NoTargetReply,
                OUTPUT_SUBSTRATE,
                vec![session.seq],
                None,
                "landed attack had no target reply to synthesize from",
            ));
            continue;
        };
        let harm = classify_with_context(&signal(session, &reply), &synthesis_context);
        if let Some(substrate) = unreachable_event_substrate(session, harm) {
            unreachable.insert(substrate.to_string());
        }
        let action_evidence = action_event_evidence(session);
        if needs_approval_substrate(harm) {
            for evidence in &action_evidence {
                approval_groups
                    .entry(evidence.event.action.operation.clone())
                    .or_default()
                    .push(evidence.clone());
            }
        }
        for evidence in &action_evidence {
            collect_label_policy_groups(&mut label_policy_groups, evidence);
            for path in candidate_param_paths(&evidence.event) {
                param_source_groups
                    .entry((evidence.event.action.operation.clone(), path))
                    .or_default()
                    .push(evidence.clone());
            }
        }
        match classes.iter_mut().find(|g| g.harm == harm) {
            Some(group) => {
                group.replies.push(reply);
                group.seqs.push(session.seq);
            }
            None => classes.push(ClassGroup {
                harm,
                rep_attack: session.attack.clone(),
                rep_goal: session.goal.clone(),
                replies: vec![reply],
                seqs: vec![session.seq],
            }),
        }
    }

    let judge: Option<&dyn SemanticPolicyJudge> = Some(state.llm.as_ref());
    let mut candidates: Vec<HardenCandidate> = Vec::new();
    for group in classes {
        let when = WhenClause {
            channels: vec![],
            domains: vec![],
            agents: agent_scope.clone(),
        };
        let rep = LandedSignal {
            attack: &group.rep_attack,
            goal: &group.rep_goal,
            reply: &group.replies[0],
            failure_modes: &[],
            harm_classes: &[],
        };
        let policy_id = harden_policy_id(agent_id.as_deref(), group.harm);
        let (candidate, source) = if HardenDrafter::is_enabled(state.llm.as_ref()) {
            match state
                .llm
                .as_ref()
                .draft(HardenDraftInput {
                    tenant: &workspace_id,
                    policy_id: &policy_id,
                    harm: group.harm,
                    agent_id: agent_id.as_deref(),
                    rep_attack: &group.rep_attack,
                    rep_goal: &group.rep_goal,
                    replies: &group.replies,
                    evidence_seqs: &group.seqs,
                    controls_count: controls.len(),
                    workflow_requirements: &workflow_requirements,
                    when: when.clone(),
                    owner_agent_id: agent_id.clone(),
                })
                .await
            {
                Ok(draft) => (draft.candidate, "llm"),
                Err(HardenDraftError::Disabled) => match synthesize_deterministic_candidate(
                    &rep,
                    &synthesis_context,
                    &policy_id,
                    when,
                    agent_id.clone(),
                ) {
                    Ok(candidate) => (candidate, "deterministic"),
                    Err(issues) => {
                        tracing::warn!(?issues, harm = ?group.harm, "skipping invalid synthesized candidate");
                        rejections.push(rejection(
                            HardenRejectionReason::SynthesisInvalid,
                            OUTPUT_SUBSTRATE,
                            group.seqs,
                            None,
                            "synthesized policy did not pass validation",
                        ));
                        continue;
                    }
                },
                Err(error) => {
                    tracing::warn!(
                        error = ?error,
                        harm = ?group.harm,
                        "skipping invalid llm-drafted harden candidate"
                    );
                    rejections.push(rejection(
                        HardenRejectionReason::SynthesisInvalid,
                        OUTPUT_SUBSTRATE,
                        group.seqs,
                        None,
                        draft_error_message(error),
                    ));
                    continue;
                }
            }
        } else {
            match synthesize_deterministic_candidate(
                &rep,
                &synthesis_context,
                &policy_id,
                when,
                agent_id.clone(),
            ) {
                Ok(candidate) => (candidate, "deterministic"),
                Err(issues) => {
                    tracing::warn!(?issues, harm = ?group.harm, "skipping invalid synthesized candidate");
                    rejections.push(rejection(
                        HardenRejectionReason::SynthesisInvalid,
                        OUTPUT_SUBSTRATE,
                        group.seqs,
                        None,
                        "synthesized policy did not pass validation",
                    ));
                    continue;
                }
            }
        };

        let verify = verify_candidate(
            &candidate.policy,
            &group.replies,
            &controls,
            judge,
            &workspace_id,
            agent_id.as_deref().unwrap_or(""),
        )
        .await;
        // A candidate that does not block what landed (or false-blocks a
        // control) protects nothing — drop it rather than recommend it.
        if !verify.passed {
            let reason = rejection_reason(&candidate.policy, &verify, judge);
            let message = rejection_message(reason);
            tracing::info!(
                job_id = %id,
                policy_id = %candidate.policy.id,
                harm = ?group.harm,
                substrate = %candidate.substrate,
                reason = ?reason,
                evidence_seqs = ?group.seqs,
                blocked_landed = verify.blocked_landed,
                landed_total = verify.landed_total,
                blocked_variants = verify.blocked_variants,
                variant_total = verify.variant_total,
                false_blocks = verify.false_blocks,
                control_total = verify.control_total,
                "rejecting harden candidate"
            );
            rejections.push(rejection(
                reason,
                candidate.substrate,
                group.seqs,
                Some(verify),
                message,
            ));
            continue;
        }

        let source_yaml = match serde_yaml::to_string(&candidate.policy) {
            Ok(yaml) => yaml,
            Err(e) => {
                return api_error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    ApiErrorCode::Internal,
                    format!("policy yaml render: {e}"),
                )
            }
        };

        let existing = match state
            .policy_store
            .get(&workspace_id, &environment_id, &candidate.policy.id)
            .await
        {
            Ok(document) => Some(document),
            Err(PolicyStoreError::NotFound) => None,
            Err(e) => return policy_store_error_response(e),
        };
        let operation = if existing.is_some() {
            HardenCandidateOperation::Tighten
        } else {
            HardenCandidateOperation::Create
        };
        let enabled_after_persist = existing.as_ref().is_some_and(|document| document.enabled);

        let policy_doc = if request.persist {
            if let Err(e) = state
                .policy_store
                .upsert(
                    &workspace_id,
                    &environment_id,
                    &candidate.policy,
                    &source_yaml,
                )
                .await
            {
                return policy_store_error_response(e);
            }
            match state
                .policy_store
                .set_enabled(
                    &workspace_id,
                    &environment_id,
                    &candidate.policy.id,
                    enabled_after_persist,
                )
                .await
            {
                Ok(document) => document,
                Err(e) => return policy_store_error_response(e),
            }
        } else {
            PolicyDocument {
                id: candidate.policy.id.clone(),
                family: tl_core::PolicyFamily::Content,
                description: candidate.policy.description.clone(),
                severity: candidate.policy.severity,
                enabled: enabled_after_persist,
                source_yaml,
            }
        };

        candidates.push(HardenCandidate {
            policy: policy_doc,
            operation,
            existing_policy_id: existing.map(|document| document.id),
            substrate: candidate.substrate.to_string(),
            evidence_seqs: group.seqs,
            source: source.to_string(),
            verify,
        });
    }

    let mut event_candidates =
        synthesize_approval_event_candidates(approval_groups, &control_action_events);
    event_candidates.extend(synthesize_param_source_event_candidates(
        param_source_groups,
        &control_action_events,
    ));
    let mut label_policy_candidates =
        synthesize_label_policy_candidates(label_policy_groups, &control_action_events);
    for candidate in &mut event_candidates {
        let existing = match state
            .tool_metadata_store
            .get(&workspace_id, &candidate.tool_metadata.tool)
            .await
        {
            Ok(entry) => Some(entry),
            Err(ToolMetadataStoreError::NotFound) => None,
            Err(e) => return tool_metadata_store_error_response("get", &e),
        };
        let enabled_after_persist = existing.as_ref().is_some_and(|entry| entry.enabled);
        if let Some(entry) = existing {
            candidate.operation = HardenCandidateOperation::Tighten;
            candidate.existing_tool_id = Some(entry.metadata.tool.clone());
            candidate.tool_metadata = merge_event_metadata(
                entry.metadata,
                &candidate.tool_metadata,
                &candidate.substrate,
            );
        }
        if request.persist {
            if let Err(e) = state
                .tool_metadata_store
                .upsert(
                    &workspace_id,
                    &candidate.tool_metadata,
                    enabled_after_persist,
                )
                .await
            {
                return tool_metadata_store_error_response("upsert", &e);
            }
        }
    }
    for candidate in &mut label_policy_candidates {
        let existing = match state
            .label_policy_store
            .get(&workspace_id, candidate.label_policy.origin)
            .await
        {
            Ok(entry) => Some(entry),
            Err(LabelPolicyStoreError::NotFound) => None,
            Err(e) => return label_policy_store_error_response("get", &e),
        };
        let enabled_after_persist = existing.as_ref().is_some_and(|entry| entry.enabled);
        if let Some(entry) = existing {
            candidate.operation = HardenCandidateOperation::Tighten;
            candidate.existing_origin = Some(entry.policy.origin);
            candidate.label_policy = merge_label_policy(entry.policy, &candidate.label_policy);
        }
        if request.persist {
            if let Err(e) = state
                .label_policy_store
                .upsert(
                    &workspace_id,
                    &candidate.label_policy,
                    enabled_after_persist,
                )
                .await
            {
                return label_policy_store_error_response("upsert", &e);
            }
        }
    }

    let regression_cases = if request.promote_regression {
        match promote_regression_cases(
            &state,
            &workspace_id,
            &job,
            &sessions,
            &candidates,
            &event_candidates,
            &label_policy_candidates,
        )
        .await
        {
            Ok(cases) => cases,
            Err(e) => return regression_store_error_response(e),
        }
    } else {
        Vec::new()
    };

    Json(HardenResponse {
        candidates,
        event_candidates,
        label_policy_candidates,
        rejections,
        unreachable: unreachable.into_iter().collect(),
        regression_cases,
        generated_at: chrono::Utc::now().to_rfc3339(),
    })
    .into_response()
}

fn unreachable_event_substrate(
    session: &RedteamAttackSession,
    harm: HarmKind,
) -> Option<&'static str> {
    let needed = needs_approval_substrate(harm).then_some(APPROVAL_SUBSTRATE)?;
    (!has_structured_action_event(session)).then_some(needed)
}

fn needs_approval_substrate(harm: HarmKind) -> bool {
    matches!(harm, HarmKind::WorkflowIntegrity | HarmKind::ActionClaim)
}

fn has_structured_action_event(session: &RedteamAttackSession) -> bool {
    session
        .events
        .iter()
        .filter_map(|event| event.guard_event.as_ref())
        .any(is_replayable_action_event)
}

fn action_event_evidence(session: &RedteamAttackSession) -> Vec<EventEvidence> {
    session
        .events
        .iter()
        .filter_map(|event| {
            let guard_event = event.guard_event.as_ref()?;
            is_replayable_action_event(guard_event).then(|| EventEvidence {
                seq: session.seq,
                event: guard_event.clone(),
            })
        })
        .collect()
}

fn is_replayable_action_event(event: &GuardEvent) -> bool {
    !matches!(event.kind, EventKind::OutputProposed) && !event.action.operation.trim().is_empty()
}

async fn promote_regression_cases(
    state: &RedteamState,
    workspace_id: &str,
    job: &RedteamJobSummary,
    sessions: &[RedteamAttackSession],
    candidates: &[HardenCandidate],
    event_candidates: &[HardenEventCandidate],
    label_policy_candidates: &[HardenLabelPolicyCandidate],
) -> Result<Vec<RegressionCaseSummary>, RedteamRegressionStoreError> {
    let mut promoted = Vec::new();
    for candidate in candidates {
        let (attack, goal) = representative_attack_goal(sessions, &candidate.evidence_seqs);
        let artifact_id = candidate.policy.id.clone();
        let case = NewRegressionCase {
            case_key: regression_case_key(
                &job.id,
                &candidate.substrate,
                &artifact_id,
                &candidate.evidence_seqs,
            ),
            environment_id: job.environment_id.clone(),
            agent_id: job.agent_id.clone(),
            source: RegressionCaseSource::Harden,
            source_job_id: Some(job.id.clone()),
            source_session_seqs: candidate.evidence_seqs.clone(),
            substrate: candidate.substrate.clone(),
            artifact_id,
            expected_outcome: RegressionExpectedOutcome::Block,
            attack,
            goal,
        };
        promoted.push(state.regression_store.upsert(workspace_id, case).await?);
    }
    for candidate in event_candidates {
        let (attack, goal) = representative_attack_goal(sessions, &candidate.evidence_seqs);
        let artifact_id = event_artifact_id(candidate);
        let case = NewRegressionCase {
            case_key: regression_case_key(
                &job.id,
                &candidate.substrate,
                &artifact_id,
                &candidate.evidence_seqs,
            ),
            environment_id: job.environment_id.clone(),
            agent_id: job.agent_id.clone(),
            source: RegressionCaseSource::Harden,
            source_job_id: Some(job.id.clone()),
            source_session_seqs: candidate.evidence_seqs.clone(),
            substrate: candidate.substrate.clone(),
            artifact_id,
            expected_outcome: event_expected_outcome(&candidate.substrate),
            attack,
            goal,
        };
        promoted.push(state.regression_store.upsert(workspace_id, case).await?);
    }
    for candidate in label_policy_candidates {
        let (attack, goal) = representative_attack_goal(sessions, &candidate.evidence_seqs);
        let artifact_id = label_policy_artifact_id(candidate.label_policy.origin);
        let case = NewRegressionCase {
            case_key: regression_case_key(
                &job.id,
                &candidate.substrate,
                &artifact_id,
                &candidate.evidence_seqs,
            ),
            environment_id: job.environment_id.clone(),
            agent_id: job.agent_id.clone(),
            source: RegressionCaseSource::Harden,
            source_job_id: Some(job.id.clone()),
            source_session_seqs: candidate.evidence_seqs.clone(),
            substrate: candidate.substrate.clone(),
            artifact_id,
            expected_outcome: RegressionExpectedOutcome::Stop,
            attack,
            goal,
        };
        promoted.push(state.regression_store.upsert(workspace_id, case).await?);
    }
    Ok(promoted)
}

fn representative_attack_goal(sessions: &[RedteamAttackSession], seqs: &[i32]) -> (String, String) {
    let first = seqs
        .iter()
        .find_map(|seq| sessions.iter().find(|session| session.seq == *seq));
    match first {
        Some(session) => (session.attack.clone(), session.goal.clone()),
        None => ("unknown".to_string(), "unknown".to_string()),
    }
}

fn event_artifact_id(candidate: &HardenEventCandidate) -> String {
    if candidate.substrate == PARAM_SOURCE_SUBSTRATE {
        let paths: Vec<&str> = candidate
            .tool_metadata
            .params
            .iter()
            .map(|param| param.path.as_str())
            .collect();
        if !paths.is_empty() {
            return format!("{}:{}", candidate.tool_metadata.tool, paths.join(","));
        }
    }
    candidate.tool_metadata.tool.clone()
}

fn label_policy_artifact_id(origin: Origin) -> String {
    format!("source-label-{}", origin_key(origin))
}

fn event_expected_outcome(substrate: &str) -> RegressionExpectedOutcome {
    match substrate {
        APPROVAL_SUBSTRATE => RegressionExpectedOutcome::Escalate,
        _ => RegressionExpectedOutcome::Stop,
    }
}

fn regression_case_key(
    job_id: &str,
    substrate: &str,
    artifact_id: &str,
    evidence_seqs: &[i32],
) -> String {
    let evidence = evidence_seqs
        .iter()
        .map(i32::to_string)
        .collect::<Vec<_>>()
        .join("-");
    format!("harden:{job_id}:{substrate}:{artifact_id}:{evidence}")
}

fn synthesize_approval_event_candidates(
    approval_groups: BTreeMap<String, Vec<EventEvidence>>,
    control_events: &[GuardEvent],
) -> Vec<HardenEventCandidate> {
    let mut candidates = Vec::new();
    for (tool, evidence) in approval_groups {
        let Some(first) = evidence.first() else {
            continue;
        };
        let metadata = approval_tool_metadata(tool, &first.event);
        let matching_controls: Vec<&GuardEvent> = control_events
            .iter()
            .filter(|event| event.action.operation == metadata.tool)
            .collect();
        let verify = verify_approval_candidate(&metadata, &evidence, &matching_controls);
        if !verify.passed {
            tracing::info!(
                tool = %metadata.tool,
                substrate = APPROVAL_SUBSTRATE,
                escalated_landed = verify.escalated_landed,
                landed_total = verify.landed_total,
                false_blocks = verify.false_blocks,
                control_total = verify.control_total,
                "rejecting event harden candidate"
            );
            continue;
        }
        let evidence_seqs = evidence
            .iter()
            .map(|evidence| evidence.seq)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        candidates.push(HardenEventCandidate {
            tool_metadata: metadata,
            operation: HardenCandidateOperation::Create,
            existing_tool_id: None,
            substrate: APPROVAL_SUBSTRATE.to_string(),
            evidence_seqs,
            source: "deterministic".to_string(),
            verify,
        });
    }
    candidates
}

fn synthesize_param_source_event_candidates(
    param_source_groups: BTreeMap<(String, String), Vec<EventEvidence>>,
    control_events: &[GuardEvent],
) -> Vec<HardenEventCandidate> {
    let mut candidates = Vec::new();
    for ((tool, path), evidence) in param_source_groups {
        let Some(first) = evidence.first() else {
            continue;
        };
        let matching_controls: Vec<&GuardEvent> = control_events
            .iter()
            .filter(|event| {
                event.action.operation == tool
                    && parameter_path_present(&event.action.parameters, &path)
            })
            .collect();
        let allowed_sources = allowed_sources_from_controls(&path, &matching_controls);
        if allowed_sources.is_empty() {
            continue;
        }
        let metadata = param_source_tool_metadata(tool, &path, allowed_sources, &first.event);
        let verify = verify_param_source_candidate(&metadata, &evidence, &matching_controls);
        if !verify.passed {
            tracing::info!(
                tool = %metadata.tool,
                path = %path,
                substrate = PARAM_SOURCE_SUBSTRATE,
                escalated_landed = verify.escalated_landed,
                landed_total = verify.landed_total,
                false_blocks = verify.false_blocks,
                control_total = verify.control_total,
                "rejecting parameter-source harden candidate"
            );
            continue;
        }
        let evidence_seqs = evidence
            .iter()
            .map(|evidence| evidence.seq)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        candidates.push(HardenEventCandidate {
            tool_metadata: metadata,
            operation: HardenCandidateOperation::Create,
            existing_tool_id: None,
            substrate: PARAM_SOURCE_SUBSTRATE.to_string(),
            evidence_seqs,
            source: "deterministic".to_string(),
            verify,
        });
    }
    candidates
}

fn collect_label_policy_groups(groups: &mut Vec<LabelPolicyGroup>, evidence: &EventEvidence) {
    let side_effect = evidence
        .event
        .action
        .side_effect
        .unwrap_or_else(|| inferred_side_effect(evidence.event.kind));
    if !is_high_impact_side_effect(side_effect) {
        return;
    }

    for source in contributing_sources(&evidence.event) {
        if is_external_sink_side_effect(side_effect)
            && source_label_family_is_policy_mutable(
                &evidence.event,
                &source.id,
                LabelPolicyFamily::Confidentiality,
            )
        {
            push_label_policy_proposal(
                groups,
                source.origin,
                None,
                Some(Confidentiality::Private),
                None,
                evidence,
            );
        }
        if !matches!(source.origin, Origin::User | Origin::System)
            && source_label_family_is_policy_mutable(
                &evidence.event,
                &source.id,
                LabelPolicyFamily::Trust,
            )
        {
            push_label_policy_proposal(
                groups,
                source.origin,
                Some(Trust::Untrusted),
                None,
                None,
                evidence,
            );
        }
    }
}

fn push_label_policy_proposal(
    groups: &mut Vec<LabelPolicyGroup>,
    origin: Origin,
    trust: Option<Trust>,
    confidentiality: Option<Confidentiality>,
    integrity: Option<Integrity>,
    evidence: &EventEvidence,
) {
    match groups.iter_mut().find(|group| group.origin == origin) {
        Some(group) => {
            group.trust = trust.or(group.trust);
            group.confidentiality = confidentiality.or(group.confidentiality);
            group.integrity = integrity.or(group.integrity);
            if !group.evidence.iter().any(|seen| seen.seq == evidence.seq) {
                group.evidence.push(evidence.clone());
            }
        }
        None => groups.push(LabelPolicyGroup {
            origin,
            trust,
            confidentiality,
            integrity,
            evidence: vec![evidence.clone()],
        }),
    }
}

#[derive(Clone, Copy)]
enum LabelPolicyFamily {
    Trust,
    Confidentiality,
}

fn synthesize_label_policy_candidates(
    groups: Vec<LabelPolicyGroup>,
    control_events: &[GuardEvent],
) -> Vec<HardenLabelPolicyCandidate> {
    let mut candidates = Vec::new();
    for group in groups {
        let policy = SourceLabelPolicy {
            origin: group.origin,
            trust: group.trust,
            confidentiality: group.confidentiality,
            integrity: group.integrity,
        };
        let matching_controls: Vec<&GuardEvent> = control_events
            .iter()
            .filter(|event| {
                contributing_sources(event)
                    .iter()
                    .any(|source| source.origin == policy.origin)
            })
            .collect();
        let verify = verify_label_policy_candidate(&policy, &group.evidence, &matching_controls);
        if !verify.passed {
            tracing::info!(
                origin = %origin_key(policy.origin),
                substrate = LABEL_POLICY_SUBSTRATE,
                escalated_landed = verify.escalated_landed,
                landed_total = verify.landed_total,
                false_blocks = verify.false_blocks,
                control_total = verify.control_total,
                "rejecting label-policy harden candidate"
            );
            continue;
        }
        let evidence_seqs = group
            .evidence
            .iter()
            .map(|evidence| evidence.seq)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        candidates.push(HardenLabelPolicyCandidate {
            label_policy: policy,
            operation: HardenCandidateOperation::Create,
            existing_origin: None,
            substrate: LABEL_POLICY_SUBSTRATE.to_string(),
            evidence_seqs,
            source: "deterministic".to_string(),
            verify,
        });
    }
    candidates
}

fn approval_tool_metadata(tool: String, event: &GuardEvent) -> ToolMetadata {
    ToolMetadata {
        tool: tool.clone(),
        side_effect: event
            .action
            .side_effect
            .unwrap_or_else(|| inferred_side_effect(event.kind)),
        reversible: false,
        params: vec![],
        approval: Some(ApprovalRule {
            required: true,
            approver_roles: vec!["admin".to_string()],
            reason: Some(format!(
                "Red-team hardening requires approval before `{tool}` after a landed action attack."
            )),
        }),
        sandbox_hint: None,
    }
}

fn param_source_tool_metadata(
    tool: String,
    path: &str,
    allowed_sources: Vec<AllowedSource>,
    event: &GuardEvent,
) -> ToolMetadata {
    ToolMetadata {
        tool,
        side_effect: event
            .action
            .side_effect
            .unwrap_or_else(|| inferred_side_effect(event.kind)),
        reversible: false,
        params: vec![ParamSpec {
            path: path.to_string(),
            role: ParamRole::AuthorityBearing,
            allowed_sources,
            limit: None,
        }],
        approval: None,
        sandbox_hint: None,
    }
}

fn merge_event_metadata(
    mut existing: ToolMetadata,
    proposed: &ToolMetadata,
    substrate: &str,
) -> ToolMetadata {
    match substrate {
        APPROVAL_SUBSTRATE => existing.approval = proposed.approval.clone(),
        PARAM_SOURCE_SUBSTRATE => {
            for proposed_spec in &proposed.params {
                match existing
                    .params
                    .iter_mut()
                    .find(|spec| spec.path == proposed_spec.path)
                {
                    Some(existing_spec) => {
                        existing_spec.role = proposed_spec.role;
                        existing_spec.allowed_sources = proposed_spec.allowed_sources.clone();
                    }
                    None => existing.params.push(proposed_spec.clone()),
                }
            }
        }
        _ => {}
    }
    existing
}

fn merge_label_policy(
    mut existing: SourceLabelPolicy,
    proposed: &SourceLabelPolicy,
) -> SourceLabelPolicy {
    existing.trust = proposed.trust.or(existing.trust);
    existing.confidentiality = proposed.confidentiality.or(existing.confidentiality);
    existing.integrity = proposed.integrity.or(existing.integrity);
    existing
}

fn inferred_side_effect(kind: EventKind) -> SideEffectClass {
    match kind {
        EventKind::OutputProposed => SideEffectClass::None,
        EventKind::ToolCallProposed | EventKind::ApiMutationProposed => {
            SideEffectClass::ApiMutation
        }
        EventKind::MemoryWriteProposed => SideEffectClass::MemoryWrite,
        EventKind::FileActionProposed => SideEffectClass::FileWrite,
        EventKind::ShellActionProposed => SideEffectClass::ShellExec,
        EventKind::NetworkRequestProposed => SideEffectClass::NetworkCall,
        EventKind::DatabaseMutationProposed => SideEffectClass::DbMutation,
        EventKind::BrowserActionProposed | EventKind::ExternalMessageProposed => {
            SideEffectClass::ExternalCommunication
        }
        EventKind::MemoryRetrievalUsedForAction => SideEffectClass::Read,
    }
}

fn verify_approval_candidate(
    metadata: &ToolMetadata,
    landed: &[EventEvidence],
    controls: &[&GuardEvent],
) -> EventVerifyResult {
    let escalated_landed = landed
        .iter()
        .filter(|evidence| approval_candidate_escalates(&evidence.event, metadata))
        .count() as u32;
    let false_blocks = controls
        .iter()
        .filter(|event| approval_candidate_escalates(event, metadata))
        .count() as u32;
    let landed_total = landed.len() as u32;
    let control_total = controls.len() as u32;
    EventVerifyResult {
        escalated_landed,
        landed_total,
        false_blocks,
        control_total,
        passed: landed_total > 0 && escalated_landed == landed_total && false_blocks == 0,
    }
}

fn verify_param_source_candidate(
    metadata: &ToolMetadata,
    landed: &[EventEvidence],
    controls: &[&GuardEvent],
) -> EventVerifyResult {
    let escalated_landed = landed
        .iter()
        .filter(|evidence| param_source_candidate_stops(&evidence.event, metadata))
        .count() as u32;
    let false_blocks = controls
        .iter()
        .filter(|event| param_source_candidate_stops(event, metadata))
        .count() as u32;
    let landed_total = landed.len() as u32;
    let control_total = controls.len() as u32;
    EventVerifyResult {
        escalated_landed,
        landed_total,
        false_blocks,
        control_total,
        passed: landed_total > 0
            && control_total > 0
            && escalated_landed == landed_total
            && false_blocks == 0,
    }
}

fn verify_label_policy_candidate(
    policy: &SourceLabelPolicy,
    landed: &[EventEvidence],
    controls: &[&GuardEvent],
) -> EventVerifyResult {
    let escalated_landed = landed
        .iter()
        .filter(|evidence| label_policy_candidate_newly_stops(&evidence.event, policy))
        .count() as u32;
    let false_blocks = controls
        .iter()
        .filter(|event| label_policy_candidate_newly_stops(event, policy))
        .count() as u32;
    let landed_total = landed.len() as u32;
    let control_total = controls.len() as u32;
    EventVerifyResult {
        escalated_landed,
        landed_total,
        false_blocks,
        control_total,
        passed: landed_total > 0
            && control_total > 0
            && escalated_landed == landed_total
            && false_blocks == 0,
    }
}

fn approval_candidate_escalates(event: &GuardEvent, metadata: &ToolMetadata) -> bool {
    if event.action.operation != metadata.tool {
        return false;
    }
    let mut replay = event.clone();
    replay.resolution = Some(ToolResolution::Resolved {
        metadata: metadata.clone(),
    });
    ApprovalChecker
        .check(&replay)
        .iter()
        .any(|finding| finding.verdict == Some(Verdict::Escalate))
}

fn param_source_candidate_stops(event: &GuardEvent, metadata: &ToolMetadata) -> bool {
    if event.action.operation != metadata.tool {
        return false;
    }
    let mut replay = event.clone();
    replay.resolution = Some(ToolResolution::Resolved {
        metadata: metadata.clone(),
    });
    ParameterAuthChecker
        .check(&replay)
        .iter()
        .any(|finding| matches!(finding.verdict, Some(Verdict::Block | Verdict::Escalate)))
}

fn label_policy_candidate_newly_stops(event: &GuardEvent, policy: &SourceLabelPolicy) -> bool {
    !label_policy_replay_stops(&replay_current_labels(event))
        && label_policy_replay_stops(&replay_label_policy(event, std::slice::from_ref(policy)))
}

fn label_policy_replay_stops(event: &GuardEvent) -> bool {
    InformationFlowChecker
        .check(event)
        .iter()
        .any(|finding| matches!(finding.verdict, Some(Verdict::Block | Verdict::Escalate)))
        || MemoryChecker
            .check(event)
            .iter()
            .any(|finding| matches!(finding.verdict, Some(Verdict::Block | Verdict::Escalate)))
}

fn replay_label_policy(event: &GuardEvent, policies: &[SourceLabelPolicy]) -> GuardEvent {
    let mut replay = event.clone();
    let mut sources = Vec::with_capacity(replay.sources.len());
    for source in &mut replay.sources {
        source.labels = declared_labels_for_replay(event, &source.id, source.labels);
        let (labels, basis) = resolve_source_labels(source, policies);
        source.labels = labels;
        sources.push(SourceLabelEvidence {
            source_id: source.id.clone(),
            labels,
            basis,
        });
    }
    replay.label_resolution = Some(tl_core::LabelResolution {
        policy_status: if policies.is_empty() {
            LabelPolicyStatus::NotConfigured
        } else {
            LabelPolicyStatus::Applied
        },
        sources,
        derived: BTreeMap::new(),
    });
    ProvenancePropagator.resolve(&mut replay);
    replay
}

fn replay_current_labels(event: &GuardEvent) -> GuardEvent {
    let mut replay = event.clone();
    if replay.label_resolution.is_none() {
        replay.label_resolution = Some(tl_core::LabelResolution {
            policy_status: LabelPolicyStatus::NotConfigured,
            sources: replay
                .sources
                .iter()
                .map(|source| SourceLabelEvidence {
                    source_id: source.id.clone(),
                    labels: source.labels,
                    basis: LabelBasisSet {
                        trust: LabelBasis::Declared,
                        confidentiality: LabelBasis::Declared,
                        integrity: LabelBasis::Declared,
                    },
                })
                .collect(),
            derived: BTreeMap::new(),
        });
    }
    ProvenancePropagator.resolve(&mut replay);
    replay
}

fn declared_labels_for_replay(event: &GuardEvent, source_id: &str, labels: Labels) -> Labels {
    let Some(basis) = label_basis_for_source(event, source_id) else {
        return labels;
    };
    Labels {
        trust: if basis.trust == LabelBasis::Declared {
            labels.trust
        } else {
            Trust::Unknown
        },
        confidentiality: if basis.confidentiality == LabelBasis::Declared {
            labels.confidentiality
        } else {
            Confidentiality::Unknown
        },
        integrity: if basis.integrity == LabelBasis::Declared {
            labels.integrity
        } else {
            Integrity::Unknown
        },
    }
}

fn label_basis_for_source(event: &GuardEvent, source_id: &str) -> Option<LabelBasisSet> {
    event
        .label_resolution
        .as_ref()?
        .sources
        .iter()
        .find(|evidence| evidence.source_id == source_id)
        .map(|evidence| evidence.basis)
}

fn source_label_family_is_policy_mutable(
    event: &GuardEvent,
    source_id: &str,
    family: LabelPolicyFamily,
) -> bool {
    let Some(basis) = label_basis_for_source(event, source_id) else {
        return event
            .sources
            .iter()
            .find(|source| source.id == source_id)
            .is_some_and(|source| match family {
                LabelPolicyFamily::Trust => source.labels.trust == Trust::Unknown,
                LabelPolicyFamily::Confidentiality => {
                    source.labels.confidentiality == Confidentiality::Unknown
                }
            });
    };
    match family {
        LabelPolicyFamily::Trust => basis.trust != LabelBasis::Declared,
        LabelPolicyFamily::Confidentiality => basis.confidentiality != LabelBasis::Declared,
    }
}

fn contributing_sources(event: &GuardEvent) -> Vec<&tl_core::Source> {
    let mut sources = Vec::new();
    for ids in event.provenance.0.values() {
        for id in ids {
            if let Some(source) = event.sources.iter().find(|source| &source.id == id) {
                if !sources
                    .iter()
                    .any(|seen: &&tl_core::Source| seen.id == source.id)
                {
                    sources.push(source);
                }
            }
        }
    }
    sources
}

fn is_external_sink_side_effect(side_effect: SideEffectClass) -> bool {
    matches!(
        side_effect,
        SideEffectClass::ExternalCommunication
            | SideEffectClass::Publish
            | SideEffectClass::NetworkCall
    )
}

fn is_high_impact_side_effect(side_effect: SideEffectClass) -> bool {
    matches!(
        side_effect,
        SideEffectClass::ExternalCommunication
            | SideEffectClass::Publish
            | SideEffectClass::NetworkCall
            | SideEffectClass::FileWrite
            | SideEffectClass::ShellExec
            | SideEffectClass::DbMutation
            | SideEffectClass::ApiMutation
            | SideEffectClass::MemoryWrite
    )
}

fn candidate_param_paths(event: &GuardEvent) -> Vec<String> {
    let mut paths = BTreeSet::new();
    collect_parameter_leaf_paths("", &event.action.parameters, &mut paths);
    for path in event.provenance.0.keys() {
        if parameter_path_present(&event.action.parameters, path) {
            paths.insert(path.clone());
        }
    }
    paths.into_iter().collect()
}

fn collect_parameter_leaf_paths(
    prefix: &str,
    value: &serde_json::Value,
    paths: &mut BTreeSet<String>,
) {
    match value {
        serde_json::Value::Object(map) => {
            for (key, value) in map {
                let next = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{prefix}.{key}")
                };
                collect_parameter_leaf_paths(&next, value, paths);
            }
        }
        serde_json::Value::Null => {}
        _ if !prefix.is_empty() => {
            paths.insert(prefix.to_string());
        }
        _ => {}
    }
}

fn parameter_path_present(parameters: &serde_json::Value, path: &str) -> bool {
    let mut current = parameters;
    for segment in path.split('.') {
        let Some(next) = current.as_object().and_then(|object| object.get(segment)) else {
            return false;
        };
        current = next;
    }
    true
}

fn allowed_sources_from_controls(path: &str, controls: &[&GuardEvent]) -> Vec<AllowedSource> {
    let mut allowed_sources = Vec::new();
    for source in controls
        .iter()
        .flat_map(|event| sources_for_param_path(event, path))
    {
        let allowed = AllowedSource {
            origin: source.origin,
            source_id: None,
            kind: source.kind.clone(),
        };
        if !allowed_sources.contains(&allowed) {
            allowed_sources.push(allowed);
        }
    }
    allowed_sources
}

fn sources_for_param_path<'a>(event: &'a GuardEvent, path: &str) -> Vec<&'a tl_core::Source> {
    let Some(source_ids) = event.provenance.0.get(path) else {
        return vec![];
    };
    source_ids
        .iter()
        .filter_map(|id| event.sources.iter().find(|source| &source.id == id))
        .collect()
}

fn session_reply(session: &RedteamAttackSession) -> Option<String> {
    session
        .events
        .iter()
        .find(|event| event.kind == "target_reply")
        .and_then(|event| event.content_text.clone())
}

fn synthesize_deterministic_candidate(
    signal: &LandedSignal<'_>,
    context: &SynthesisContext<'_>,
    policy_id: &str,
    when: WhenClause,
    owner_agent_id: Option<String>,
) -> Result<tl_policy::synthesis::Candidate, Vec<ValidationIssue>> {
    synthesize(signal, context, policy_id.to_string(), when, owner_agent_id)
}

async fn load_workflow_requirements(
    state: &RedteamState,
    workspace_id: &str,
    agent_id: Option<&str>,
) -> Vec<WorkflowRequirement> {
    let Some(agent_id) = agent_id else {
        return vec![];
    };
    let Some(agent_store) = state.agent_store.as_ref() else {
        return vec![];
    };
    match agent_store.get(workspace_id, agent_id).await {
        Ok(profile) => profile.workflow_requirements.clone(),
        Err(AgentStoreError::NotFound) => {
            tracing::debug!(%agent_id, "harden agent profile not found");
            vec![]
        }
        Err(error) => {
            tracing::warn!(%agent_id, error = %error, "harden could not load agent workflow requirements");
            vec![]
        }
    }
}

fn rejection(
    reason: HardenRejectionReason,
    substrate: impl Into<String>,
    evidence_seqs: Vec<i32>,
    verify: Option<VerifyResult>,
    message: impl Into<String>,
) -> HardenRejection {
    HardenRejection {
        reason,
        substrate: substrate.into(),
        evidence_seqs,
        verify,
        message: message.into(),
    }
}

fn rejection_reason(
    policy: &Policy,
    verify: &VerifyResult,
    judge: Option<&dyn SemanticPolicyJudge>,
) -> HardenRejectionReason {
    if judge.map_or(true, |judge| !judge.is_enabled())
        && policy_has_semantic_matcher(policy)
        && verify.blocked_landed < verify.landed_total
    {
        return HardenRejectionReason::SemanticJudgeUnavailable;
    }
    if verify.blocked_landed < verify.landed_total {
        return HardenRejectionReason::MissedLanded;
    }
    if verify.blocked_variants < verify.variant_total {
        return HardenRejectionReason::MissedVariant;
    }
    if verify.false_blocks > 0 {
        return HardenRejectionReason::FalseBlockedControl;
    }
    HardenRejectionReason::UnreachableSubstrate
}

fn rejection_message(reason: HardenRejectionReason) -> &'static str {
    match reason {
        HardenRejectionReason::NoTargetReply => {
            "landed attack had no target reply to synthesize from"
        }
        HardenRejectionReason::SynthesisInvalid => "synthesized policy did not pass validation",
        HardenRejectionReason::MissedLanded => "candidate did not block every landed reply",
        HardenRejectionReason::MissedVariant => {
            "candidate missed a reworded version of the landed reply"
        }
        HardenRejectionReason::FalseBlockedControl => "candidate blocked a benign control reply",
        HardenRejectionReason::SemanticJudgeUnavailable => {
            "semantic policy judge is not configured, so the candidate could not be verified"
        }
        HardenRejectionReason::UnreachableSubstrate => {
            "candidate required a substrate this job could not verify"
        }
    }
}

fn draft_error_message(error: HardenDraftError) -> String {
    match error {
        HardenDraftError::Disabled => "harden-draft LLM route is not configured".into(),
        HardenDraftError::Provider(message) => {
            format!("harden-draft LLM provider error: {message}")
        }
        HardenDraftError::Invalid(message) => {
            format!("harden-draft LLM returned an invalid policy candidate: {message}")
        }
    }
}

fn tool_metadata_store_error_response(
    operation: &'static str,
    err: &ToolMetadataStoreError,
) -> Response {
    tracing::error!(error = %err, operation, "tool metadata store error during harden");
    api_error_response(
        StatusCode::INTERNAL_SERVER_ERROR,
        ApiErrorCode::Internal,
        "internal error".to_string(),
    )
}

fn label_policy_store_error_response(
    operation: &'static str,
    err: &LabelPolicyStoreError,
) -> Response {
    tracing::error!(error = %err, operation, "label policy store error during harden");
    api_error_response(
        StatusCode::INTERNAL_SERVER_ERROR,
        ApiErrorCode::Internal,
        "internal error".to_string(),
    )
}

fn regression_store_error_response(err: RedteamRegressionStoreError) -> Response {
    tracing::error!(error = %err, "regression store error during harden promotion");
    api_error_response(
        StatusCode::INTERNAL_SERVER_ERROR,
        ApiErrorCode::Internal,
        "internal error".to_string(),
    )
}

fn policy_has_semantic_matcher(policy: &Policy) -> bool {
    match_has_semantic(&policy.r#match)
}

fn match_has_semantic(r#match: &MatchClause) -> bool {
    match r#match {
        MatchClause::Single(matcher) => matcher_is_semantic(matcher),
        MatchClause::Any { any } => any.iter().any(matcher_is_semantic),
        MatchClause::All { all } => all.iter().any(matcher_is_semantic),
    }
}

fn matcher_is_semantic(matcher: &Matcher) -> bool {
    matches!(matcher, Matcher::Semantic(_))
}

fn origin_key(origin: Origin) -> String {
    match serde_json::to_value(origin) {
        Ok(serde_json::Value::String(value)) => value,
        _ => "unknown".to_string(),
    }
}
