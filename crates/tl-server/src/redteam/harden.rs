//! `POST /v1/redteam/jobs/{id}/harden` — synthesize + verify guardrails from a
//! job's landed attacks.
//!
//! For each landed (non-control) attack we classify the harm mechanism, group by
//! class so one policy covers a class, synthesize a generalized candidate
//! (`tl_policy::synthesis`), and *verify* it through the real evaluator before
//! recommending. Survivors are returned `enabled = false` (and persisted when
//! `persist`), mirroring `guardrails:generate` — an operator opts in via
//! `PATCH /v1/policies/{id}/enabled`.

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
#[allow(unused_imports)]
use tl_core::ApiError;
use tl_core::{
    ApiErrorCode, HardenCandidate, HardenRequest, HardenResponse, PolicyDocument, RedteamJobResult,
};
use tl_engine::SemanticPolicyJudge;
use tl_policy::policy_ast::WhenClause;
use tl_policy::synthesis::{classify, synthesize, HarmKind, LandedSignal};

use super::response::job_error_response;
use super::verify::verify_candidate;
use super::RedteamState;
use crate::policies::{api_error_response, policy_store_error_response, workspace_id_from_headers};

fn is_control(result: &RedteamJobResult) -> bool {
    matches!(result.kind.as_deref(), Some("benign") | Some("control")) || result.outcome == "clean"
}

fn signal(result: &RedteamJobResult) -> LandedSignal<'_> {
    LandedSignal {
        attack: &result.attack,
        goal: &result.goal,
        reply: &result.reply,
        failure_modes: &[],
        harm_classes: &[],
    }
}

fn harm_slug(harm: HarmKind) -> &'static str {
    match harm {
        HarmKind::Credential => "credential",
        HarmKind::Pii => "pii",
        HarmKind::SystemPrompt => "system-prompt",
        HarmKind::ActionClaim => "action",
        HarmKind::ProtectedInfo => "protected",
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
    let results = match state.store.list_results(&workspace_id, &id).await {
        Ok(results) => results,
        Err(e) => return job_error_response(e),
    };

    let agent_id = job.agent_id.clone();
    let agent_scope: Vec<String> = agent_id.clone().into_iter().collect();
    let controls: Vec<String> = results
        .iter()
        .filter(|r| is_control(r))
        .map(|r| r.reply.clone())
        .collect();

    // Group landed (non-control) cases by harm class so one policy covers a
    // class and re-hardening upserts in place via a stable id.
    let mut classes: Vec<ClassGroup> = Vec::new();
    for result in results.iter().filter(|r| r.landed && !is_control(r)) {
        let harm = classify(&signal(result));
        match classes.iter_mut().find(|g| g.harm == harm) {
            Some(group) => {
                group.replies.push(result.reply.clone());
                group.seqs.push(result.seq);
            }
            None => classes.push(ClassGroup {
                harm,
                rep_attack: result.attack.clone(),
                rep_goal: result.goal.clone(),
                replies: vec![result.reply.clone()],
                seqs: vec![result.seq],
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
        let candidate = match synthesize(
            &rep,
            format!("harden-{}", harm_slug(group.harm)),
            when,
            agent_id.clone(),
        ) {
            Ok(candidate) => candidate,
            Err(issues) => {
                tracing::warn!(?issues, harm = ?group.harm, "skipping invalid synthesized candidate");
                continue;
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
                .set_enabled(&workspace_id, &environment_id, &candidate.policy.id, false)
                .await
            {
                Ok(document) => document,
                Err(e) => return policy_store_error_response(e),
            }
        } else {
            PolicyDocument {
                id: candidate.policy.id.clone(),
                description: candidate.policy.description.clone(),
                severity: candidate.policy.severity,
                enabled: false,
                source_yaml,
            }
        };

        candidates.push(HardenCandidate {
            policy: policy_doc,
            substrate: candidate.substrate.to_string(),
            evidence_seqs: group.seqs,
            source: "deterministic".to_string(),
            verify,
        });
    }

    Json(HardenResponse {
        candidates,
        unreachable: vec![],
        generated_at: chrono::Utc::now().to_rfc3339(),
    })
    .into_response()
}
